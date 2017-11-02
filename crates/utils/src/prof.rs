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

