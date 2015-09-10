fn main() {
    let project_dir = env!("CARGO_MANIFEST_DIR");
    //let out_dir = env!("OUT_DIR");

    //Command::new("make -C src/structure").status().unwrap();

    println!("cargo:rustc-link-search=native={}/src/structure", project_dir);
    println!("cargo:rustc-libdir={}/src/structure", project_dir);
    println!("cargo:rustc-link-search=native={}/src/bluefox", project_dir);
    println!("cargo:rustc-libdir={}/src/bluefox", project_dir);
    println!("cargo:rustc-link-search=native={}/src/optoforce", project_dir);
    println!("cargo:rustc-libdir={}/src/optoforce", project_dir);
}
