extern crate csv;
extern crate hprof;
#[macro_use] extern crate error_chain;

use std::env;
use std::io::{self, Write};
use std::fs::File;
use std::path::Path;
use std::process::Command;
use hprof::Profiler;

error_chain! {
    foreign_links {
        Csv(csv::Error);
        Io(io::Error);
    }
}

fn april(fname: String, prof: &Profiler) -> (u32, String, String, String, String, String, String) {
    let converted = env::temp_dir().join("bluefox.pnm");

    {
        let _g = prof.enter("convert PNG>PNM");
        // use imagemagick to convert PNG to PNM
        assert!(Command::new("convert")
                .arg(&fname)
                .arg(&converted)
                .status().unwrap()
                .success());
    }

    let output = {
        let _g = prof.enter("apriltag-c");
        // run apriltag-c program
        let output = Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("april").join("apriltag-2015-03-18").join("apriltag_demo"))
            .arg(&converted)
            .output().unwrap()
            .stdout;
        String::from_utf8(output).unwrap()
    };

    {
        let _g = prof.enter("read output");
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
}

fn try_main() -> Result<()> {
    let prof = Profiler::new("");

    let mut aprwtr = csv::Writer::from_memory();
    aprwtr.encode(("Frame number", "Tag IDs", "Tag Centers", "Tag P1s", "Tag P2s", "Tag P3s", "Tag P4s"))?;
    let inname = env::args().skip(1).next().unwrap();
    let csvfile = File::open(&inname)?;
    let mut csvrdr = csv::Reader::from_reader(csvfile).has_headers(false);
    let mut csvwtr = csv::Writer::from_memory();
    csvwtr.encode(("Frame number", "Filename", "Unix timestamp"))?;

    for row in csvrdr.decode() {
        let (num, _, stamp): (u32, String, String) = row?;
        let fname = format!("bluefox{}.png", num);
        let png = Path::new(&inname).parent().unwrap().join("bluefox").join(&fname);
        println!("{}", png.display());
        if Path::exists(&png) {
            aprwtr.encode(april(png.display().to_string(), &prof))?;
            csvwtr.encode((num, fname, stamp))?;
        }
    }

    File::create(Path::new(&inname).parent().unwrap().join("bluefox").join("april.csv"))?.write_all(aprwtr.as_bytes())?;
    File::create(Path::new(&inname).parent().unwrap().join("bluefox").join("bluefox_times.csv"))?.write_all(csvwtr.as_bytes())?;

    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("ERROR: {:?}", e);
    }
}

