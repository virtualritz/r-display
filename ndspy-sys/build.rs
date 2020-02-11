// build.rs
extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // TODO: make this generic & work on Linux/Windows

    let delight =
        &env::var("DELIGHT").expect("DELIGHT environment variable not set – cannot find 3Delight.");

    // Emit linker searchpath
    /*
    println!(
        "cargo:rustc-link-search={}",
        Path::new(delight).join("lib").display()
    );
    println!("cargo:rustc-link-lib=3delight");
    */

    // Build bindings
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Searchpath
        .clang_arg(format!(
            "-I{}",
            Path::new(delight).join("include").display()
        ))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("ndspy-bindings.rs"))
        .expect("Couldn't write bindings.");
}
