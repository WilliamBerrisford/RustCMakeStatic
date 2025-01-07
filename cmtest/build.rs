use std::{
    collections::{HashMap, HashSet},
    error::Error,
    ffi::OsStr,
    fmt::Debug,
    fs::File,
    hash::Hash,
    io::Read,
    path::Path,
    sync::LazyLock,
};

use object::{Object, ObjectSymbol};
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

    let libs = find_libs(Path::new(&lib.display().to_string()));

    let symbols = generate_lookup_tables(libs.clone());
    symbols
        .defined
        .iter()
        .for_each(|(symbol, static_lib)| println!("{:?} {:?}", symbol, static_lib));

    symbols
        .undefined
        .iter()
        .for_each(|(defined, static_lib)| println!("{:?} {:?}", defined, static_lib));

    for lib in &libs {
        println!("Found static lib: {}", lib.name);
        //lib.undefined_symbols
        //    .into_iter()
        //    .map(|bytes| String::from_utf8(bytes).unwrap_or(String::from("Not utf8!")))
        //    .for_each(|name| println!("UndefinedSymbol symbol: {}", name));

        //lib.defined_symbols
        //    .into_iter()
        //    .map(|bytes| String::from_utf8(bytes).unwrap_or(String::from("Not utf8!")))
        //    .for_each(|name| println!("DefinedSymbol symbol: {}", name));
    }
}

fn generate_lookup_tables<I>(libs: I) -> AllSymbols
where
    I: IntoIterator<Item = StaticLib>,
{
    let mut defined_table: HashMap<DefinedSymbol, StaticLib> = HashMap::new();
    let mut undefined_table: HashMap<UnDefinedSymbol, StaticLib> = HashMap::new();
    for lib in libs {
        for symbol in lib.defined_symbols.clone() {
            defined_table.insert(symbol, lib.clone());
        }

        for symbol in lib.undefined_symbols.clone() {
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

fn find_libs(base_path: &Path) -> HashSet<StaticLib> {
    WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| entry.metadata().ok().map(|meta| (meta, entry)))
        .filter(|(metadata, _)| metadata.is_file())
        .filter(|(_, file)| is_static_lib(file.file_name()))
        .filter(|(_, file)| file.file_name().to_str().is_some())
        .map(|(_, file)| {
            let (defined, undefined) = get_symbols(&file).unwrap_or_default();
            StaticLib {
                name: file.file_name().to_str().unwrap().to_owned(),
                entry: file,
                defined_symbols: defined,
                undefined_symbols: undefined,
            }
        })
        .collect::<HashSet<StaticLib>>()
}

struct AllSymbols {
    defined: HashMap<DefinedSymbol, StaticLib>,
    undefined: HashMap<UnDefinedSymbol, StaticLib>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct DefinedSymbol {
    symbol: Vec<u8>,
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
struct StaticLib {
    name: String,
    entry: DirEntry,
    defined_symbols: Vec<DefinedSymbol>,
    undefined_symbols: Vec<UnDefinedSymbol>,
}

impl Hash for StaticLib {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for StaticLib {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Eq for StaticLib {}

impl Debug for StaticLib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticLib")
            .field("name", &self.name)
            .finish()
    }
}

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
