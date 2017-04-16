#[macro_use] extern crate lazy_static;
extern crate chrono;
extern crate notify;

use chrono::Duration;
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

#[macro_export]
macro_rules! jsonize {
    ($map:ident, $selph:ident, $var:ident) => {{
        $map.insert(stringify!($var).to_owned(), $selph.$var.to_json())
    }};
    ($map:ident, $selph:ident; $($var:ident),+) => {{
        $(jsonize!($map, $selph, $var));+
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
/// This is done by creating an inner module from which all public items are re-exported (both
/// conditional on the given attribute). This should be mostly transparent -- `use super::*` is
/// automatically inserted so all public items of the parent module are available (should be all
/// items, but see [rust-lang/rust#23157][glob-pub]). The items may begin with extern crate statements, which
/// will be hoisted out and put in the parent module, plus imported into the inner module (in 1.6
/// this should be covered by the glob), though any extern crate statements after the first
/// non-extern-crate item will be missed.
///
/// By default the inner module is named `__internal`. Obviously this could conflict with a real
/// module, and will definitely conflict if this macro is used twice in the same module. For that
/// reason an explicit name for the inner module can be passed in. If this state of affairs annoys
/// you, please agitate at [rust-lang/rfcs#1266][gensym].
///
/// [glob-pub]: https://github.com/rust-lang/rust/issues/23157
/// [gensym]: https://github.com/rust-lang/rfcs/issues/1266
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
    // interior rule to hoist an unaliased extern crate
    (@inner $name:ident ($($krates:ident)*) #[cfg($attr:meta)] $(#[$krattr:meta])* extern crate $krate:ident; $($yes:tt)*) => {
        // hoist the extern crate statement
        #[cfg($attr)] $(#[$krattr])* extern crate $krate;

        // collect the crate name and continue processing
        group_attr!(@inner $name ($($krates)* $krate) #[cfg($attr)] $($yes)*);
    };

    // interior rule to hoist an aliased extern crate
    (@inner $name:ident ($($krates:ident)*) #[cfg($attr:meta)] $(#[$krattr:meta])* extern crate $krate:ident as $alias:ident; $($yes:tt)*) => {
        // hoist the extern crate statement
        #[cfg($attr)] $(#[$krattr])* extern crate $krate as $alias;

        // collect the alias (not the real name) and continue processing
        group_attr!(@inner $name ($($krates)* $alias) #[cfg($attr)] $($yes)*);
    };

    // interior rule for outputting items, after extern crates are done
    (@inner $name:ident ($($krates:ident)*) #[cfg($attr:meta)] $($yes:item)*) => {
        // a curious inner module
        #[cfg($attr)]
        mod $name {
            // glob import gets all pub items from enclosing module
            #[allow(unused_imports)] use super::*;

            // explicitly import all the collected extern crates
            #[allow(unused_imports)] use super::{$($krates),*};

            // output the rest of the items
            $($yes)*
        }
    
        // re-export everything that the inner module produced
        #[cfg($attr)]
        pub use $name::*;
    };

    // entry point for default inner module name
    (#[cfg($attr:meta)] $($yes:tt)*) => { group_attr!(__internal #[cfg($attr)] $($yes)*); };

    // entry point with a name given for the inner module
    ($name:ident #[cfg($attr:meta)] $($yes:tt)*) => { group_attr!(@inner $name () #[cfg($attr)] $($yes)*); };
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

