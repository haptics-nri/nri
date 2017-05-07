#[macro_use] extern crate lazy_static;
extern crate time;
extern crate notify;

use time::Duration;
use notify::{Watcher, RecommendedWatcher};
use std::{env, fs, io};
use std::path::{Path, PathBuf};
use std::ops::Deref;
use std::sync::{mpsc, RwLock, Mutex};
use std::thread;

pub mod config;

pub fn in_original_dir<F: FnOnce() -> R, R>(f: F) -> io::Result<R> {
    lazy_static! {
        static ref ORIGINAL_DIR: PathBuf = env::current_dir().unwrap();
        static ref CRITICAL_SECTION: Mutex<()> = Mutex::new(());
    }

    // ensure that two threads don't fight over the current dir
    let _guard = CRITICAL_SECTION.lock().unwrap();

    let before = env::current_dir()?;
    env::set_current_dir(&*ORIGINAL_DIR)?;
    let ret = f();
    env::set_current_dir(before)?;
    Ok(ret)
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

pub fn watch<T, U, F>(mut thing: T,
                  global: &'static U,
                  root: &'static Path,
                  ext: &'static str,
                  mut f: F) -> RwLock<T>
    where F: FnMut(&mut T, PathBuf) + Send + 'static,
          U: Deref<Target=RwLock<T>> + Send + Sync + 'static
{
    let update = |thingref: &mut T,
                  f: &mut F,
                  root: &'static Path,
                  ext: &'static str| {
        fs::read_dir(root).expect(&format!("could not read directory {:?}", root))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| match p.extension() { Some(x) if x == ext => true, _ => false })
            .map(|p| f(thingref, p))
            .count();
    };

    update(&mut thing, &mut f, root, ext);

    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut w: RecommendedWatcher = Watcher::new(tx).expect("failed to crate watcher");
        w.watch(root).expect("watcher refused to watch");

        for evt in rx {
            if let Some(path) = evt.path {
                if let Some(x) = path.extension() {
                    if x == ext {
                        print!("Updating... ({:?} {:?})", path.file_name().expect(&format!("could not get file name of {:?}", path)),
                                                          evt.op.expect("no operation for event"));
                        let mut thing = global.write().expect("couldn't get a write lock");
                        update(&mut *thing, &mut f, root, ext);
                        println!(" done.");
                    }
                }
            }
        }
    });

    RwLock::new(thing)
}

/// Groups a number of items under one conditional-compilation attribute
///
/// Examples:
///
/// ```rust
/// # #[macro_use] extern crate utils;
/// // this one compiles!
///
/// pub struct Foo;
///
/// group_attr! {
///     #[cfg(any(unix, not(unix)))] // always true
///
///     extern crate hprof;
///
///     pub struct Bar(
///         Foo,             // types from outside are accessible
///         hprof::Profiler, // extern crates are accessible
///     );
/// }
///
/// type Baz = Bar; // types from inside are accessible
/// # fn main() {}
/// ```
///
/// ```rust,ignore
/// # #[macro_use] extern crate utils;
/// // this one doesn't compile!
///
/// group_attr! {
///     #[cfg(all(unix, not(unix)))] // never true
///
///     pub struct Bar;
/// }
///
/// type Baz = Bar; // undefined
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! group_attr {
    (#[cfg($attr:meta)] $($yes:item)*) => {
          $(#[cfg($attr)] $yes)*
    }
}

pub mod prof {
    extern crate hprof;
    use std::cell::RefCell;

    pub use self::hprof::enter;

    thread_local! {
        // Thread-local profiler object (FIXME no doc comments on thread locals)
        pub static PROF: RefCell<Option<hprof::Profiler>> = RefCell::new(None)
    }

    #[macro_export]
    macro_rules! prof {
        ($b:expr) => { prof!(stringify!($b), $b) };
        //($n:expr, $b:expr) => ($b)
        ($n:expr, $b:expr) => {{
            $crate::prof::PROF.with(|wrapped_prof| {
                let appease_borrowck = wrapped_prof.borrow();
                let g = match *appease_borrowck {
                    Some(ref prof) => prof.enter($n),
                    None => $crate::prof::enter($n)
                };
                let ret = { $b }; //~ ALLOW let_unit_value
                drop(g);
                ret
            })
        }}
    }
}

pub use prof::PROF;

pub trait DurationExt {
    fn sleep(&self);
}

impl DurationExt for Duration {
    fn sleep(&self) {
        thread::sleep(self.to_std().unwrap());
    }
}

