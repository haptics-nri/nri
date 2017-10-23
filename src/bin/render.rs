#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate csv;
extern crate image;
extern crate indicatif;
extern crate line_drawing;
extern crate nalgebra as na;
extern crate rayon;
extern crate tempdir;

extern crate nri;

use std::cmp;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{GenericImage, ImageFormat, Pixel, RgbaImage};
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

        let framedir = TempDir::new("nri").chain_err(|| Io("create", "temp dir".into()))?;
        let mut bt = csv::Reader::from_file([epdir, "bluefox", "bluefox_times.csv"].iter().collect::<PathBuf>())?;
        let mut april = csv::Reader::from_file([epdir, "bluefox", "april.csv"].iter().collect::<PathBuf>())?;

        let frames = bt.decode()
                       .map(|r| r.map_err(Into::into))
                       .collect::<Result<Vec<(u32, String, f64)>>>()?;
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
        let first_frame = frames.iter().map(|&(num, _, _)| num).min().ok_or("no frames")?;
        let bar = nri::make_bar(frames.len() as u64);
        bar.set_message("Frames");

        frames.into_par_iter()
              .map(|(fa, filename, _stamp)| {
                  let mut from = Path::new(epdir).to_owned();
                  from.push("bluefox");
                  from.push(&filename);
                  let mut to = framedir.path().to_owned();
                  to.push(&filename);

                  let mut img = image::open(from)?.to_rgba();

                  let apra = &aprils[&fa];
                  let ida = apra.keys().collect::<HashSet<_>>();
                  let pta = na::VectorN::<_, na::U3>::from_row_slice(&[778., 760., 1.]);
                  let mut prev: Option<na::VectorN<f64, na::U3>> = None;
                  for (&fb, aprb) in &aprils {
                      if aprb.len() > 20 {
                          let idb = aprb.keys().collect::<HashSet<_>>();
                          let inter = ida.intersection(&idb).collect::<Vec<_>>();
                          if inter.len() >= 4 {
                              let ctra = inter.iter().map(|id| apra[id]).collect::<Vec<_>>();
                              let ctrb = inter.iter().map(|id| aprb[id]).collect::<Vec<_>>();

                              // find similarity
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
                              
                              let ptb = xform * pta;
                              if     ptb[0] >= 0. && (ptb[0] as u32) < img.width()
                                  && ptb[1] >= 0. && (ptb[1] as u32) < img.height() {
                                  let discount = match (fa as i32 - fb as i32).abs() {
                                          0 ...15 => 1.,
                                      d @ 15...75 => -0.0125*(d as f64) + 1.1875,
                                      _           => 0.25
                                  };

                                  fn blend(img: &mut RgbaImage, x: i32, y: i32, blends: &[(usize, u8)], weight: f64) {
                                      let px = img.get_pixel_mut(x as u32, y as u32).channels_mut();
                                      for &(ch, val) in blends {
                                          px[ch] = (((val as f64) * weight) + ((px[ch] as f64) * (1. - weight))) as u8;
                                      }
                                  }

                                  if let Some(prev) = prev {
                                      for ((lx, ly), val) in Line::<f64, i32>::new((prev[0], prev[1]), (ptb[0], ptb[1])) {
                                          for x in cmp::max(0, lx - 2) .. cmp::min(img.width() as i32, lx + 2) {
                                              for y in cmp::max(0, ly - 2) .. cmp::min(img.height() as i32, ly + 2) {
                                                  blend(&mut img, x, y, &[(0, 255)], val * discount);
                                              }
                                          }
                                      }
                                  } else {
                                      for x in cmp::max(0, ptb[0] as i32 - 2) .. cmp::min(img.width() as i32, ptb[0] as i32 + 2) {
                                          for y in cmp::max(0, ptb[1] as i32 - 2) .. cmp::min(img.height() as i32, ptb[1] as i32 + 2) {
                                              blend(&mut img, x, y, &[(0, 255)], discount);
                                          }
                                      }
                                  }
                                  prev = Some(ptb);
                              }
                          }
                      }
                  }

                  img.save(&to).chain_err(|| Io("save", to.clone()))?;

                  bar.inc(1);
                  Ok(())
              })
            .collect::<Result<Vec<()>>>()?;

        println!("\tRunning ffmpeg");
        let code = Command::new("ffmpeg")
            .narg("-r", 15)
            .narg("-start_number", first_frame as i32)
            .parg("-i", framedir.path().join("bluefox%d.png"))
            .parg("-pix_fmt", "yuv420p")
            .arg("-y")
            .arg(matches.value_of("DEST").unwrap())
            .status().chain_err(|| Io("run", "ffmpeg".into()))?;
        if !code.success() {
            bail!("ffmpeg returned {}", code);
        }
    }

    Ok(0)
});

trait CommandExt {
    fn narg(&mut self, s: &str, n: i32) -> &mut Self;
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

