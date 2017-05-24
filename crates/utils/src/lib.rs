#[macro_use] extern crate lazy_static;
extern crate time;
extern crate notify;

use time::Duration;
use std::{env, mem, slice, thread, ptr};
use std::io::{self, Read};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::ops::{self, Deref, Add};
use std::sync::{mpsc, RwLock, Mutex};

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

pub fn slurp<P: AsRef<Path>>(p: P) -> io::Result<String> {
    let mut data = String::new();
    File::open(p.as_ref())?.read_to_string(&mut data)?;
    Ok(data)
}

pub fn circular_push<T>(vec: &mut Vec<T>, item: T) {
    if vec.len() == vec.capacity() {
        let len = vec.len()-1;
        unsafe {
            ptr::copy(&vec[1], &mut vec[0], len);
        }
        vec.truncate(len);
    }
    vec.push(item);
}

/// Retry some action on failure
pub fn retry<R, F: FnMut() -> Option<R>, G: FnOnce() -> R>(label: Option<&str>, times: usize, delay: Duration, mut action: F, fallback: G) -> R {
    for i in 0..times {
        match action() {
            Some(ret) => return ret,
            None =>
                if i == times-1 {
                    if let Some(label) = label {
                        println!("ERROR: {} failed {} times :(", label, times);
                    }
                    return fallback()
                } else {
                    if let Some(label) = label {
                        println!("\tRetrying (#{}/{}) {}", i+1, times, label);
                    }
                    delay.sleep();
                }
        }
    }
    unreachable!()
}

/// StepBy iterator
pub struct StepBy<T> {
    range: ops::Range<T>,
    step: T
}

impl<T> Iterator for StepBy<T> where T: PartialOrd, for<'a> &'a T: Add<Output=T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.range.start < self.range.end {
            let new = &self.range.start + &self.step;
            Some(mem::replace(&mut self.range.start, new))
        } else {
            None
        }
    }
}

/// create a StepBy iterator
pub fn step<T>(range: ops::Range<T>, step: T) -> StepBy<T> {
    StepBy { range, step }
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
    use notify::{Watcher, watcher};
    use notify::RecursiveMode::*;
    use notify::DebouncedEvent::*;
    use std::time::Duration;

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
        let mut w = watcher(tx, Duration::from_millis(100)).expect("failed to crate watcher");
        w.watch(root, Recursive).expect("watcher refused to watch");

        for evt in rx {
            match evt {
                Create(ref path) | Write(ref path) | Remove(ref path) | Rename(_, ref path) => {
                    if let Some(x) = path.extension() {
                        if x == ext {
                            print!("Updating... ({:?})", evt);
                            let mut thing = global.write().expect("couldn't get a write lock");
                            update(&mut *thing, &mut f, root, ext);
                            println!(" done.");
                        }
                    }
                }
                _ => {}
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

/// Generate const and mut versions of an item
///
/// Takes a square-bracketed list of bindings of the form `$name:frag -> const_val/mut_val`
/// and an item, which can use the bindings
macro_rules! const_and_mut {
    ([$($dol:tt $var:tt : $frag:ident => $const_name:tt / $mut_name:tt),* $(,)*] $($code:tt)*) => {
        macro_rules! __const_and_mut_inner {
            ($($dol $var:$frag),*) => {
                $($code)*
            }
        }
        macro_rules! cm { (*$t:ty) => { *const $t }; (&$t:ty) => { &$t } }
        __const_and_mut_inner!($($const_name),*);
        macro_rules! cm { (*$t:ty) => { *mut $t }; (&$t:ty) => { &mut $t } }
        __const_and_mut_inner!($($mut_name),*);
    }
}

/// Marker trait for "plain old data"
///
/// Types that have no illegal bit patterns and can be implicitly
/// reinterpreted as each other without issues
pub trait Pod {}
macro_rules! i { ($($t:ty)*) => { $(impl Pod for $t {})* } }
i!(u8 i8 u16 i16 u32 i32 u64 i64 usize isize f32 f64);

/// Reasons why VecExt::as_slice_of can fail
#[derive(Debug)]
pub enum AsContainerOfError {
    /// The Vec's backing storage does not have the required alignment for the target type
    BadAlignment,
    /// Sizes of the Vec's element type and the target type are not divisible
    IncompatibleSize,
    /// The Vec's len is not divisible by the ratio of the target type's size to the Vec's element size
    IncompatibleLen,
}

fn check_container_compatibility<T: Pod, U: Pod>(ptr: *const T, len: usize, cap: usize) -> Result<(usize, usize), AsContainerOfError> {
    use self::AsContainerOfError::*;

    let my_size = mem::size_of::<T>();
    let slice_size = mem::size_of::<U>();
    
    if ptr as usize % mem::align_of::<U>() != 0 {
        Err(BadAlignment)
    } else if slice_size < my_size {
        if my_size % slice_size != 0 {
            Err(IncompatibleSize)
        } else {
            let ratio = my_size / slice_size;
            Ok((len * ratio, cap * ratio))
        }
    } else {
        let ratio = slice_size / my_size;
        if slice_size % my_size != 0 {
            Err(IncompatibleSize)
        } else if len % ratio != 0 {
            Err(IncompatibleLen)
        } else if cap % ratio != 0 {
            Err(IncompatibleLen)
        } else {
            Ok((len / ratio, cap / ratio))
        }
    }
}

pub trait AsVecOf {
    fn as_vec_of<U: Pod>(self) -> Result<Vec<U>, AsContainerOfError>;
}

impl<T: Pod> AsVecOf for Vec<T> {
    fn as_vec_of<U: Pod>(self) -> Result<Vec<U>, AsContainerOfError> {
        let (ptr, len, cap) = (self.as_ptr(), self.len(), self.capacity());
        check_container_compatibility::<T, U>(ptr, len, cap)
            .map(|(new_len, new_cap)| unsafe {
                mem::forget(self);
                Vec::from_raw_parts(ptr as *mut U, new_len, new_cap)
            })
    }
}

const_and_mut! {
    [
        $trait_name:ident => AsSliceOfExt/AsMutSliceOfExt,
        $fn_name:ident => as_slice_of/as_mut_slice_of,
        $as_ptr:ident => as_ptr/as_mut_ptr,
        $from_raw:ident => from_raw_parts/from_raw_parts_mut,
    ]

    /// Extension trait for Vec
    pub trait $trait_name {
        /// View a Vec as a slice of some other type (if compatible)
        fn $fn_name<U: Pod>(self: cm!(&Self)) -> Result<cm!(&[U]), AsContainerOfError>;
    }

    impl<T: Pod> $trait_name for Vec<T> {
        fn $fn_name<U: Pod>(self: cm!(&Self)) -> Result<cm!(&[U]), AsContainerOfError> {
            let (ptr, len) = (self.$as_ptr(), self.len());
            check_container_compatibility::<T, U>(ptr, len, len)
                .map(|(new_len, _)| unsafe {
                    slice::$from_raw(ptr as cm!(*U), new_len)
                })
        }
    }
}

pub trait SliceExt<T> {
    fn map_in_place<F: FnMut(T) -> T>(&mut self, f: F) where T: Copy;
}

impl<T> SliceExt<T> for [T] {
    fn map_in_place<F: FnMut(T) -> T>(&mut self, mut f: F) where T: Copy {
        for i in 0..self.len() {
            self[i] = f(self[i]);
        }
    }
}

