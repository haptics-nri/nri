pub use time::Duration;

use std::fmt;

use extension_traits::*;

/// Retry some action on failure
pub fn retry<T, E: fmt::Debug, F: FnMut() -> Result<T, E>>(label: Option<&str>, times: usize, delay: Duration, mut action: F) -> Result<T, E> {
    for i in 0..times {
        match action() {
            Ok(t) => return Ok(t),
            Err(e) =>
                if i == times-1 {
                    if let Some(label) = label {
                        println!("ERROR: {} failed {} times :(", label, times);
                    }
                    return Err(e)
                } else {
                    if let Some(label) = label {
                        println!("\tRetrying (#{}/{}) {} ({:?})", i+1, times, label, e);
                    }
                    delay.sleep();
                }
        }
    }
    unreachable!()
}

