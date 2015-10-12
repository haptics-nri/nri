//! Utilities for writing stuff to files

extern crate libc;

use std::{mem, ptr};
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc;
use std::marker::PhantomData;
use std::thread;
use std::convert::Into;
use std::ops::DerefMut;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Handle(usize);

impl Handle {
    pub fn new() -> Handle {
        Handle(0)
    }
    pub fn next(self) -> Handle {
        Handle(self.0 + 1)
    }
}

struct Worker {
    thread : Option<thread::JoinHandle<()>>,
    tx     : Option<mpsc::Sender<Message>>,
}

lazy_static! {
    static ref WORKER: Mutex<Worker> = {
        let (tx, rx) = mpsc::channel();

        let mutex = Mutex::new(Worker {
            thread: Some(thread::spawn(move || {
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
                        },
                        Message::Decoy(h) => {
                            let i = indices[&h] + 1;
                            indices.insert(h, i + 1);
                        },
                    }
                }
            })),

            tx: Some(tx)
        });

        unsafe { libc::atexit(finish_writing) };

        mutex
    };
}

#[derive(PartialEq)]
enum Destination {
    Name,
    Pattern,
}

pub enum Message {
    Open(String, mpsc::Sender<Handle>),
    Close(Handle),
    Write(Handle, Box<[u8]>),

    Register(String, mpsc::Sender<Handle>),
    Unregister(Handle),
    Packet(Handle, Box<[u8]>),
    Decoy(Handle),
}

pub unsafe trait Writable {}

pub struct Writer<T: ?Sized> {
    handle : Handle,
    dst    : Destination,
    _ghost : PhantomData<*const T>,
}

impl<T: ?Sized> Writer<T> {
    pub fn with_file<S: Into<String>>(name: S) -> Writer<T> {
        let (tx, rx) = mpsc::channel();
        send(Message::Open(name.into(), tx));

        Writer {
            handle: rx.recv().unwrap(),
            dst: Destination::Name,
            _ghost: PhantomData
        }
    }

    pub fn with_files<S: Into<String>>(pattern: S) -> Writer<T> {
        let (tx, rx) = mpsc::channel();
        send(Message::Register(pattern.into(), tx));

        Writer {
            handle: rx.recv().unwrap(),
            dst: Destination::Pattern,
            _ghost: PhantomData
        }
    }

    pub fn decoy(&mut self) {
        if self.dst == Destination::Pattern {
            send(Message::Decoy(self.handle));
        }
    }
}

impl<T: Writable + Send + 'static> Writer<T> {
    pub fn write(&mut self, data: T) {
        let mut raw_data = vec![0u8; mem::size_of::<T>()].into_boxed_slice();
        unsafe {
            ptr::copy::<T>(&data as *const T,
                           raw_data.deref_mut() as *mut [u8] as *mut T,
                           1);
        }

        send(match self.dst {
                Destination::Name    => Message::Write(self.handle, raw_data),
                Destination::Pattern => Message::Packet(self.handle, raw_data),
            });
    }
}

impl Writer<[u8]> {
    pub fn write(&mut self, data: &[u8]) {
        let mut raw_data = vec![0u8; data.len()].into_boxed_slice();
        unsafe {
            ptr::copy::<u8>(data as *const[u8] as *const u8,
                            raw_data.deref_mut() as *mut [u8] as *mut u8,
                            data.len());
        }

        send(match self.dst {
                Destination::Name    => Message::Write(self.handle, raw_data),
                Destination::Pattern => Message::Packet(self.handle, raw_data),
            });
    }
}

fn send(m: Message) {
    WORKER.lock().unwrap().tx.as_ref().unwrap().send(m).unwrap();
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
#[allow(unknown_lints)]
#[allow(let_unit_value)]
extern "C" fn finish_writing() {
    // NB: this function must not panic as that will unwind into libc

    abort_on_panic!("Panic while waiting for scribe thread", {
        errorln!("Scribe thread: waiting for outstanding writes");

        // lock the worker thread
        let mut w = match WORKER.lock() {
            Ok(guard)   => guard,
            Err(poison) => {
                errorln!("Scibe thread: mutex poisoned: {:?}", poison);
                return;
            },
        };

        // close the channel
        // this will cause the receiver loop to end once it has processed all outstanding messages
        drop(w.tx.take());

        // now join the thread
        match w.thread.take() {
            Some(handle) => match handle.join() {
                Ok(()) => errorln!("Scribe thread: finished"),
                Err(e) => errorln!("Scribe thread: error while finishing writes: {:?}", e),
            },
            None         => errorln!("Scribe thread: finish_writing called twice!"),
        }
    });
}
