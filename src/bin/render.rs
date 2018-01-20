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
extern crate thread_local_object;

extern crate nri;
extern crate utils;

use utils::prelude::*;

use std::{cmp, f64, fs, io, ops, str};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use cast::{u8, u32, u64, i32, usize, f64};
use image::{GenericImage, Pixel, RgbaImage};
use line_drawing::XiaolinWu as Line;
use rayon::prelude::*;
use tempdir::TempDir;
use thread_local_object::ThreadLocal;

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
use std::result::Result as StdResult;

trait AllowErrExt<T, E> {
    fn allow_err<F: FnOnce(&E) -> bool, G: FnOnce(E) -> T>(self, pred: F, gen: G) -> Self;
}

impl<T, E> AllowErrExt<T, E> for StdResult<T, E> {
    fn allow_err<F: FnOnce(&E) -> bool, G: FnOnce(E) -> T>(self, pred: F, gen: G) -> Self {
        match self {
            Ok(t) => Ok(t),
            Err(e) => if pred(&e) { Ok(gen(e)) } else { Err(e) },
        }
    }
}

trait ThruExt {
    fn thru(self, end: Self) -> Box<Iterator<Item=Self>>;
}

// TODO bring in num and make this T: PartialOrd + Add + One
impl ThruExt for i32 {
    fn thru(self, end: Self) -> Box<Iterator<Item=Self>> {
        if end > self {
            Box::new(self .. end+1)
        } else {
            Box::new((end .. self+1).rev())
        }
    }
}

enum Mode {
    Movie(Movie),
    Crops(Crops),
}

#[derive(Default)]
struct Movie {
    framedir: Option<TempDir>,
    output_filename: Option<PathBuf>,
    px: Option<f64>,
    py: Option<f64>,
    nframes: Option<usize>,
    first_frame: Option<u32>,
}

#[derive(Default)]
struct Crops {
    output_dir: Option<PathBuf>,
    pts: Option<ThreadLocal<Vec<(f64, f64)>>>,
    pcts: Option<Arc<Mutex<HashMap<u32, f64>>>>,
}

macro_rules! mode {
    (@scan (, $($rest:tt)*) -> $output:tt $thru:tt) => {
        mode!(@scan ($($rest)*) -> $output $thru)
    };
    (@scan (ref mut $field:ident $($rest:tt)*) -> ($($output:tt)*) $thru:tt) => {
        mode!(@scan ($($rest)*) -> ($($output)* (ref mut $field)) $thru)
    };
    (@scan (ref $field:ident $($rest:tt)*) -> ($($output:tt)*) $thru:tt) => {
        mode!(@scan ($($rest)*) -> ($($output)* ($field: Some(ref $field))) $thru)
    };
    (@scan ($field:ident $($rest:tt)*) -> ($($output:tt)*) $thru:tt) => {
        mode!(@scan ($($rest)*) -> ($($output)* ($field: Some($field))) $thru)
    };
    (@scan () -> ($(($($field:tt)*))*) [$mode:ident]) => {
        Mode::$mode($mode { $($($field)*,)* .. })
    };
    ($mode:ident { $($fields:tt)* }) => { mode!(@scan ($($fields)*) -> () [$mode]) };
}

impl Mode {
    fn parse(s: &str) -> Result<Mode> {
        match s {
            "movie" => Ok(Mode::Movie(Default::default())),
            "crops" => Ok(Mode::Crops(Default::default())),
            _ => bail!("unknown mode {}", s)
        }
    }

    fn init(&mut self, matches: &clap::ArgMatches) -> Result<()> {
        match *self {
            mode!(Movie { ref mut px, ref mut py }) => {
                if let Some(mut values) = matches.values_of("PT") {
                    *px = values.next().unwrap().parse().ok();
                    *py = values.next().unwrap().parse().ok();
                } else {
                    *px = Some(778.);
                    *py = Some(760.);
                }
                println!("Tracking ({}, {})", px.unwrap(), py.unwrap());
            }

            mode!(Crops {}) => {}
        }
        Ok(())
    }

    fn prepare_to_process(&mut self, epdir: &str, frames: &[(u32, String, f64)]) -> Result<()> {
        match *self {
            mode!(Movie { ref mut output_filename, ref mut framedir, ref mut first_frame, ref mut nframes }) => {
                *output_filename = Some(Path::new(epdir).join("movie.mp4"));

                // store frames in a temporary directory before running FFMPEG
                *framedir = Some(TempDir::new("nri").chain_err(|| Io("create", "temp dir".into()))?);

                *first_frame = Some(frames.iter().map(|&(num, _, _)| num).min().ok_or("no frames")?);
                *nframes = Some(frames.len());
            }

            mode!(Crops { ref mut output_dir, ref mut pts, ref mut pcts }) => {
                // clear out crop dir
                let cropdir = Path::new(epdir).join("crops");
                fs::remove_dir_all(&cropdir).allow_err(|e| e.kind() == io::ErrorKind::NotFound, |_| ())
                                            .chain_err(|| Io("remove", cropdir.clone()))?;
                fs::create_dir(&cropdir).chain_err(|| Io("create", cropdir.clone()))?;

                *output_dir = Some(cropdir);

                *pts = Some(ThreadLocal::new());
                *pcts = Some(Arc::new(Mutex::new(HashMap::new())));
            }
        }
        Ok(())
    }

    fn process(&self, img: &mut RgbaImage, fa: u32, fb: u32, xform: na::MatrixMN<f64, na::U3, na::U3>) -> Result<()> {
        match *self {
            mode!(Movie { ref px, ref py }) => {
                // finally use the fitted transformation to get the end-effector location in the current frame
                
                if fb > fa { return Ok(()) } // only frames up to the current one

                let pta = na::VectorN::<_, na::U3>::from_row_slice(&[*px, *py, 1.]); // empirical end-effector location
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

                    thread_local! { static PREV: RefCell<Option<na::VectorN<f64, na::U3>>> = RefCell::new(None); }
                    PREV.with(|prev| -> Result<_> {
                        let mut prev = prev.borrow_mut();
                        if let Some(ref prev) = *prev {
                            // draw an anti-aliased line from the previous point to this one
                            for ((lx, ly), val) in Line::<f64, i32>::new((prev[0], prev[1]), (ptb[0], ptb[1])) {
                                blend(img, lx, ly, &[(0, 255)], val * discount, 2)?;
                            }
                        } else {
                            // this is the first point, so there's no line to draw
                            blend(img, i32(ptb[0])?, i32(ptb[1])?, &[(0, 255)], discount, 10)?;
                        }
                        if fa == fb {
                            blend(img, i32(ptb[0])?, i32(ptb[1])?, &[(0, 255)], discount, 10)?;
                            *prev = None; // clear out for next frame
                        } else {
                            *prev = Some(ptb); // save transformed point for line drawing in next iteration
                        }
                        Ok(())
                    })?;
                }
            }

            mode!(Crops { ref pts }) => {
                let pta = na::VectorN::<_, na::U3>::from_row_slice(&[778., 760., 1.]); // empirical end-effector location
                let ptb = xform * pta;

                pts.entry(|pts| pts.or_insert(vec![])
                                   .push((ptb[0], ptb[1])));
            }

            _ => unreachable!()
        }
        Ok(())
    }

    fn end_frame(&self, fa: u32, img: &mut RgbaImage, filename: &str) -> Result<()> {
        match *self {
            mode!(Movie { ref framedir }) => {
                let mut to = framedir.path().to_owned();
                to.push(filename);

                // save frame in temp dir (will be processed by ffmpeg)
                img.save(&to).chain_err(|| Io("save", to.clone()))?;
            }

            mode!(Crops { ref output_dir, ref pts, ref pcts }) => {
                if let Some(pts) = pts.remove() {
                    const L: usize = 0;
                    const B: usize = 1;
                    const R: usize = 2;
                    const T: usize = 3;
                    let rect = [655, 1200, 928, 704]; // empirical end-effector envelope
                    //          L    B     R     T

                    let bboxb = pts.into_iter()
                                   .fold([f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::INFINITY],
                                         |bb, (x, y)| {
                                             [f64::min(bb[L], x),
                                              f64::max(bb[B], y),
                                              f64::max(bb[R], x),
                                              f64::min(bb[T], y)]
                                         });
                    let bboxb = [cmp::max(i32(bboxb[L] - 25.)?, 1),
                                 cmp::min(i32(bboxb[B] + 25.)?, i32(img.height())?),
                                 cmp::min(i32(bboxb[R] + 25.)?, i32(img.width())?),
                                 cmp::max(i32(bboxb[T] - 25.)?, 1)];

                    if bboxb[B] > bboxb[T] && bboxb[R] > bboxb[L] {
                        // calculate intersection percentage of rect vs bboxb
                        let bboxb_area = (bboxb[R] - bboxb[L]) * (bboxb[B] - bboxb[T]);
                        let overlap_area = cmp::max(0, cmp::min(rect[R], bboxb[R]) - cmp::max(rect[L], bboxb[L])) *
                                           cmp::max(0, cmp::min(rect[B], bboxb[B]) - cmp::max(rect[T], bboxb[T]));
                        let pct = f64(overlap_area) / f64(bboxb_area);

                        if pct < 0.25 {
                            let cropped = output_dir.join(format!("{}_crop.png", fa));
                            img.sub_image(u32(bboxb[L])?, u32(bboxb[T])?, u32(bboxb[R] - bboxb[L])?, u32(bboxb[B] - bboxb[T])?)
                               .to_image()
                               .save(&cropped).chain_err(|| Io("save", cropped.clone()))?;

                            let boxed = output_dir.join(format!("{}_frame.png", fa));
                            for line in [(L, T), (R, T), (R, B), (L, B), (L, T)].windows(2) {
                                for lx in bboxb[line[0].0].thru(bboxb[line[1].0]) {
                                    for ly in bboxb[line[0].1].thru(bboxb[line[1].1]) {
                                        blend(img, lx, ly, &[(0, 255)], 1., 2)?;
                                    }
                                }
                            }
                            img.save(&boxed).chain_err(|| Io("save", boxed.clone()))?;

                            pcts.lock().unwrap().insert(fa, pct);
                        }
                    }
                }
            }

            _ => unreachable!()
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        match *self {
            mode!(Movie { ref framedir, nframes, first_frame, ref output_filename }) => {
                // step 3: use ffmpeg to assemble frame images into video
                
                println!("\tRunning ffmpeg");
                let bar = nri::make_bar(u64(nframes));
                bar.set_message("Encode");
                let mut child = Command::new("ffmpeg")
                    .narg("-r", 15)                                    // frame rate = 15 FPS
                    .narg("-start_number", i32(first_frame)?)          // start at first frame number
                    .parg("-i", framedir.path().join("bluefox%d.png")) // filename pattern
                    .parg("-pix_fmt", "yuv420p")                       // make an MP4
                    .arg("-y")                                         // don't ask to overwrite
                    .arg(output_filename)                              // output filename
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
                let code = child.wait().chain_err(|| Io("wait for", "ffmpeg.1".into()))?;
                if !code.success() {
                    bail!("ffmpeg.1 returned {}", code);
                }

                // add audio if exists
                let audiopath = output_filename.with_file_name("acc.wav");
                if audiopath.exists() {
                    let moved_output = framedir.path().join("movie.mp4");
                    fs::copy(output_filename, &moved_output).chain_err(|| Io("move", output_filename.into()))?;
                    let mut child = Command::new("ffmpeg")
                        .parg("-i", moved_output)    // input video
                        .parg("-i", audiopath)       // input audio
                        .parg("-c:v", "copy").parg("-c:a", "aac").narg("-strict", -2) // stream modes
                        .arg("-y")                   // don't ask to overwrite
                        .arg(output_filename)        // output filename
                        .stdout(Stdio::null())       // ignore stdout
                        .stderr(Stdio::null())       // ignore stderr
                        .spawn().chain_err(|| Io("run", "ffmpeg".into()))?;
                    let code = child.wait().chain_err(|| Io("wait for", "ffmpeg.2".into()))?;
                    if !code.success() {
                        bail!("ffmpeg.2 returned {}", code);
                    }
                }
            }

            mode!(Crops { ref output_dir, ref mut pcts }) => {
                let pcts = Arc::try_unwrap(pcts.take().unwrap()).unwrap().into_inner().unwrap(); // such unwrap
                let mut sorted = pcts.iter().collect::<Vec<_>>();
                sorted.sort_by(|&(_, pct1), &(_, pct2)| pct1.partial_cmp(&pct2).expect("NaN"));
                let mut keepers = sorted.iter().map(|&(fa, _)| fa).collect::<Vec<_>>();
                for i in 0..5 {
                    if i >= keepers.len() { break }

                    // delete everything within 1 second
                    let cur = *keepers[i];
                    keepers.retain(|&&fa| fa == cur || fa.absdiff(cur) > 15);
                }
                keepers.truncate(5);

                for (fa, _) in sorted {
                    if !keepers.contains(&fa) {
                        let frame_file = output_dir.join(format!("{}_frame.png", fa));
                        let crop_file = output_dir.join(format!("{}_crop.png", fa));
                        fs::remove_file(&frame_file).chain_err(|| Io("delete", frame_file.clone()))?;
                        fs::remove_file(&crop_file).chain_err(|| Io("delete", crop_file.clone()))?;
                    }
                }
            }

            _ => unreachable!()
        }
        Ok(())
    }
}


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

quick_main!(|| -> Result<i32> {
    let matches = clap_app! { nri_render =>
        (version: crate_version!())
        (author: crate_authors!("\n"))
        (about: "Helper for rendering stuff onto images/movies")

        (@arg EPDIR: *... "Episode directory")
        (@arg MODE: -m --mode <MODE>... {|s| Mode::parse(&s)} "Render mode")
        (@arg PT: -p [coords] #{2,2} {|s| s.parse::<f64>()} "Tracked point")
    }.get_matches();

    let mut modes = matches.values_of("MODE").unwrap()
                           .map(|s| Mode::parse(s).unwrap())
                           .collect::<Vec<_>>();

    for mode in &mut modes { mode.init(&matches)?; }

    for epdir in matches.values_of("EPDIR").unwrap() {
        println!("Processing {}...", epdir);

        // step 1: read bluefox data from CSV files
        
        let mut bt = csv::Reader::from_path([epdir, "bluefox", "bluefox_times.csv"].iter().collect::<PathBuf>())?;
        let mut april = csv::Reader::from_path([epdir, "bluefox", "april.csv"].iter().collect::<PathBuf>())?;

        // format of bluefox_times.csv is "Frame number (int), Filename (str), Unix Timestamp (float)"
        let frames = bt.deserialize()
                       .map(|r| r.map_err(Into::into))
                       .collect::<Result<Vec<(u32, String, f64)>>>()?;
        // format of april.csv is "Frame number (int), Tag IDs (ints, semicolon-sep), Tag centers (float comma-sep coord pairs, semicolon-sep), ..."
        let aprils = april.deserialize()
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
        

        // step 2: do mode-specific stuff
        
        let bar = nri::make_bar(u64(frames.len()));
        bar.set_message("Process");

        // process frames using all available CPUs
        for mode in &mut modes { mode.prepare_to_process(&epdir, &frames)?; }
        frames.into_par_iter()
            .map(|(fa, filename, _stamp)| {
                // step 2a: load camera frame from file
                
                let mut from = Path::new(epdir).to_owned();
                from.push("bluefox");
                from.push(&filename);

                let mut img = image::open(from)?.to_rgba();

                // step 2b: plot transformed end-effector location from all frames up to now

                // everything labeled "a" is the frame we're drawing on, while "b" is the frame being transformed from
                let apra = &aprils[&fa];
                let ida = apra.keys().collect::<HashSet<_>>(); // tag IDs visible in the current frame
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

                            for mode in &modes { mode.process(&mut img, fa, fb, xform)?; }
                        }
                    }
                }

                for mode in &modes { mode.end_frame(fa, &mut img, &filename)?; }

                bar.inc(1);
                Ok(())
            })
            .collect::<Result<Vec<()>>>()?;
        bar.finish();

        for mode in &mut modes { mode.finish()?; }
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

