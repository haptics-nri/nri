#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate cast;
extern crate csv;
extern crate image;
extern crate indicatif;
extern crate line_drawing;
extern crate nalgebra as na;
extern crate rayon;
extern crate tempdir;

extern crate nri;

use std::{cmp, str};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cast::{u8, u32, u64, i32, f64};
use image::{Pixel, RgbaImage};
use line_drawing::XiaolinWu as Line;
use rayon::prelude::*;
use tempdir::TempDir;

error_chain! {
    errors {
        Io(op: &'static str, path: PathBuf) {
            description("I/O operation failed")
            display("Could not {} {}", op, path.display())
        }
    }

    foreign_links {
        Cast(cast::Error);
        Csv(csv::Error);
        Image(image::ImageError);
    }
}
use ErrorKind::*;

quick_main!(|| -> Result<i32> {
    let matches = clap_app! { nri_render =>
        (version: crate_version!())
        (author: crate_authors!("\n"))
        (about: "Helper for rendering stuff onto images/movies")

        (@arg EPDIR: +required +multiple "Episode directory")
        (@arg DEST: +required "Destination (path)")
    }.get_matches();

    for epdir in matches.values_of("EPDIR").unwrap() {
        println!("Processing {}...", epdir);

        // step 1: read bluefox data from CSV files
        
        let mut bt = csv::Reader::from_file([epdir, "bluefox", "bluefox_times.csv"].iter().collect::<PathBuf>())?;
        let mut april = csv::Reader::from_file([epdir, "bluefox", "april.csv"].iter().collect::<PathBuf>())?;

        // format of bluefox_times.csv is "Frame number (int), Filename (str), Unix Timestamp (float)"
        let frames = bt.decode()
                       .map(|r| r.map_err(Into::into))
                       .collect::<Result<Vec<(u32, String, f64)>>>()?;
        // format of april.csv is "Frame number (int), Tag IDs (ints, semicolon-sep), Tag centers (float comma-sep coord pairs, semicolon-sep), ..."
        let aprils = april.decode()
                          .map(|r| r.map_err(Into::into)
                                    .map(|(num, ids, centers, _p1s, _p2s, _p3s, _p4s): (u32, String, String, String, String, String, String)| {
                                        (num,
                                         ids.split(';')
                                            .filter(|s| !s.is_empty())
                                            .map(|s| s.parse().expect("failed to parse id"))
                                            .zip(centers.split(';')
                                                        .map(|s| {
                                                            let mut sp = s.split(',');
                                                            (sp.next().unwrap().parse().expect("failed to parse center X"),
                                                             sp.next().unwrap().parse().expect("failed to parse center Y"))
                                                        }))
                                            .collect())
                          }))
                          .collect::<Result<BTreeMap<u32, HashMap<u32, (f64, f64)>>>>()?;
                          // collect into a map from frame number to (a map from tag ID to tag center)
                          // the BTreeMap keeps the frames in order for later iteration
        

        // step 2: overlay the path-so-far on each frame
        
        let first_frame = frames.iter().map(|&(num, _, _)| num).min().ok_or("no frames")?;
        let bar = nri::make_bar(u64(frames.len()));
        bar.set_message("Process");

        // store frames in a temporary directory before running FFMPEG
        let framedir = TempDir::new("nri").chain_err(|| Io("create", "temp dir".into()))?;
        // process frames using all available CPUs
        let units = frames.into_par_iter()
            .map(|(fa, filename, _stamp)| {
                // step 2a: load camera frame from file
                
                let mut from = Path::new(epdir).to_owned();
                from.push("bluefox");
                from.push(&filename);
                let mut to = framedir.path().to_owned();
                to.push(&filename);

                let mut img = image::open(from)?.to_rgba();

                // step 2b: plot transformed end-effector location from all frames up to now

                // everything labeled "a" is the frame we're drawing on, while "b" is the frame being transformed from
                let apra = &aprils[&fa];
                let ida = apra.keys().collect::<HashSet<_>>(); // tag IDs visible in the current frame
                let pta = na::VectorN::<_, na::U3>::from_row_slice(&[778., 760., 1.]); // empirical end-effector location
                let mut prev: Option<na::VectorN<f64, na::U3>> = None;
                for (&fb, aprb) in &aprils {
                    if fb > fa { break } // only frames up to the current one

                    if aprb.len() > 20 { // bail if there aren't enough tags
                        let idb = aprb.keys().collect::<HashSet<_>>();
                        let inter = ida.intersection(&idb).collect::<Vec<_>>();
                        if inter.len() >= 4 { // bail if there aren't enough tags in common
                            // extract centers of tags visible in both frames
                            let ctra = inter.iter().map(|id| apra[id]).collect::<Vec<_>>();
                            let ctrb = inter.iter().map(|id| aprb[id]).collect::<Vec<_>>();

                            // fit affine transformation to tag centers
                            // the equation here is Ax=b, where:
                            // A = [ x1b, y1b,   0,   0, 1, 0 ]
                            //     [   0,   0, x1b, y1b, 0, 1 ]
                            //     [ ...       ...       ...  ]
                            // x = [ m1 ]
                            //     [ m2 ]
                            //     [ m3 ]
                            //     [ m4 ]
                            //     [ m5 ]
                            //     [ m6 ]
                            // b = [ x1a ]
                            //     [ y1a ]
                            //     [ ... ]
                            //
                            // given that M [ xib yib ]' = [ xia yia ]' where:
                            // M = [ m1, m2, m5 ]
                            //     [ m3, m4, m6 ]
                            // (so m1..4 are the rotation/scaling/skew components and m5..6 are the translation)
                            #[allow(non_snake_case)]
                            let A = na::MatrixMN::<_, na::Dynamic, na::U6>::from_fn(
                                inter.len() * 2,
                                |i, j| {
                                    match j {
                                        0 => if i % 2 == 0 { ctrb[i/2].0 } else { 0.              },
                                        1 => if i % 2 == 0 { ctrb[i/2].1 } else { 0.              },
                                        2 => if i % 2 == 0 { 0.          } else { ctrb[(i-1)/2].0 },
                                        3 => if i % 2 == 0 { 0.          } else { ctrb[(i-1)/2].1 },
                                        4 => if i % 2 == 0 { 1.          } else { 0.              },
                                        5 => if i % 2 == 0 { 0.          } else { 1.              },
                                        _ => unreachable!()
                                    }
                                });
                            let b = na::DVector::from_fn(
                                inter.len() * 2,
                                |i, _| {
                                    if i % 2 == 0 { ctra[i/2].0 } else { ctra[(i-1)/2].1 }
                                });
                            let x = A.pseudo_inverse(1e-7) * b;
                            let xform = na::MatrixMN::<_, na::U3, na::U3>::from_row_slice(
                                &[x[0], x[1], x[4],
                                  x[2], x[3], x[5],
                                  0.,   0.,   1.  ]);

                            // finally use the fitted transformation to get the end-effector location in the current frame
                            let ptb = xform * pta;
                            if     ptb[0] >= 0. && u32(ptb[0])? < img.width()
                                && ptb[1] >= 0. && u32(ptb[1])? < img.height() {
                                    // use a piecewise linear discount to draw the past few points
                                    // brightly, then dimming after that (but never dropping out)
                                    let discount = match (i32(fa)? - i32(fb)?).abs() {
                                            0 ...15 => 1.,
                                        d @ 15...75 => -0.0125*f64(d) + 1.1875, // smooth grade from 1.0 to 0.25
                                        _           => 0.25
                                    };

                                    /// Blends a pixel in the given channels using a weighted average
                                    fn blend(img: &mut RgbaImage, x: i32, y: i32, blends: &[(usize, u8)], weight: f64, margin: i32) -> Result<()> {
                                        for xx in cmp::max(0, x - margin) .. cmp::min(i32(img.width())?, x + margin) {
                                            for yy in cmp::max(0, y - margin) .. cmp::min(i32(img.height())?, y + margin) {
                                                let px = img.get_pixel_mut(u32(xx)?, u32(yy)?).channels_mut();
                                                for &(ch, val) in blends {
                                                    px[ch] = u8(f64::min(255., (f64(val) * weight) + (f64(px[ch]) * (1. - weight))))?;
                                                }
                                            }
                                        }
                                        Ok(())
                                    }

                                    if let Some(prev) = prev {
                                        // draw an anti-aliased line from the previous point to this one
                                        for ((lx, ly), val) in Line::<f64, i32>::new((prev[0], prev[1]), (ptb[0], ptb[1])) {
                                            blend(&mut img, lx, ly, &[(0, 255)], val * discount, 2)?;
                                        }
                                    } else {
                                        // this is the first point, so there's no line to draw
                                        blend(&mut img, i32(ptb[0])?, i32(ptb[1])?, &[(0, 255)], discount, 2)?;
                                    }
                                    prev = Some(ptb); // save transformed point for line drawing in next iteration
                                }
                        }
                    }
                }

                // save frame in temp dir (will be processed by ffmpeg)
                img.save(&to).chain_err(|| Io("save", to.clone()))?;

                bar.inc(1);
                Ok(())
            })
            .collect::<Result<Vec<()>>>()?;
        bar.finish();

        // step 3: use ffmpeg to assemble frame images into video
        
        println!("\tRunning ffmpeg");
        let bar = nri::make_bar(u64(units.len()));
        bar.set_message("Encode");
        let mut child = Command::new("ffmpeg")
            .narg("-r", 15)                                    // frame rate = 15 FPS
            .narg("-start_number", i32(first_frame)?)          // start at first frame number
            .parg("-i", framedir.path().join("bluefox%d.png")) // filename pattern
            .parg("-pix_fmt", "yuv420p")                       // make an MP4
            .arg("-y")                                         // don't ask to overwrite
            .arg(matches.value_of("DEST").unwrap())            // output filename
            .stdout(Stdio::null())                             // ignore stdout
            .stderr(Stdio::piped())                            // capture stderr to draw progress bar
            .spawn().chain_err(|| Io("run", "ffmpeg".into()))?;
        for line in BufReader::new(child.stderr.as_mut().unwrap()).split('\r' as u8) {
            // parse stderr "frame=###" to draw progress bar
            let line = line.chain_err(|| Io("read output of", "ffmpeg".into()))?;
            let line = str::from_utf8(&line).chain_err(|| "non-UTF8 output from ffmpeg")?;
            if line.starts_with("frame=") {
                let f = line["frame=".len() .. line.find("fps").unwrap()].trim().parse().unwrap();
                bar.set_position(f);
            }
        }
        bar.finish();
        let code = child.wait().chain_err(|| Io("wait for", "ffmpeg".into()))?;
        if !code.success() {
            bail!("ffmpeg returned {}", code);
        }
    }

    Ok(0)
});

/// Extension trait adding some utility functions to std::process::Command
trait CommandExt {
    /// Pass a numeric argument on the command line
    fn narg(&mut self, s: &str, n: i32) -> &mut Self;
    /// Pass a path argument on the command line
    fn parg<P: AsRef<Path>>(&mut self, s: &str, p: P) -> &mut Self;
}

impl CommandExt for Command {
    fn narg(&mut self, s: &str, n: i32) -> &mut Self {
        self.arg(s)
            .arg(format!("{}", n))
    }

    fn parg<P: AsRef<Path>>(&mut self, s: &str, p: P) -> &mut Self {
        self.arg(s)
            .arg(p.as_ref())
    }
}

