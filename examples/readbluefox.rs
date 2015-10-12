#[macro_use] extern crate lazy_static;
extern crate csv;
extern crate lodepng;

#[macro_use] mod common;

use std::{env, thread, process, fmt, mem, slice};
use std::sync::mpsc;
use std::io::{Read, Write};
use std::fs::File;
use std::path::{Path, PathBuf};
use lodepng::{encode_file, ColorType};

struct Row {
    pixels: [[u8; 3]; 1600]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Err(fmt::Error)
    }
}

fn main() {
    let inname = common::parse_in_arg(&mut env::args().skip(1));

    let mut csvfile = attempt!(File::open(&inname));
    let mut csvrdr = csv::Reader::from_reader(csvfile).has_headers(false);
    let mut csvwtr = csv::Writer::from_memory();
    csvwtr.encode(("Frame number", "Filename", "Unix timestamp"));

    const N_THREADS: usize = 4;
    let mut threads: [Option<(thread::JoinHandle<()>, mpsc::Sender<PathBuf>)>; N_THREADS] = unsafe { mem::uninitialized() };
    for i in 0..N_THREADS {
        let (tx, rx) = mpsc::channel::<PathBuf>();
        threads[i] = Some((
            thread::spawn(move || {
                for dat_path in rx {
                    let dat = dat_path.to_str().unwrap().to_string();
                    let png = dat_path.with_extension("png").to_str().unwrap().to_string();
                    indentln!("parsing {} into {}", dat, png);
                    let rows = common::do_binary::<Row>("", (dat, None));
                    let mut pixels = Vec::with_capacity(1200*3*rows.len());
                    for i in 0..rows.len() {
                        for j in 0..1600 {
                            pixels.push(rows[i].pixels[j]);
                        }
                    }
                    attempt!(encode_file(png, &pixels, 1600, rows.len(), ColorType::LCT_RGB, 8));
                }
            }),
            tx
        ));
    }

    let mut i = 0;
    let mut t = 0;
    for row in csvrdr.decode() {
        indentln!(> "reading frame {}...", i);
        let (num, fname, stamp): (usize, String, f64) = row.ok().expect(&format!("failed to parse row {} of {}", i, inname));
        csvwtr.encode((num, Path::new(&fname).with_extension("png").to_str().unwrap().to_string(), stamp));
        i += 1;
        let dat_path = Path::new(&inname).with_file_name(fname);
        threads[t].as_ref().unwrap().1.send(dat_path);
        t = (t + 1) % 4;
    }
    indentln!("finished {} frames", i);

    for t in 0..N_THREADS {
        attempt!(threads[t].take().unwrap().0.join());
    }

    attempt!(File::create(&inname)).write_all(csvwtr.as_bytes());
}

