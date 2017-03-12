extern crate foreman;
use foreman::*;

fn main() {
    let project_dir = manifest_dir().unwrap();

    //Command::new("make -C crates/drivers/structure").status().unwrap();

    let driver_dir = project_dir.join("crates").join("drivers");
    link_search(SearchKind::Native, &driver_dir.join("structure"));
    link_search(SearchKind::Native, &driver_dir.join("bluefox"));
    link_search(SearchKind::Native, &driver_dir.join("optoforce"));
    link_search(SearchKind::Native, &driver_dir.join("biotac").join("src").join("wrapper"));
}
