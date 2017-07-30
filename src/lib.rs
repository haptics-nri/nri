extern crate csv;
extern crate lodepng;
extern crate libc;
extern crate hprof;
extern crate num_cpus;
extern crate indicatif;
#[macro_use] extern crate closet;

use std::{env, fs, process, mem, ptr, thread};
use std::io::{self, Read, Write};
use std::fs::File;
use std::fmt::Debug;
use std::sync::{mpsc, Arc};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
use lodepng::{encode_file, ColorType};
use hprof::Profiler;
pub use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

/// Single, multiple or no progress bar(s)
pub enum Bar {
    Multi(&'static str, Arc<MultiProgress>),
    Single,
    None
}

/// Verbosity
///
/// Messages indented more levels than this will be suppressed
pub static VERBOSITY: AtomicUsize = ATOMIC_USIZE_INIT;

/// Semiautomatically-indenting `println!` replacement
///
/// Example:
/// 
/// ```
/// indentln!("outer"); // no indentation
/// for i in 0..10 {
///     indentln!(> "inner"); // no indentation
///     indentln!("{}", i*i); // indented one level
///     // scope ends, indentation automatically restored here
/// }
/// ```
#[macro_use] pub mod indent {
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    /// Tracks the current indent level
    pub static _INDENT: AtomicUsize = ATOMIC_USIZE_INIT;

    #[doc(hidden)]
    #[allow(dead_code)]
    pub struct IndentGuard { prev: usize }
    #[allow(dead_code)]
    impl IndentGuard {
        pub fn new() -> IndentGuard {
            IndentGuard { prev: _INDENT.load(Ordering::SeqCst) }
        }
    }
    impl Drop for IndentGuard {
        fn drop(&mut self) {
            _INDENT.store(self.prev, Ordering::SeqCst);
        }
    }

    /// Indenting replacement for println!
    /// 
    /// Two syntaxes:
    ///     1. exactly the same as println!: prepends the current indentation and prints the stuff
    ///     2. like (1), but with '>' prepended: prints the stuff, then increases the indentation
    ///        level. The indentation will revert at the end of the scope.
    #[macro_export]
    macro_rules! indentln {
        () => {};
        (> $($arg:expr),*) => {
            indentln!($($arg),*);
            let _indent_guard = $crate::indent::IndentGuard::new();
            $crate::indent::_INDENT.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
        };
        ($($arg:expr),*)   => {{
            let indent = $crate::indent::_INDENT.load(::std::sync::atomic::Ordering::SeqCst);
            let verbosity = $crate::VERBOSITY.load(::std::sync::atomic::Ordering::SeqCst);
            if indent < verbosity {
                println!("{s: <#w$}{a}", s = "", w = 4 * indent, a = format!($($arg),*));
            }
        }}
    }
}

/// Just like println!, but prints to stderr
#[macro_export]
macro_rules! errorln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    }}
}

/// Just like try!, but kills the process on Err
#[macro_export]
macro_rules! attempt {
    ($e:expr) => {
        /*match $e {
            Ok(val) => val,
            Err(err) => {
                errorln!("Error: {:?}", err);
                process::exit(1);
            }
        }*/
        $e.unwrap()
    }
}

trait Leakable {
    type Target: ?Sized;

    fn leak(self) -> &'static Self::Target;
}

impl Leakable for String {
    type Target = str;

    fn leak(self) -> &'static str {
        unsafe {
            let s = mem::transmute::<&[u8], &'static [u8]>(self.as_bytes());
            mem::forget(self);
            str::from_utf8_unchecked(s)
        }
    }
}

pub fn parse_inout_args<I>(args: &mut I) -> (String, String)
        where I: ExactSizeIterator<Item=String>
{
    if args.len() != 2 && args.len() != 3 {
        errorln!("Failed to parse command line arguments.");
        errorln!("Usage: {} <binary input file> [<csv output file>]", args.next().unwrap());
        process::exit(1);
    }
    else {
        args.next().unwrap();
    }
    let inname = parse_in_arg(args);
    let outname = if args.peekable().peek().is_some() {
        parse_out_arg(args)
    } else {
        Path::new(&inname).with_extension("csv").to_str().unwrap().to_owned()
    };
    indentln!("in = {}, out = {}", inname, outname);
    (inname, outname)
}

pub fn parse_in_arg<I>(args: &mut I) -> String
        where I: Iterator<Item=String>
{
    args.next().unwrap()
}

pub fn parse_out_arg<I>(args: &mut I) -> String
        where I: Iterator<Item=String>
{
    args.next().unwrap()
}

pub fn do_binary<Data: Debug>(header: &str, bars: Bar, (inname, outname): (String, Option<String>)) -> Vec<Data> {
    indentln!("packet size {}", mem::size_of::<Data>());

    // open files
    let mut infile = attempt!(File::open(inname));
    let mut outfile = outname.map(|n| attempt!(File::create(n)));

    // write CSV header
    outfile.as_mut().map(|ref mut f| attempt!(writeln!(f, "{}", header)));

    // read file
    let mut vec = vec![0u8; 0];
    attempt!(infile.read_to_end(&mut vec));
    indentln!("file size = {} ({} packets)", vec.len(), vec.len() as f64 / mem::size_of::<Data>() as f64);

    let chunks = vec.chunks(mem::size_of::<Data>());
    let bar = make_bar(chunks.len() as u64);
    let bar = match bars {
        Bar::Multi(label, ref bars) => { let bar = bars.add(bar); bar.set_message(label); Some(bar) },
        Bar::Single => Some(bar),
        Bar::None => None
    };
    let mut i = 0;
    let datums = chunks.map(|chunk: &[u8]| {
        i += 1;
        if let Some(ref bar) = bar { if i % 100 == 0 { bar.inc(100); } }
        let mut data: Data = unsafe { mem::uninitialized() };
        unsafe {
            ptr::copy(chunk.as_ptr(), &mut data as *mut _ as *mut _, mem::size_of_val(&data));
        }
        outfile.as_mut().map(|ref mut f| attempt!(writeln!(f, "{:?}", data)));
        data
    }).collect::<Vec<_>>();

    match bars {
        Bar::Multi(..) => bar.unwrap().finish(),
        Bar::Single => bar.unwrap().finish_and_clear(),
        Bar::None => {}
    }
    indentln!("translated {} packets", datums.len());
    datums
}

pub fn read_binary<Data: Debug>(header: &str) -> Vec<Data> {
    let (inname, outname) = parse_inout_args(&mut env::args());
    do_binary::<Data>(header, Bar::Single, (inname, Some(outname)))
}

pub trait Pixels<T> {
    fn pixel(&self, i: usize) -> T;
}

pub fn make_bar(len: u64) -> ProgressBar {
    let bar = ProgressBar::new(len);
    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}/{eta_precise}] {bar:80.cyan/blue} {pos:>7}/{len:7} {msg}").progress_chars("##-"));
    bar
}

pub fn make_bar_bytes(len: u64) -> ProgressBar {
    let bar = ProgressBar::new(len);
    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}/{eta_precise}] {bar:80.cyan/blue} {bytes:>7}/{total_bytes:7} {msg}").progress_chars("##-"));
    bar
}

pub fn do_camera<T: Copy, Data: Debug + Pixels<T>, F: for<'a> Fn(String, C, &'a Profiler) + Send + Sync + 'static, C: Clone + Send + 'static>(name: &str, func: F, param: C, width: usize, height: usize, channels: usize, color: ColorType, depth: libc::c_uint) -> String {
    let func = Arc::new(func);
    let inname = parse_in_arg(&mut env::args().skip(1));
    attempt!(fs::create_dir_all(Path::new(&inname).parent().unwrap().join(name)));

    let csvfile = attempt!(File::open(&inname));
    let mut csvrdr = csv::Reader::from_reader(csvfile).has_headers(false);
    let mut csvwtr = csv::Writer::from_memory();
    attempt!(csvwtr.encode(("Frame number", "Filename", "Unix timestamp")));

    let records = csvrdr.decode().collect::<Vec<_>>();

    let num_threads = num_cpus::get();
    print!("Creating {} threads...", num_threads);
    let mut threads = vec![];
    let bar = Arc::new(make_bar(records.len() as u64));
    for i in 0..num_threads {
        print!("{}...", i);
        let (tx, rx) = mpsc::channel::<PathBuf>();
        let name = String::from(name);
        let (func, param) = (func.clone(), param.clone());
        threads.push(Some((
            thread::spawn(clone_army!([bar] move || {
                let prof = Profiler::new(format!("thread #{}", i).leak());
                for dat_path in rx {
                    bar.inc(1);
                    let _g = prof.enter("frame");
                    let dat = dat_path.to_str().unwrap().to_string();
                    let png = dat_path.parent().unwrap().join(&name).join(dat_path.file_name().unwrap()).with_extension("png").to_str().unwrap().to_string();
                    let rows = {
                        let _g = prof.enter("do_binary");
                        do_binary::<Data>("", Bar::None, (dat, None))
                    };
                    let pixels = {
                        let _g = prof.enter("into vec");
                        let mut pixels = Vec::with_capacity(height*channels*rows.len());
                        for row in &rows {
                            for j in 0..width {
                                pixels.push(row.pixel(j));
                            }
                        }
                        pixels
                    };
                    {
                        let _g = prof.enter("encode PNG");
                        attempt!(encode_file(&png, &pixels, width, rows.len(), color, depth));
                    }
                    {
                        let _g = prof.enter("callback");
                        func(png, param.clone(), &prof);
                    }
                }
                {
                    let s = io::stdout();
                    let _g = s.lock();
                    //prof.print_timing();
                }
            })),
            tx
        )));
    }
    println!("done!");

    let mut i = 0;
    let mut t = 0;
    for row in records {
        let (num, fname, stamp): (usize, String, f64) = row.expect(&format!("failed to parse row {} of {}", i, inname));
        attempt!(csvwtr.encode((num, Path::new(&fname).with_extension("png").to_str().unwrap().to_string(), stamp)));
        i += 1;
        let dat_path = Path::new(&inname).with_file_name(fname);
        attempt!(threads[t].as_ref().unwrap().1.send(dat_path));
        t = (t + 1) % 4;
    }

    for mut thread in threads {
        let present = thread.take().unwrap(); // unwrap the present
        drop(present.1); // drop Sender causing the thread to stop looping
        attempt!(present.0.join()); // now safe to join the thread
    }
    bar.finish_and_clear();

    attempt!(attempt!(File::create(Path::new(&inname).parent().unwrap().join(name).join(Path::new(&inname).file_name().unwrap()))).write_all(csvwtr.as_bytes()));

    inname
}

