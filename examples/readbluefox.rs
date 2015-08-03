#[macro_use] extern crate lazy_static;
extern crate csv;
extern crate image;

#[macro_use] mod common;

use std::{env, process, fmt, mem, slice};
use std::fs::File;
use std::path::Path;
use self::image::{ImageBuffer, ColorType};
use self::image::png::PNGEncoder;

struct Row {
    pixels: [[u8; 3]; 1200]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Err(fmt::Error)
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
        let mut pngfile = File::create(png).unwrap();
        PNGEncoder::new(&mut pngfile).encode(
            unsafe {
                slice::from_raw_parts::<u8>(&rows[0] as *const Row as *const u8,
                                            rows.len()*1200*3)
            },
            rows.len() as u32, 1200u32,
            ColorType::RGB(8)).unwrap();
    }
    indentln!("finished {} frames", i);
}

