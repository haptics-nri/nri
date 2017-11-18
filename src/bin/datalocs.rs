#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate unborrow;
extern crate boolinator;
extern crate csv;
extern crate image;
extern crate serde;
extern crate tabwriter;

extern crate utils;

error_chain! {
    errors {
        Io(op: &'static str, path: PathBuf) {
            description("I/O operation failed")
            display("Could not {} {}", op, path.display())
        }

        Row(i: usize, file: PathBuf, msg: String) {
            description("error while processing row")
            display("error in row {} of {}: {}", i, file.display(), msg)
        }
    }

    foreign_links {
        Csv(csv::Error);
        Image(image::ImageError);
    }
}
use ErrorKind::*;
use std::result::Result as StdResult;

use image::GenericImage;
use serde::Serializer;
use tabwriter::TabWriter;

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

use utils::prelude::*;

#[derive(Clone, Serialize)]
struct Amazon {
    date: u32,
    hit_id: String,
    assignment_id: String,
    worker_id: String,
    work_time: u32,
    crop_num: u32,
    #[serde(serialize_with="serialize_cropinfo")] source: CropInfo,
    surface: String,
    img_width: u32,
    img_height: u32,
    answer_quality: String,
    answer_shape: String,
    answer_hard: u8,
    answer_rough: u8,
    answer_sticky: u8,
    answer_warm: u8,
}

fn serialize_cropinfo<S: Serializer>(ci: &CropInfo, ser: S) -> StdResult<S::Ok, S::Error> {
    ser.serialize_str(&format!("{}-{}-{}", ci.date, ci.end_effector, ci.episode_number))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AmazonRaw {
    #[serde(rename = "HITId")] hit_id: String,
    assignment_id: String,
    worker_id: String,
    #[serde(rename = "WorkTimeInSeconds")] work_time: u32,
    #[serde(rename = "Input.image_url")] input_image_url: String,
    #[serde(rename = "Input.image_width")] input_image_width: u32,
    #[serde(rename = "Input.image_height")] input_image_height: u32,
    #[serde(rename = "Answer.quality")] answer_quality: String,
    #[serde(rename = "Answer.shape")] answer_shape: String,
    #[serde(rename = "Answer.hard")] answer_hard: u8,
    #[serde(rename = "Answer.rough")] answer_rough: u8,
    #[serde(rename = "Answer.sticky")] answer_sticky: u8,
    #[serde(rename = "Answer.warm")] answer_warm: u8,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CropInfo {
    #[serde(rename = "Cropped image")] cropped_image: u32,
    date: u32,
    #[serde(rename = "End-effector")] end_effector: String,
    #[serde(rename = "Episode number")] episode_number: u32,
    #[serde(rename = "Data location")] data_location: String,
}

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
        (@arg CROPDIR:   -c --cropdir [dir]
                              "Directory which should contain crops and CSV (will be cleared!)")
        (@arg CROPURL:   -C --cropurl [url] requires[CROPDIR] "URL where cropdir will be accessible")
        (@arg CHECKDATA: -d --data "Check that data is present & processed")
        (@arg AMAZON:    -A --amazon [dir] "Load and analyze Amazon study data")
    }.get_matches();

    let after = matches.value_of("AFTER")
                       .map_or(0, |a| a.parse().unwrap());
    let before = matches.value_of("BEFORE")
                       .map_or(20180509, |a| a.parse().unwrap());
    let check_data = matches.is_present("CHECKDATA");
    let cropdir = matches.value_of("CROPDIR").map(|p| Path::new(p));
    let prefix = matches.value_of("CROPURL").unwrap_or("");
    let amazon = matches.value_of("AMAZON").map(|p| Path::new(p));
    let args = matches.values_of("DATADIR").unwrap();

    let mut cropcsv = if let Some(cropdir) = cropdir {
        if cropdir.is_dir() {
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
        }

        fs::create_dir(cropdir).chain_err(|| Io("create", cropdir.into()))?;

        let mut csv1 = csv::Writer::from_path(cropdir.join("crops.csv"))?;
        csv1.serialize(("Cropped image", "Date", "End-effector", "Episode number", "Data location"))?;
        let mut csv2 = csv::Writer::from_path(cropdir.join("crop_amazon.csv"))?;
        csv2.serialize(("image_url", "image_width", "image_height"))?;
        Some((csv1, csv2))
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

    let mut good = BTreeMap::new(); // dates with at least two locations and no missing episodes in any location
    let mut bad = BTreeMap::new();
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
            good.insert(date, (dir_desc, None));
        } else {
            bad.insert(date, (dir_desc, None));
        }
    }

    // step 3: parse CSV

    let surf_file = matches.value_of("SURFACES").unwrap();
    let surfaces = csv::Reader::from_path(surf_file)?
        .records()
        .map(|row| row.map_err(Into::into)
                      .map(|row| (row[0].to_owned(),                   // surface name
                                  (row[2].to_owned(), row[3].to_owned()), // stick
                                  (row[4].to_owned(), row[5].to_owned()), // opto
                                  (row[6].to_owned(), row[7].to_owned()), // bio
                                  row[10].to_owned(),                  // loc 1
                                  row[11].to_owned())))                // loc 2
        .collect::<Result<Vec<_>>>()?;
    
    if let Some(amazon_path) = amazon {
        for ent in amazon_path.read_dir().chain_err(|| Io("list", amazon_path.into()))? {
            let ent = ent.chain_err(|| Io("read entry", amazon_path.into()))?;
            let meta = ent.metadata().chain_err(|| Io("stat", ent.path()))?;
            if meta.is_dir() {
                let date = ent.path().file_name().unwrap().to_str().unwrap().parse().unwrap();
                if let Ok(mut amz_rdr) = csv::ReaderBuilder::new()
                                                            .flexible(true)
                                                            .from_path(ent.path().join("amazon_output_raw.csv")) {
                    // fix up study results headers
                    let headers = {
                        let mut headers = amz_rdr.headers()?.clone();
                        unborrow!(headers.truncate(headers.len() - 2));
                        headers
                    };
                    amz_rdr.set_headers(headers);

                    // read all crops for cross-referencing
                    let mut crops_rdr = csv::Reader::from_path(ent.path().join("crops.csv"))?;
                    let crops: Vec<CropInfo> = crops_rdr.deserialize()
                        .map(|r| r.map_err(Into::into))
                        .collect::<Result<_>>()?;

                    // munge ratings
                    // write new file and enter data in surfaces map

                    let mut ratings = vec![];
                    for (i, raw) in amz_rdr.deserialize().enumerate() {
                        let raw: AmazonRaw = raw.chain_err(|| Row(i, ent.path(), "parse error".into()))?;

                        let crop_num = Path::new(&raw.input_image_url)
                            .file_stem().unwrap()
                            .to_str().unwrap()[4..]
                            .parse::<u32>().unwrap();

                        let mut source = None;
                        for crop in &crops {
                            if crop.cropped_image == crop_num {
                                source = Some(crop);
                                break;
                            }
                        }
                        let source = source.ok_or_else(|| Row(i, ent.path(), format!("no entry for crop #{}", crop_num)))?;

                        let mut surface = None;
                        for &(ref name, ref stick, ref opto, ref bio, ..) in &surfaces {
                            let spec = match &source.end_effector[..] {
                                "stick" => stick,
                                "opto" => opto,
                                "bio" => bio,
                                _ => unreachable!()
                            };

                            if spec == &(source.date.to_string(), source.episode_number.to_string()) {
                                surface = Some(name.clone());
                                break;
                            }
                        }
                        let surface = surface.ok_or_else(|| Row(i, ent.path(), format!("no surface matching {}-{}-{}", source.date, source.end_effector, source.episode_number)))?;

                        let info = Amazon {
                            date,
                            crop_num,
                            surface,
                            source: source.clone(),
                            hit_id: raw.hit_id,
                            assignment_id: raw.assignment_id,
                            worker_id: raw.worker_id,
                            work_time: raw.work_time,
                            img_width: raw.input_image_width,
                            img_height: raw.input_image_height,
                            answer_quality: raw.answer_quality,
                            answer_shape: raw.answer_shape,
                            answer_hard: raw.answer_hard,
                            answer_rough: raw.answer_rough,
                            answer_sticky: raw.answer_sticky,
                            answer_warm: raw.answer_warm,
                        };
                        ratings.push(info.clone());

                        good.get_mut(&source.date)
                            .or_else(|| bad.get_mut(&source.date))
                            .ok_or_else(|| Error::from_kind(Row(i, ent.path(), format!("no such episode date {}", source.date))))?
                            .1.get_or_insert(vec![])
                              .push(info);
                    }

                    let mut amz_wtr = csv::Writer::from_path(ent.path().join("amazon_output_cooked.csv"))?;
                    for rating in ratings {
                        amz_wtr.serialize(rating)?;
                    }
                }
            }
        }
    }

    // step 4: print naughty/nice list

    let out = &mut TabWriter::new(io::stdout());
    println!("GOOD DATADIRS:");
    for (date, (dirs, amz)) in good { writeln!(out, "{}\t{}\t{}", date, dirs, amz.map_or(0, |v| v.len())).unwrap(); }
    out.flush().unwrap();
    println!("\nBAD DATADIRS:");
    for (date, (dirs, amz)) in bad { writeln!(out, "{}\t{}\t{}", date, dirs, amz.map_or(0, |v| v.len())).unwrap(); }
    out.flush().unwrap();

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
                                    if let (Some(cropdir), Some(&mut (ref mut csv1, ref mut csv2))) = (cropdir.as_ref(), cropcsv.as_mut()) {
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
                                                                       csv1.serialize((cropdescs.len(),
                                                                                    date, endeff, num,
                                                                                    loc))?;
                                                                       let img = image::open(&newpath)?;
                                                                       csv2.serialize((format!("{}crop{}.png",
                                                                                            prefix,
                                                                                            cropdescs.len()),
                                                                                    img.width(), img.height()))?;
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

