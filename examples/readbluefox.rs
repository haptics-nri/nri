#[macro_use] extern crate lazy_static;
#[macro_use] mod common;
extern crate lodepng;
extern crate csv;

use std::{env, fmt};
use std::fs::File;
use std::path::Path;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::process::Command;
use lodepng::ColorType;

struct Row {
    pixels: [[u8; 3]; 1600]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Err(fmt::Error)
    }
}

impl common::Pixels<[u8; 3]> for Row {
    fn pixel(&self, i: usize) -> [u8; 3] {
        self.pixels[i]
    }
}

fn april(fname: String) -> (u32, String, String, String, String, String, String) {
    let converted = env::temp_dir().join("bluefox.pnm");

    // use imagemagick to convert PNG to PNM
    assert!(Command::new("convert")
            .arg(&fname)
            .arg(&converted)
            .status().unwrap()
            .success());

    // run apriltag-c program
    let output = Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("april").join("apriltag-2015-03-18").join("apriltag_demo"))
        .arg(&converted)
        .output().unwrap()
        .stdout;
    let output = String::from_utf8(output).unwrap();

    // split by line and match parts by regex (or change output to be parseable without regex)
    let frame_number: u32 = Path::new(&fname).with_extension("").file_name().unwrap().to_str().unwrap()[7..].parse().unwrap();
    let mut ids: Vec<u32> = vec![];
    let mut centers: Vec<(f64, f64)> = vec![];
    let mut p1s: Vec<(f64, f64)> = vec![];
    let mut p2s: Vec<(f64, f64)> = vec![];
    let mut p3s: Vec<(f64, f64)> = vec![];
    let mut p4s: Vec<(f64, f64)> = vec![];
    for line in output.lines().skip(1) {
        let mut sections = line.split(';');

        let id = sections.next().unwrap();
        let center = sections.next().unwrap();
        let p1 = sections.next().unwrap();
        let p2 = sections.next().unwrap();
        let p3 = sections.next().unwrap();
        let p4 = sections.next().unwrap();

        ids.push(id.parse().unwrap());
        let mut center_coords = center.split(',');
        centers.push((center_coords.next().unwrap().parse().unwrap(), center_coords.next().unwrap().parse().unwrap()));
        let mut p1_coords = p1.split(',');
        p1s.push((p1_coords.next().unwrap().parse().unwrap(), p1_coords.next().unwrap().parse().unwrap()));
        let mut p2_coords = p2.split(',');
        p2s.push((p2_coords.next().unwrap().parse().unwrap(), p2_coords.next().unwrap().parse().unwrap()));
        let mut p3_coords = p3.split(',');
        p3s.push((p3_coords.next().unwrap().parse().unwrap(), p3_coords.next().unwrap().parse().unwrap()));
        let mut p4_coords = p4.split(',');
        p4s.push((p4_coords.next().unwrap().parse().unwrap(), p4_coords.next().unwrap().parse().unwrap()));
    }

    (frame_number,
     ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>().join(";"),
     centers.into_iter().map(|(x,y)| format!("{},{}", x, y)).collect::<Vec<_>>().join(";"),
     p1s.into_iter().map(|(x,y)| format!("{},{}", x, y)).collect::<Vec<_>>().join(";"),
     p2s.into_iter().map(|(x,y)| format!("{},{}", x, y)).collect::<Vec<_>>().join(";"),
     p3s.into_iter().map(|(x,y)| format!("{},{}", x, y)).collect::<Vec<_>>().join(";"),
     p4s.into_iter().map(|(x,y)| format!("{},{}", x, y)).collect::<Vec<_>>().join(";"))
}

fn main() {
    let mut csvwtr = csv::Writer::from_memory();
    csvwtr.encode(("Frame number", "Tag IDs", "Tag Centers", "Tag P1s", "Tag P2s", "Tag P3s", "Tag P4s"));
    let csvwtr = Arc::new(Mutex::new(csvwtr));

    let inname = common::do_camera::<[u8; 3], Row, _, _>("bluefox", |png, csvwtr| csvwtr.lock().unwrap().encode(april(png)).unwrap(), csvwtr.clone(), 1600, 1200, 3, ColorType::LCT_RGB, 8);

    let mut csvwtr = Arc::try_unwrap(csvwtr).ok().unwrap().into_inner().unwrap();
    attempt!(File::create(Path::new(&inname).parent().unwrap().join("bluefox").join("april.csv"))).write_all(csvwtr.as_bytes());
}

