/// Just like println!, but prints to stderr
// FIXME: remove after upgrading since eprintln! is stable
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

#[macro_export]
macro_rules! join {
    ($iter:expr, $sep:expr) => {
        $iter.collect::<Vec<_>>().join($sep)
    };
    ($prefix:expr => $iter:expr, $sep:expr) => {
        join!(::std::iter::once($prefix).chain($iter), $sep)
    };
    ($prefix:expr => $iter:expr => $suffix:expr, $sep:expr) => {
        join!(::std::iter::once($prefix).chain($iter).chain(::std::iter::once($suffix)), $sep)
    };
}

#[macro_export]
macro_rules! foreach {
    ($dol:tt $var:tt => [$($val:ident),*] { $($body:tt)* }) => {{
        macro_rules! __foreach {
            ($dol $var:ident) => {
                $($body)*
            }
        }
        $(__foreach!($val);)*
    }}
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

macro_rules! pub_use_mod {
    ($name:ident) => {
        mod $name;
        pub use $name::*;
    }
}

