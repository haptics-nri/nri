use std::{mem, ops};
use std::ops::Add;

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

