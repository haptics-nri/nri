use std::{env, process, mem, ptr};
use std::io::{Read, Write};
use std::fs::File;
use std::fmt::Debug;

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

pub fn read_binary<Data: Debug>(header: &str) {
    let mut args = env::args();
    let (inname, outname) = 
        if args.len() != 3 {
            errorln!("Failed to parse command line arguments.");
            errorln!("Usage: {} [binary input file] [csv output file]", args.next().unwrap());
            process::exit(1)
        } else {
            let _ = args.next().unwrap();
            (args.next().unwrap(), args.next().unwrap())
        };
    println!("in = {}, out = {}", inname, outname);
    println!("packet size {}", mem::size_of::<Data>());

    // open files
    let mut infile = attempt!(File::open(inname));
    let mut outfile = attempt!(File::create(outname));

    // write CSV header
    attempt!(writeln!(outfile, "{}", header));

    // read file
    let mut vec = vec![0u8; 0];
    attempt!(infile.read_to_end(&mut vec));

    let mut data: Data = unsafe { mem::uninitialized() };
    let mut i = 0;
    for chunk in vec.chunks(mem::size_of_val(&data)) {
        i += 1;
        unsafe {
            ptr::copy(chunk.as_ptr(), &mut data as *mut _ as *mut _, mem::size_of_val(&data));
        }
        attempt!(writeln!(outfile, "{:?}", data));
    }
    println!("translated {} packets", i);
}


