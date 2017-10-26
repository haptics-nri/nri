#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate csv;

extern crate utils;

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

use utils::prelude::*;

/// Run a callback for each directory inside another directory
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
        (@arg DATADIR: +required +multiple "Dataset directory (can be a directory or a Nickname=directory pair")
    }.get_matches();

    // step 1: catalog the YYYYMMDD dirs in each datadir
    
    let args = matches.values_of("DATADIR").unwrap();
    let mut datalocs = BTreeMap::<u32, _>::new();
    let mut nicknames = BTreeMap::new();
    for arg in args {
        // extract nickname if present (otherwise set nickname to dirname)
        let mut sp = arg.splitn(2, '=');
        let nickname = sp.next().ok_or("empty argument")?;
        let datadir = sp.next().unwrap_or(nickname);
        nicknames.insert(nickname, datadir);

        // scan the datadir
        for_each_subdir(datadir, |date| {
            if let Ok(date) = date.file_name().to_string_lossy().parse() {
                datalocs.entry(date)
                    .or_insert(vec![])
                    .push(nickname);
            }
            Ok(())
        })?;
    }

    // step 2: check status of each YYYYMMDD

    let mut good = vec![]; // dates with at least two locations and no missing episodes in any location
    let mut bad = vec![];
    for (date, dirs) in datalocs {
        let mut ok = true; // will be falsified if we find an issue with this date
        let dir_desc = {
            let mut contents = BTreeMap::new(); // map from nickname to episodes from this date
            let mut longest = vec![]; // longest list of episodes (in order to find out if any datadirs are missing episodes)
            for dir in &dirs {
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
                            // datadirs with missing episodes get an exclamation point
                            ok = false;
                            format!("{}({}!)", dir, eps.len())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
        };

        if ok && dirs.len() > 1 {
            good.push((date, dir_desc));
        } else {
            bad.push((date, dir_desc));
        }
    }

    // step 3: print naughty/nice list

    println!("GOOD DATADIRS:");
    for (date, dirs) in good { println!("{}\t{}", date, dirs); }
    println!("\nBAD DATADIRS:");
    for (date, dirs) in bad { println!("{}\t{}", date, dirs); }

    // step 4: parse CSV

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

    // step 5: look for and print various problems:
    //  - missing info about data location in the CSV (or reference to a datadir that wasn't passed
    //    in on the command line)
    //  - duplicate/incomplete entries in the CSV
    //  - inaccurate info in the CSV (specifying a datadir that doesn't contain the data)

    println!("\nEXCEPTIONS:");
    let mut episodes = BTreeMap::new();
    for (i, (name, stick, opto, bio, loc1, loc2)) in surfaces.into_iter().enumerate() {
        let i = i+2; // google sheets is 1-based and has a header row
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

            // check that the episode for each present end-effector exists on each specified datadir
            for (endeff, &(ref date, ref num)) in vec![("stick", &stick), ("opto", &opto), ("bio", &bio)] {
                if !date.is_empty() {
                    if num.is_empty() {
                        println!("Row {} ({}) has a {} date but no episode number", i, name, endeff);
                    } else {
                        episodes.entry((endeff, date.clone(), num.clone()))
                                .and_modify(|dupes: &mut Vec<_>| {
                                    let dupes_desc = dupes.iter()
                                                          .map(|&(i, ref name)|
                                                               format!("row {} ({})", i, name))
                                                          .collect::<Vec<_>>().join(", ");
                                    println!("Row {} ({}) refers to {}/{}cam/{} which was already used in {}",
                                             i, name, date, endeff, num, dupes_desc);
                                    dupes.push((i, name.clone()));
                                })
                                .or_insert_with(|| vec![(i, name.clone())]);

                        for loc in &locs {
                            if let Some(dir) = nicknames.get(&loc[..]) {
                                if !Path::new(dir).join(date)
                                                  .join(format!("{}cam", &endeff))
                                                  .join(num)
                                                  .is_dir() {
                                    println!("Row {} ({}) claims {}/{}cam/{} is on {} but it isn't",
                                             i, name, date, endeff, num, loc);
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

