use std::{env, process, mem, ptr};
use std::io::{Read, Write};
use std::fs::File;
use std::fmt::Debug;

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
        match $e {
            Ok(val) => val,
            Err(err) => {
                errorln!("Error: {:?}", err);
                process::exit(1);
            }
        }
    }
}

pub fn parse_inout_args<I>(args: &mut I) -> (String, String)
        where I: ExactSizeIterator<Item=String>
{
    if args.len() != 3 {
        errorln!("Failed to parse command line arguments.");
        errorln!("Usage: {} [binary input file] [csv output file]", args.next().unwrap());
        process::exit(1);
    }
    else {
        args.next().unwrap();
    }
    let (inname, outname) = (parse_in_arg(args), parse_out_arg(args));
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


