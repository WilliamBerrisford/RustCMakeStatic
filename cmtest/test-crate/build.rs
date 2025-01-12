use cmtest::{find_libs, link_to_dependencies, order_dependencies};
use std::path::Path;

fn main() {
    let lib = cmake::build("../../tinkwrap");

    cxx_build::bridge("src/main.rs")
        .file("include/bridge.cpp")
        .std("c++14")
        .compile("test-crate");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=include/bridge.cpp");
    println!("cargo:rerun-if-changed=include/bridge.h");
    println!("cargo:rerun-if-changed=../../tinkwrap");

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

    let ordered_libs = order_dependencies(all_libs);

    println!("Ordered dependencies: {:?}", ordered_libs);
    link_to_dependencies(ordered_libs);
}
