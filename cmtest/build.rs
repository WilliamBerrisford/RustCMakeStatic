use std::{
    collections::HashSet,
    convert::identity,
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    hash::Hash,
    io::Read,
    path::{Path, PathBuf},
    str,
    sync::LazyLock,
};

use cmake;
use object::{
    read::archive::{ArchiveFile, ArchiveSymbol},
    Object, ObjectSymbol,
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

    for lib in find_libs(Path::new(&lib.display().to_string())) {
        println!("Found static lib: {}", lib.name);
        lib.undefined_symbols
            .into_iter()
            .map(|bytes| String::from_utf8(bytes).unwrap_or(String::from("Not utf8!")))
            .for_each(|name| println!("UndefinedSymbol symbol: {}", name));

        lib.defined_symbols
            .into_iter()
            .map(|bytes| String::from_utf8(bytes).unwrap_or(String::from("Not utf8!")))
            .for_each(|name| println!("DefinedSymbol symbol: {}", name));
    }

    println!(
        "cargo:rustc-link-search=native={}/build/3rdParty/a/",
        lib.display()
    );
    println!("cargo:rustc-link-lib=static=a");
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

struct StaticLib {
    name: String,
    entry: DirEntry,
    defined_symbols: Vec<DefinedSymbol>,
    undefined_symbols: Vec<UnDefinedSymbol>,
}

type DefinedSymbol = Vec<u8>;
type UnDefinedSymbol = Vec<u8>;

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

fn get_symbols(
    entry: &DirEntry,
) -> Result<(Vec<DefinedSymbol>, Vec<UnDefinedSymbol>), Box<dyn Error>> {
    let archive_file = File::open(entry.path())?;
    let mut archive = ar::Archive::new(archive_file);

    let mut all_defined: Vec<DefinedSymbol> = vec![];
    let mut all_undefined: Vec<UnDefinedSymbol> = vec![];

    while let Some(mut entry) = archive.next_entry() {
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
            //.filter(|symbol| symbol.is_definition())
            .filter_map(|symbol| symbol.name_bytes().ok())
            .map(|bytes| bytes.to_vec())
            .collect();

        let mut undefined = file
            .symbols()
            .filter(|symbol| symbol.is_undefined())
            .filter_map(|symbol| symbol.name_bytes().ok())
            .map(|bytes| bytes.to_vec())
            .collect();

        all_defined.append(&mut defined);
        all_undefined.append(&mut undefined);
    }

    Ok((all_defined, all_undefined))
}

fn get_symbols_from_object(
    entry: &DirEntry,
) -> Result<(Vec<DefinedSymbol>, Vec<UnDefinedSymbol>), Box<dyn Error>> {
    let data = fs::read(entry.path())?;
    println!("Parsing file {:?}", entry.path());
    let file = object::File::parse(&*data)?;
    println!("Parsed file");
    let defined = file
        .symbols()
        //.filter(|symbol| symbol.is_definition())
        .filter_map(|symbol| symbol.name_bytes().ok())
        .map(|bytes| bytes.to_vec())
        .collect();

    let undefined = file
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name_bytes().ok())
        .map(|bytes| bytes.to_vec())
        .collect();

    Ok((defined, undefined))
}

fn is_static_lib(file_name: &OsStr) -> bool {
    let Some(file_name) = file_name.to_str() else {
        return false;
    };
    LIB_REGEX.is_match(file_name)
}
