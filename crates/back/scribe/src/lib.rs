//! Utilities for writing stuff to files

#[macro_use] extern crate utils;

extern crate libc;
#[macro_use] extern crate lazy_static;

use std::{mem, ptr};
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::marker::PhantomData;
use std::thread;
use std::convert::Into;
use std::ops::DerefMut;
use std::panic;

/// Helper struct to wrap an auto-incrementing integer.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct Handle(usize);

impl Handle {
    /// Make a new handle.
    fn new() -> Handle {
        Handle(0)
    }

    /// Get the next handle in the sequence.
    fn next(self) -> Handle {
        Handle(self.0 + 1)
    }
}

/// Worker thread that takes data from other threads and writes them to files.
///
/// A singleton thread runs in a lazy static.
struct Worker {
    /// Handle to running thread
    thread : thread::JoinHandle<()>,

    /// Sending end of channel that the worker thread listens to
    tx     : mpsc::Sender<Message>,
}

lazy_static! {
    static ref WORKER: Mutex<Worker> = {
        let (tx, rx) = mpsc::channel();

        let mutex = Mutex::new(Worker {
            thread: thread::spawn(move || {
                let mut files = HashMap::<Handle, File>::new();
                let mut patterns = HashMap::<Handle, String>::new();
                let mut indices = HashMap::<Handle, usize>::new();

                let mut max_file = Handle::new();
                let mut max_pattern = Handle::new();

                for msg in rx.into_iter() {
                    match msg {
                        Message::Open(s, tx) => {
                            max_file = max_file.next();
                            files.insert(max_file, File::create(&s).unwrap());
                            tx.send(max_file).unwrap();
                        },
                        Message::Close(h) => {
                            files.remove(&h);
                        },
                        Message::Write(h, data) => {
                            files.get_mut(&h).unwrap().write_all(&data).unwrap();
                            COUNT.fetch_sub(1, Ordering::SeqCst);
                        },

                        Message::Register(s, tx) => {
                            max_pattern = max_pattern.next();
                            patterns.insert(max_pattern, s);
                            indices.insert(max_pattern, 1);
                            tx.send(max_pattern).unwrap();
                        },
                        Message::Unregister(h) => {
                            patterns.remove(&h);
                            indices.remove(&h);
                        },
                        Message::Packet(h, data) => {
                            let i = indices[&h];
                            indices.insert(h, i + 1);
                            File::create(patterns[&h].replace("{}", &i.to_string())).unwrap().write_all(&data).unwrap();
                            COUNT.fetch_sub(1, Ordering::SeqCst);
                        },
                        Message::SetIndex(h, index) => {
                            indices.insert(h, index);
                        },
                    }
                }
            }),

            tx: tx
        });

        unsafe { libc::atexit(finish_writing) };

        mutex
    };
}

pub static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

#[derive(PartialEq)]
enum Destination {
    Name,
    Pattern,
}

enum Message {
    Open(String, mpsc::Sender<Handle>),
    Close(Handle),
    Write(Handle, Box<[u8]>),

    Register(String, mpsc::Sender<Handle>),
    Unregister(Handle),
    Packet(Handle, Box<[u8]>),
    SetIndex(Handle, usize),
}

/// Marks a type that can be "serialized" by transmuting to `[u8]`
pub unsafe trait Writable {}

/// Helper type used for writing a sequence of `Writable` packets into a file or files
pub struct Writer<T: ?Sized> {
    handle : Handle,
    dst    : Destination,
    _ghost : PhantomData<*const T>,
}

impl<T: ?Sized> Writer<T> {
    /// Create a new `Writer` that creates the file `name` and writes `T`s into it.
    ///
    /// The file is closed when the `Writer` is dropped.
    pub fn with_file<S: Into<String>>(name: S) -> Writer<T> {
        let (tx, rx) = mpsc::channel();
        send(Message::Open(name.into(), tx));

        Writer {
            handle: rx.recv().unwrap(),
            dst: Destination::Name,
            _ghost: PhantomData
        }
    }

    /// Create a new `Writer` that creates files based on `pattern` for each `T`.
    pub fn with_files<S: Into<String>>(pattern: S) -> Writer<T> {
        let (tx, rx) = mpsc::channel();
        send(Message::Register(pattern.into(), tx));

        Writer {
            handle: rx.recv().unwrap(),
            dst: Destination::Pattern,
            _ghost: PhantomData
        }
    }

    /// Fix up the internal packet index of an existing `Writer` (in the common case, it is automatically incremented).
    pub fn set_index(&mut self, index: usize) {
        if self.dst == Destination::Pattern {
            send(Message::SetIndex(self.handle, index));
        }
    }
}

impl<T: Writable + Send + 'static> Writer<T> {
    /// Write a serializable type to disk (into the open file or a new one, according to whether
    /// this `Writer` was constructed with `with_file` or `with_files`).
    pub fn write(&mut self, data: T) {
        // "serialize" by copying into a Box<[u8]>
        let mut raw_data = vec![0u8; mem::size_of::<T>()].into_boxed_slice();
        unsafe {
            ptr::copy_nonoverlapping::<T>(&data as *const T,
                                          raw_data.deref_mut() as *mut [u8] as *mut T,
                                          1);
        }

        // internal scribe status counter
        COUNT.fetch_add(1, Ordering::SeqCst);

        send(match self.dst {
                Destination::Name    => Message::Write(self.handle, raw_data),
                Destination::Pattern => Message::Packet(self.handle, raw_data),
            });
    }
}

impl Writer<[u8]> {
    /// Special case for `&[u8]`. Write to disk (into the open file or a new one, according to
    /// whether this `Writer` was constructed with `with_file` or `with_files`).
    pub fn write(&mut self, data: &[u8]) {
        // copy to Box<[u8]>
        let mut raw_data = vec![0u8; data.len()].into_boxed_slice();
        unsafe {
            ptr::copy::<u8>(data as *const[u8] as *const u8,
                            raw_data.deref_mut() as *mut [u8] as *mut u8,
                            data.len());
        }

        // internal scribe status counter
        COUNT.fetch_add(1, Ordering::SeqCst);

        send(match self.dst {
                Destination::Name    => Message::Write(self.handle, raw_data),
                Destination::Pattern => Message::Packet(self.handle, raw_data),
            });
    }
}

/// Convenience method for sending off a message to the worker thread.
fn send(m: Message) {
    WORKER.lock().unwrap().tx.send(m).unwrap();
}

impl<T: ?Sized> Drop for Writer<T> {
    fn drop(&mut self) {
        send(match self.dst {
                Destination::Name    => Message::Close(self.handle),
                Destination::Pattern => Message::Unregister(self.handle),
            });
    }
}

/// Joins the worker thread, waiting for all outstanding writes to finish.
///
/// This function is called automatically at program exit (assuming the scribe thread has been
/// started).
///
/// Do not call this function twice, as it will panic!
extern "C" fn finish_writing() {
    // NB: this function must not panic as that will unwind into libc
    // To prevent such unwinding, we catch panics here. The only possible unintentional panic is
    // from errorln!, which panics if stderr is not available due to the process shutdown already
    // in progress. Also, we intentionally panic in order to poison the WORKER mutex. For both of
    // those the correct action is just to silently exit.
    let _ = panic::catch_unwind(|| {
        errorln!("Scribe thread: waiting for outstanding writes");

        // lock the worker thread
        let w = match WORKER.lock() {
            Ok(guard)   => guard,
            Err(_) => {
                errorln!("Scribe thread: ✗ finish_writing called twice!");
                return;
            },
        };

        // The following code is unsafe because we use ptr::read to extract owned values from the
        // mutex. This is sound because we panic in order to poison the mutex, so it can't happen
        // twice. Reentrancy is also not possible because we hold the mutex guard.
        unsafe {
            // close the channel
            // this will cause the receiver loop to end once it has processed all outstanding messages
            drop(ptr::read(&w.tx));

            // now join the thread
            match ptr::read(&w.thread).join() {
                Ok(()) => errorln!("Scribe thread: ✓ finished"),
                Err(e) => errorln!("Scribe thread: ✗ error while finishing writes: {:?}", e),
            }

            // we are about to panic intentionally, so turn off the panic message
            panic::set_hook(Box::new(|_| {}));
            panic!("intentionally poisoning the WORKER mutex");
        }
    });
}
