extern crate csv;
extern crate lodepng;
extern crate libc;

use std::{env, process, mem, ptr, thread};
use std::io::{Read, Write};
use std::fs::File;
use std::fmt::Debug;
use std::sync::mpsc;
use std::path::{Path, PathBuf};
use self::lodepng::{encode_file, ColorType};

/// Semiautomatically-indenting `println!` replacement
///
/// ¡Achtung! Requres `#[macro_use] extern crate lazy_static;` at the crate root!
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
    use std::sync::Mutex;
    lazy_static! {
        /// Tracks the current indent level
        pub static ref _INDENT: Mutex<usize> = Mutex::new(0);
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub struct IndentGuard { prev: usize }
    #[allow(dead_code)]
    impl IndentGuard {
        pub fn new() -> IndentGuard {
            IndentGuard { prev: *_INDENT.lock().unwrap() }
        }
    }
    impl Drop for IndentGuard {
        fn drop(&mut self) {
            *_INDENT.lock().unwrap() = self.prev;
        }
    }

    /// Indenting replacement for println!
    /// 
    /// Two syntaxes:
    ///     1. exactly the same as println!: prepends the current indentation and prints the stuff
    ///     2. like (1), but with '>' prepended: prints the stuff, then increases the indentation
    ///        level. The indentation will revert at the end of the scope.
    #[macro_export] macro_rules! indentln {
        (> $($arg:expr),*) => {
            indentln!($($arg),*);
            let _indent_guard = $crate::common::indent::IndentGuard::new();
            *$crate::common::indent::_INDENT.lock().unwrap() += 1;
        };
        ($($arg:expr),*)   => {
            println!("{s: <#w$}{a}", s = "", w = 4 * *$crate::common::indent::_INDENT.lock().unwrap(), a = format!($($arg),*));
        }
    }
}

/// Just like println!, but prints to stderr
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

pub fn do_binary<Data: Debug>(header: &str, (inname, outname): (String, Option<String>)) -> Vec<Data> {
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

    let mut i = 0;
    let datums = vec
        .chunks(mem::size_of::<Data>())
        .map(|chunk: &[u8]| {
            i += 1;
            let mut data: Data = unsafe { mem::uninitialized() };
            unsafe {
                ptr::copy(chunk.as_ptr(), &mut data as *mut _ as *mut _, mem::size_of_val(&data));
            }
            outfile.as_mut().map(|ref mut f| attempt!(writeln!(f, "{:?}", data)));
            data
        })
        .collect();

    indentln!("translated {} packets", i);
    datums
}

pub fn read_binary<Data: Debug>(header: &str) -> Vec<Data> {
    let (inname, outname) = parse_inout_args(&mut env::args());
    do_binary::<Data>(header, (inname, Some(outname)))
}

pub trait Pixels<T> {
    fn pixel(&self, i: usize) -> T;
}

pub fn do_camera<T, Data: Debug + Pixels<T>>(width: usize, height: usize, channels: usize, color: ColorType, depth: libc::c_uint) {
    let inname = parse_in_arg(&mut env::args().skip(1));

    let mut csvfile = attempt!(File::open(&inname));
    let mut csvrdr = csv::Reader::from_reader(csvfile).has_headers(false);
    let mut csvwtr = csv::Writer::from_memory();
    csvwtr.encode(("Frame number", "Filename", "Unix timestamp"));

    const N_THREADS: usize = 4;
    print!("Creating {} threads...", N_THREADS);
    let mut threads = Vec::with_capacity(N_THREADS);
    unsafe { threads.set_len(N_THREADS); }
    for i in 0..N_THREADS {
        print!("{}...", i);
        let (tx, rx) = mpsc::channel::<PathBuf>();
        threads[i] = Some((
            thread::spawn(move || {
                for dat_path in rx {
                    let dat = dat_path.to_str().unwrap().to_string();
                    let png = dat_path.with_extension("png").to_str().unwrap().to_string();
                    let rows = do_binary::<Data>("", (dat, None));
                    let mut pixels = Vec::with_capacity(height*channels*rows.len());
                    for i in 0..rows.len() {
                        for j in 0..width {
                            pixels.push(rows[i].pixel(j));
                        }
                    }
                    attempt!(encode_file(png, &pixels, width, rows.len(), color, depth));
                }
            }),
            tx
        ));
    }
    println!("done!");

    let mut i = 0;
    let mut t = 0;
    for row in csvrdr.decode() {
        println!("reading frame {}...", i);
        let (num, fname, stamp): (usize, String, f64) = row.ok().expect(&format!("failed to parse row {} of {}", i, inname));
        csvwtr.encode((num, Path::new(&fname).with_extension("png").to_str().unwrap().to_string(), stamp));
        i += 1;
        let dat_path = Path::new(&inname).with_file_name(fname);
        threads[t].as_ref().unwrap().1.send(dat_path);
        t = (t + 1) % 4;
    }
    println!("finished {} frames", i);

    for t in 0..N_THREADS {
        let present = threads[t].take().unwrap(); // unwrap the present
        drop(present.1); // drop Sender causing the thread to stop looping
        attempt!(present.0.join()); // now safe to join the thread
    }

    attempt!(File::create(&inname)).write_all(csvwtr.as_bytes());
}


