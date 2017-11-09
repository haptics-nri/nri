use std::collections::{btree_map, hash_map};
use std::{cmp, mem, ptr, slice, thread};
use time::Duration;

extension_trait! {
    pub DurationExt for Duration {
        fn sleep(&self) {
            thread::sleep(self.to_std().unwrap());
        }
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

extension_trait! {
    <T: Pod> pub AsVecOf for Vec<T> {
        fn as_vec_of<U: Pod>(self) -> Result<Vec<U>, AsContainerOfError> {
            let (ptr, len, cap) = (self.as_ptr(), self.len(), self.capacity());
            check_container_compatibility::<T, U>(ptr, len, cap)
                .map(|(new_len, new_cap)| unsafe {
                    mem::forget(self);
                    Vec::from_raw_parts(ptr as *mut U, new_len, new_cap)
                })
        }
    }
}

const_and_mut! {
    [
        $trait_name:ident => AsSliceOfExt/AsMutSliceOfExt,
        $fn_name:ident => as_slice_of/as_mut_slice_of,
        $as_ptr:ident => as_ptr/as_mut_ptr,
        $from_raw:ident => from_raw_parts/from_raw_parts_mut,
    ]

    extension_trait! {
        <T: Pod> pub $trait_name for Vec<T> {
            fn $fn_name<U: Pod>(self: cm!(&Self)) -> Result<cm!(&[U]), AsContainerOfError> {
                let (ptr, len) = (self.$as_ptr(), self.len());
                check_container_compatibility::<T, U>(ptr, len, len)
                    .map(|(new_len, _)| unsafe {
                        slice::$from_raw(ptr as cm!(*U), new_len)
                    })
            }
        }
    }
}

extension_trait! {
    <T: Copy> pub SliceExt<T> for [T] {
        fn map_in_place<F: FnMut(T) -> T>(&mut self, mut f: F) {
            for i in 0..self.len() {
                self[i] = f(self[i]);
            }
        }
    }
}

extension_trait! {
    <T> pub CircularPush<T> for Vec<T> {
        fn circular_push(&mut self, item: T) {
            if self.len() == self.capacity() {
                let len = self.len() - 1;
                unsafe {
                    ptr::copy(&self[1], &mut self[0], len);
                }
                self.truncate(len);
            }
            self.push(item);
        }
    }
}

// FIXME remove when and_modify stabilizes #44733
extension_trait! {
    <'a, K, V> pub HashMapAndModifyExt<V> for hash_map::Entry<'a, K, V> {
        fn and_modify<F>(self, mut f: F) -> Self where F: FnMut(&mut V) {
            use self::hash_map::Entry::*;
            match self {
                Occupied(mut entry) => {
                    f(entry.get_mut());
                    Occupied(entry)
                },
                Vacant(entry) => Vacant(entry),
            }
        }
    }
}

extension_trait! {
    <'a, K: cmp::Ord, V> pub BTreeMapAndModifyExt<V> for btree_map::Entry<'a, K, V> {
        fn and_modify<F>(self, mut f: F) -> Self where F: FnMut(&mut V) {
            use self::btree_map::Entry::*;
            match self {
                Occupied(mut entry) => {
                    f(entry.get_mut());
                    Occupied(entry)
                },
                Vacant(entry) => Vacant(entry),
            }
        }
    }
}

extension_trait! {
    pub UnsignedExt for u32 {
        fn absdiff(self, rhs: u32) -> u32 {
            if self > rhs {
                self - rhs
            } else {
                rhs - self
            }
        }
    }
}

