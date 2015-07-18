#[macro_use] extern crate lazy_static;
extern crate csv;

#[macro_use] mod common;

use std::{env, process, fmt};
use std::fs::File;
use std::path::Path;

struct Data {
    pixels: [u16; 640]
}

impl fmt::Debug for Data {
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
        let (num, stamp): (usize, f64) = row.ok().expect(&format!("failed to parse row {} of {}", i, inname));
        i += 1;
        let dat = Path::new(&inname).with_file_name(format!("structure{}.dat", i)).to_str().unwrap().to_string();
        let csv = Path::new(&inname).with_file_name(format!("structure{}.csv", i)).to_str().unwrap().to_string();
        indentln!("parsing {} into {}", dat, csv);
        common::do_binary::<Data>("", (dat, csv));
    }
    indentln!("finished {} frames", i);
}

