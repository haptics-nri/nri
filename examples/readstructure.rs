#[macro_use] extern crate lazy_static;
extern crate csv;
extern crate lodepng;

#[macro_use] mod common;

use std::{env, process, fmt};
use std::fs::File;
use std::path::Path;
use lodepng::{encode_file, ColorType};

struct Row {
    pixels: [u16; 640]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for col in 0..640 {
            try!(write!(f, "{}, ", self.pixels[col]));
        }
        try!(write!(f, "\n"));
        Ok(())
    }
}

fn main() {
    let inname = common::parse_in_arg(&mut env::args().skip(1));

    let mut csvrdr = csv::Reader::from_reader(attempt!(File::open(&inname))).has_headers(false);
    let mut i = 0;
    for row in csvrdr.decode() {
        indentln!(> "reading frame {}...", i);
        let (num, fname, stamp): (usize, String, f64) = row.ok().expect(&format!("failed to parse row {} of {}", i, inname));
        i += 1;
        let dat_path = Path::new(&inname).with_file_name(fname);
        let dat = dat_path.to_str().unwrap().to_string();
        let png = dat_path.with_extension("png").to_str().unwrap().to_string();
        indentln!("parsing {} into {}", dat, png);
        let rows = common::do_binary::<Row>("", (dat, None));
        indentln!("have {} rows", rows.len());
        let mut pixels = Vec::with_capacity(640*rows.len());
        for i in 0..rows.len() {
            for j in 0..640 {
                pixels.push(rows[i].pixels[j]);
            }
        }
        println!("{} pixels!", pixels.len());
        attempt!(encode_file(png, &pixels, 640, rows.len(), ColorType::LCT_GREY, 16));
    }
    indentln!("finished {} frames", i);
}

