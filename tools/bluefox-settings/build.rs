extern crate foreman;
use foreman::*;

fn go() -> Result<()> {
    let project_dir = manifest_dir()?;
    link_search(SearchKind::Native, &project_dir.join("..").join("..").join("lib"));

    Ok(())
}

fn main() {
    go().unwrap();
}

