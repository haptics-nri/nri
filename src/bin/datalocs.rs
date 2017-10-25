#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate csv;

error_chain! {
    errors {
        Io(op: &'static str, path: PathBuf) {
            description("I/O operation failed")
            display("Could not {} {}", op, path.display())
        }
    }

    foreign_links {
        Csv(csv::Error);
    }
}
use ErrorKind::*;

use std::collections::BTreeMap;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

fn for_each_subdir<P: AsRef<Path>, F: FnMut(DirEntry) -> Result<()>>(path: P, mut func: F) -> Result<()> {
    let path = path.as_ref();
    for entry in path.read_dir().chain_err(|| Io("list", path.into()))? {
        let entry = entry.chain_err(|| Io("read entry of", path.into()))?;
        if entry.metadata().chain_err(|| Io("inspect", entry.path()))?.is_dir() {
            func(entry)?;
        }
    }
    Ok(())
}

quick_main!(|| -> Result<i32> {
    let matches = clap_app! { nri_render =>
        (version: crate_version!())
        (author: crate_authors!("\n"))
        (about: "Helper for tabulating data locations")

        (@arg SURFACES: +required "Surfaces CSV file")
        (@arg DATADIR: +required +multiple "Dataset directory")
    }.get_matches();

    let args = matches.values_of("DATADIR").unwrap();
    let mut datalocs = BTreeMap::<u32, _>::new();
    let mut nicknames = BTreeMap::new();
    for arg in args {
        let mut sp = arg.splitn(2, '=');
        let nickname = sp.next().ok_or("empty argument")?;
        let datadir = sp.next().unwrap_or(nickname);
        nicknames.insert(nickname, datadir);

        for_each_subdir(datadir, |date| {
            if let Ok(date) = date.file_name().to_string_lossy().parse() {
                datalocs.entry(date)
                    .or_insert(vec![])
                    .push(nickname);
            }
            Ok(())
        })?;
    }

    let mut good = vec![];
    let mut bad = vec![];
    for (date, dirs) in datalocs {
        let mut ok = true;
        let dirs = {
            let mut contents = BTreeMap::new();
            let mut longest = vec![];
            for dir in dirs {
                let path = Path::new(nicknames[dir]).join(date.to_string());
                let mut eps = vec![];
                for_each_subdir(path, |endeff| {
                    for_each_subdir(endeff.path(), |ep| {
                        if let Ok(ep) = ep.file_name().to_string_lossy().parse::<u8>() {
                            eps.push((endeff.file_name().to_string_lossy().into_owned(), ep));
                        }
                        Ok(())
                    })
                })?;
                eps.sort();
                if eps.len() > longest.len() {
                    longest = eps.clone();
                }
                contents.insert(dir, eps);
            }

            contents.into_iter()
                    .map(|(dir, eps)| {
                        if eps == longest {
                            format!("{}({})", dir, eps.len())
                        } else {
                            ok = false;
                            format!("{}({}!)", dir, eps.len())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
        };

        if ok {
            good.push((date, dirs));
        } else {
            bad.push((date, dirs));
        }
    }

    println!("GOOD DATADIRS:");
    for (date, dirs) in good { println!("{}\t{}", date, dirs); }
    println!("\nBAD DATADIRS:");
    for (date, dirs) in bad { println!("{}\t{}", date, dirs); }

    let surf_file = matches.value_of("SURFACES").unwrap();
    let surfaces = csv::Reader::from_file(surf_file)?
        .records()
        .map(|row| row.map_err(Into::into)
                      .map(|row| (row[0].clone(),                   // surface name
                                  (row[2].clone(), row[3].clone()), // stick
                                  (row[4].clone(), row[5].clone()), // opto
                                  (row[6].clone(), row[7].clone()), // bio
                                  row[10].clone(),                  // loc 1
                                  row[11].clone())))                // loc 2
        .collect::<Result<Vec<_>>>()?;

    println!("\nEXCEPTIONS:");
    for (i, (name, stick, opto, bio, loc1, loc2)) in surfaces.into_iter().enumerate() {
        let i = i+2;
        let mut locs = vec![];

        if loc1.is_empty() && loc2.is_empty() {
            println!("Row {} ({}) has no locations at all.", i, name);
        } else {
            if loc1.is_empty() {
                println!("Row {} ({}) has no Location 1", i, name);
            } else {
                locs.push(loc1);
            }

            if loc2.is_empty() {
                println!("Row {} ({}) has no Location 2", i, name);
            } else {
                locs.push(loc2);
            }

            for loc in locs {
                for (endeff, &(ref date, ref num)) in vec![("stick", &stick), ("opto", &opto), ("bio", &bio)] {
                    if !date.is_empty() {
                        if num.is_empty() {
                            println!("Row {} ({}) has a {} date but no episode number", i, name, endeff);
                        } else {
                            if let Some(dir) = nicknames.get(&loc[..]) {
                                if !Path::new(dir).join(date)
                                                  .join(format!("{}cam", &endeff))
                                                  .join(num)
                                                  .is_dir() {
                                    println!("Row {} ({}) claims {}/{}cam/{} is on {} but it isn't", i, name, date, endeff, num, loc);
                                }
                            } else {
                                println!("Row {} ({}) refers to unknown location {}", i, name, loc);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(0)
});

