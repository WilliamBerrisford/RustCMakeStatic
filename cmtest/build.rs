use cmake;

fn main() {
    let lib = cmake::build("../tinkwrap");

    cxx_build::bridge("src/main.rs")
        .file("include/bridge.cpp")
        .std("c++14")
        .compile("cmtest");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=include/bridge.cpp");
    println!("cargo:rerun-if-changed=include/bridge.h");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=tinkwrap");
}
