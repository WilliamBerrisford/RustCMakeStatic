use std::{
    collections::{HashMap, HashSet},
    error::Error,
    ffi::OsStr,
    fmt::{Debug, Display},
    fs::File,
    hash::Hash,
    io::Read,
    path::Path,
    sync::LazyLock,
};

use object::{Object, ObjectSymbol};
use petgraph::{
    dot::{Config, Dot},
    graph::NodeIndex,
    Graph, IntoWeightedEdge,
};
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

fn main() {
    let lib = cmake::build("../tinkwrap");

    cxx_build::bridge("src/main.rs")
        .file("include/bridge.cpp")
        .std("c++14")
        .compile("cmtest");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=include/bridge.cpp");
    println!("cargo:rerun-if-changed=include/bridge.h");
    println!("cargo:rerun-if-changed=../tinkwrap");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=tinkwrap");

    let all_libs = find_libs(Path::new(&lib.display().to_string()));

    all_libs
        .all_symbols
        .defined
        .iter()
        .for_each(|(symbol, static_lib)| println!("{:?} {}", symbol, static_lib));

    all_libs
        .all_symbols
        .undefined
        .iter()
        .for_each(|(defined, static_lib)| println!("{:?} {}", defined, static_lib));

    for lib in &all_libs.libs {
        println!("Found static lib: {}", lib);
    }

    generate_dependancy_graph(all_libs);
}

fn generate_dependancy_graph(libs: AllLibs) {
    let mut dep_graph = Graph::<LibInfo, u8>::new();

    let mut index_to_lib_map: HashMap<NodeIndex, LibInfo> = HashMap::new();
    let mut lib_to_index_map: HashMap<LibInfo, NodeIndex> = HashMap::new();
    for lib in libs.libs {
        let index = dep_graph.add_node(lib.clone());
        lib_to_index_map.insert(lib.clone(), index);
        index_to_lib_map.insert(index, lib);
    }

    let mut dependencies: HashSet<Dependency> = HashSet::new();
    for (symbol, dependent) in libs.all_symbols.undefined {
        let Some(dependency) = get_lib_for_symbol(&symbol, &libs.all_symbols.defined) else {
            continue;
        };

        let Some(dependency_index) = lib_to_index_map.get(&dependency) else {
            continue;
        };

        let Some(dependent_index) = lib_to_index_map.get(&dependent) else {
            continue;
        };

        if dependency_index == dependent_index {
            continue;
        }

        dependencies.insert(Dependency {
            dependent: *dependent_index,
            dependency: *dependency_index,
        });
    }

    for dep in dependencies {
        dep_graph.add_edge(dep.dependent, dep.dependency, 0);
    }
    println!("{:?}", Dot::with_config(&dep_graph, &[Config::EdgeNoLabel]));
}

struct Dependency {
    dependent: NodeIndex,
    dependency: NodeIndex,
}

impl Hash for Dependency {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dependent.hash(state);
        self.dependency.hash(state);
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        self.dependent.eq(&other.dependent) && self.dependency.eq(&other.dependency)
    }
}

impl Eq for Dependency {}

fn get_lib_for_symbol(
    symbol: &UnDefinedSymbol,
    defined_libs: &HashMap<DefinedSymbol, LibInfo>,
) -> Option<LibInfo> {
    defined_libs.get(&DefinedSymbol::from(symbol)).cloned()
}

fn generate_lookup_tables<I>(libs: I) -> AllSymbols
where
    I: IntoIterator<Item = LibInfo>,
{
    let mut defined_table: HashMap<DefinedSymbol, LibInfo> = HashMap::new();
    let mut undefined_table: HashMap<UnDefinedSymbol, LibInfo> = HashMap::new();
    for lib in libs {
        let Some(lib_entry) = &lib.entry else {
            continue;
        };

        let (defined, undefined) = get_symbols(lib_entry).unwrap_or_default();
        for symbol in defined {
            defined_table.insert(symbol, lib.clone());
        }

        for symbol in undefined {
            undefined_table.insert(symbol, lib.clone());
        }
    }

    AllSymbols {
        defined: defined_table,
        undefined: undefined_table,
    }
}

static LIB_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"lib(.*)\.a").expect("static lib regex failed to compile"));

fn find_libs(base_path: &Path) -> AllLibs {
    let libs: HashSet<LibInfo> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| entry.metadata().ok().map(|meta| (meta, entry)))
        .filter(|(metadata, _)| metadata.is_file())
        .filter(|(_, file)| is_static_lib(file.file_name()))
        .filter(|(_, file)| file.file_name().to_str().is_some())
        .map(|(_, file)| {
            let name = file.file_name().to_str().unwrap().to_owned();
            LibInfo {
                name,
                entry: Some(file),
            }
        })
        .collect();

    let all_symbols = generate_lookup_tables(libs.clone());

    AllLibs { libs, all_symbols }
}

#[derive(Clone)]
struct AllSymbols {
    defined: HashMap<DefinedSymbol, LibInfo>,
    undefined: HashMap<UnDefinedSymbol, LibInfo>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct DefinedSymbol {
    symbol: Vec<u8>,
}

impl From<&UnDefinedSymbol> for DefinedSymbol {
    fn from(value: &UnDefinedSymbol) -> Self {
        DefinedSymbol {
            symbol: value.symbol.clone(),
        }
    }
}

impl Debug for DefinedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefinedSymbol")
            .field(
                "symbol",
                &String::from_utf8(self.symbol.clone()).unwrap_or(String::from("Not utf8")),
            )
            .finish()
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct UnDefinedSymbol {
    symbol: Vec<u8>,
}

impl Debug for UnDefinedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnDefinedSymbol")
            .field(
                "symbol",
                &String::from_utf8(self.symbol.clone()).unwrap_or(String::from("Not utf8")),
            )
            .finish()
    }
}

#[derive(Clone)]
struct AllLibs {
    libs: HashSet<LibInfo>,
    all_symbols: AllSymbols,
}

#[derive(Clone, Debug, Default)]
struct LibInfo {
    name: String,
    entry: Option<DirEntry>,
}

impl Hash for LibInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for LibInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Display for LibInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Eq for LibInfo {}

fn get_symbols(
    entry: &DirEntry,
) -> Result<(Vec<DefinedSymbol>, Vec<UnDefinedSymbol>), Box<dyn Error>> {
    let archive_file = File::open(entry.path())?;
    let mut archive = ar::Archive::new(archive_file);

    let mut all_defined: Vec<DefinedSymbol> = vec![];
    let mut all_undefined: Vec<UnDefinedSymbol> = vec![];

    while let Some(entry) = archive.next_entry() {
        let Ok(mut entry) = entry else {
            continue;
        };

        let mut buf: Vec<u8> = vec![];
        if entry.read_to_end(&mut buf).is_err() {
            continue;
        }

        let file = object::File::parse(&*buf)?;
        let mut defined = file
            .symbols()
            .filter(|symbol| symbol.is_definition())
            .filter_map(|symbol| symbol.name_bytes().ok())
            .map(|bytes| DefinedSymbol {
                symbol: bytes.to_vec(),
            })
            .collect();

        let mut undefined = file
            .symbols()
            .filter(|symbol| symbol.is_undefined())
            .filter_map(|symbol| symbol.name_bytes().ok())
            .map(|bytes| UnDefinedSymbol {
                symbol: bytes.to_vec(),
            })
            .collect();

        all_defined.append(&mut defined);
        all_undefined.append(&mut undefined);
    }

    Ok((all_defined, all_undefined))
}

fn is_static_lib(file_name: &OsStr) -> bool {
    let Some(file_name) = file_name.to_str() else {
        return false;
    };
    LIB_REGEX.is_match(file_name)
}
