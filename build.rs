use std::process::Command;

fn main() {
    let project_dir = env!("CARGO_MANIFEST_DIR");
    //let out_dir = env!("OUT_DIR");

    Command::new("make").status().unwrap();

    println!("cargo:rustc-link-search=native={}/src/structure", project_dir);
    println!("cargo:rustc-libdir={}/src/structure", project_dir);
}

