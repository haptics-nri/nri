fn main() {
    let project_dir = env!("CARGO_MANIFEST_DIR");
    //let out_dir = env!("OUT_DIR");

    //Command::new("make -C crates/drivers/structure").status().unwrap();

    println!("cargo:rustc-link-search=native={}/crates/drivers/structure", project_dir);
    println!("cargo:rustc-libdir={}/crates/drivers/structure", project_dir);
    println!("cargo:rustc-link-search=native={}/crates/drivers/bluefox", project_dir);
    println!("cargo:rustc-libdir={}/crates/drivers/bluefox", project_dir);
    println!("cargo:rustc-link-search=native={}/crates/drivers/optoforce", project_dir);
    println!("cargo:rustc-libdir={}/crates/drivers/optoforce", project_dir);
    println!("cargo:rustc-link-search=native={}/crates/drivers/biotac/src/wrapper", project_dir);
    println!("cargo:rustc-libdir={}/crates/drivers/biotac/src/wrapper", project_dir);
}
