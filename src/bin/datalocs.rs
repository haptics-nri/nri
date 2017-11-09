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

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, DirEntry};
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

/// Run a callback for each file inside another directory
fn for_each_file<P: AsRef<Path>, F: FnMut(DirEntry) -> Result<()>>(path: P, mut func: F) -> Result<()> {
    let path = path.as_ref();
    for entry in path.read_dir().chain_err(|| Io("list", path.into()))? {
        let entry = entry.chain_err(|| Io("read entry of", path.into()))?;
        if !entry.metadata().chain_err(|| Io("inspect", entry.path()))?.is_dir() {
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

        (@arg SURFACES: *    "Surfaces CSV file")
        (@arg DATADIR:  *... "Dataset directory (can be a directory or a Nickname=directory pair")

        (@arg AFTER:     -a --after [date] {|s| s.parse::<i32>()}
                              "Date when real data collection began (YYYYMMDD)")
        (@arg BEFORE:    -b --before [date] {|s| s.parse::<i32>()}
                              "Date when real data collection ended (YYYYMMDD)")
        (@arg CROPDIR:   -c --cropdir [dir] {|p| if Path::new(&p).is_dir() { Ok(()) } else { Err(format!("{} is not a directory", p)) }}
                              "Directory which should contain crops and CSV (will be cleared!)")
        (@arg CHECKDATA: -d --data "Check that data is present & processed")
    }.get_matches();

    let after = matches.value_of("AFTER")
                       .map_or(0, |a| a.parse().unwrap());
    let before = matches.value_of("BEFORE")
                       .map_or(20180509, |a| a.parse().unwrap());
    let check_data = matches.is_present("CHECKDATA");
    let cropdir = matches.value_of("CROPDIR").map(|p| Path::new(p));
    let args = matches.values_of("DATADIR").unwrap();

    let mut cropcsv = if let Some(cropdir) = cropdir {
        let mut all_crops = true;
        for_each_file(cropdir,
                      |ent| {
                          if !ent.path()
                                 .file_name().unwrap()
                                 .to_string_lossy()
                                 .starts_with("crop") {
                              all_crops = false;
                          }
                          Ok(())
                      })?;
        for_each_subdir(cropdir,
                        |_| {
                            all_crops = false;
                            Ok(())
                        })?;
        if !all_crops {
            bail!("Crop dir contains other stuff!");
        }

        fs::remove_dir_all(cropdir).chain_err(|| Io("delete", cropdir.into()))?;
        fs::create_dir(cropdir).chain_err(|| Io("create", cropdir.into()))?;

        let mut csv = csv::Writer::from_file(cropdir.join("crops.csv"))?;
        csv.encode(("Cropped image", "Date", "End-effector", "Episode number", "Data location"))?;
        Some(csv)
    } else {
        None
    };

    // step 1: catalog the YYYYMMDD dirs in each datadir
    
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
                if date >= after && date <= before {
                    datalocs.entry(date)
                        .or_insert(vec![])
                        .push(nickname);
                }
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
    let mut cropdescs = HashSet::new();
    for (i, (name, stick, opto, bio, loc1, loc2)) in surfaces.into_iter().enumerate() {
        let i = i+2; // google sheets is 1-based and has a header row
        let locs = [&loc1, &loc2];

        macro_rules! complain {
            ($msg:expr $(, $var:expr)*) => {
                println!(concat!("Row {} ({}) ", $msg), i, name $(, $var)*);
            }
        }

        let mut any_data = false;
        let mut crops = 0;

        // check that the episode for each present end-effector exists on each specified datadir
        for (endeff, &(ref date, ref num)) in vec![("stick", &stick), ("opto", &opto), ("bio", &bio)] {
            if !date.is_empty() && date.parse::<u32>().ok().map_or(false, |date| date >= after && date <= before) {
                if num.is_empty() {
                    complain!("has a {} date ({}) but no episode number", endeff, date);
                } else {
                    any_data = true;
                    episodes.entry((endeff, date.clone(), num.clone()))
                            .and_modify(|dupes: &mut Vec<_>| {
                                let dupes_desc = dupes.iter()
                                                      .map(|&(i, ref name)|
                                                           format!("row {} ({})", i, name))
                                                      .collect::<Vec<_>>().join(", ");
                                complain!("refers to {}/{}cam/{} which was already used in {}",
                                          date, endeff, num, dupes_desc);
                                dupes.push((i, name.clone()));
                            })
                            .or_insert_with(|| vec![(i, name.clone())]);

                    for loc in &locs {
                        if !loc.is_empty() {
                            if let Some(dir) = nicknames.get(&loc[..]) {
                                let path = Path::new(dir).join(date)
                                                             .join(format!("{}cam", &endeff))
                                                             .join(num);
                                if path.is_dir() {
                                    if let (Some(cropdir), Some(cropcsv)) = (cropdir.as_ref(), cropcsv.as_mut()) {
                                        let crop_path = path.join("crops");
                                        if crop_path.is_dir() {
                                            for_each_file(crop_path,
                                                          |ent| {
                                                              let fname = ent.path()
                                                                             .file_name().unwrap()
                                                                             .to_string_lossy().to_string();
                                                              if fname.contains("_crop") {
                                                                  crops += 1;
                                                                  let cropdesc = format!("{}{}{}{}",
                                                                                         date, endeff, num,
                                                                                         fname);
                                                                  if !cropdescs.contains(&cropdesc) {
                                                                       let newpath = cropdir.join(
                                                                           format!("crop{}.png",
                                                                                   cropdescs.len()));
                                                                       fs::copy(ent.path(),
                                                                                &newpath)
                                                                           .chain_err(|| Io("copy", ent.path()))?;
                                                                       cropcsv.encode((cropdescs.len(),
                                                                                       date, endeff, num,
                                                                                       loc))?;
                                                                  }
                                                                  cropdescs.insert(cropdesc);
                                                              }
                                                              Ok(())
                                                          })?;
                                        }
                                    }
                                    
                                    if check_data {
                                        match (path.join("teensy.dat").is_file(),
                                               path.join("teensy.ft.csv").is_file()) {
                                            (true , true ) => complain!("contains unnecessary raw {}cam data on {}", endeff, loc),
                                            (true , false) => complain!("has unprocessed {}cam data on {}", endeff, loc),
                                            (false, false) => complain!("is missing {}cam data on {}", endeff, loc),
                                            _ => {}
                                        }
                                    }
                                } else {
                                    complain!("claims {}/{}cam/{} is on {} but it isn't",
                                              date, endeff, num, loc);
                                }
                            } else {
                                complain!("refers to unknown location {}", loc);
                            }
                        }
                    }
                }
            }
        }

        if any_data {
            match (loc1.is_empty(), loc2.is_empty()) {
                (true , true ) => complain!("has no locations at all"),
                (true , false) => complain!("has no Location 1"),
                (false, true ) => complain!("has no Location 2"),
                _ => {}
            }

            if let Some(_) = cropdir {
                if crops == 0 {
                    complain!("has no crops");
                }
            }
        }
    }

    Ok(0)
});

