//use std::path::Path;
use std::process::Command;

fn main() {
    //let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    //let out_dir = Path::new(env!("OUT_DIR"));

    Command::new("make").status().unwrap();

    println!("cargo:rustc-link-search=native=src/structure");
}

