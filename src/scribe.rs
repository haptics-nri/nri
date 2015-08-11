//! Utilities for writing stuff to files

extern crate threadpool;

use std::{mem, fmt, slice};
use std::fs::File;
use std::io::{self, Write};
use std::any::Any;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::cell::Cell;
use std::marker::PhantomData;
use std::thread;
use self::threadpool::ThreadPool;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Handle(usize);

impl Handle {
    pub fn new() -> Handle { Handle(0) }
    pub fn next(self) -> Handle { Handle(self.0 + 1) }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
    tx: Option<mpsc::Sender<Message>>
}

lazy_static! {
    static ref WORKER: Mutex<Worker> = {
        let (tx, rx) = mpsc::channel();

        Mutex::new(Worker {
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
                            files.get_mut(&h).unwrap().write_all(&data);
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
                            File::create(patterns[&h].replace("{}", &i.to_string())).unwrap().write_all(&data);
                        },
                    }
                }
            })),

            tx: Some(tx)
        })
    };
}

enum Destination<'a> {
    Handle(File),
    Name(fmt::Arguments<'a>)
}

pub enum Message {
    Open(String, mpsc::Sender<Handle>),
    Close(Handle),
    Write(Handle, Box<[u8]>),

    Register(String, mpsc::Sender<Handle>),
    Unregister(Handle),
    Packet(Handle, Box<[u8]>),
}

pub unsafe trait Writable {}

pub struct Writer<'a, T> {
    i: usize,
    dst: Destination<'a>,
    _ghost: PhantomData<*const T>
}

impl<'a, T: Writable + Send + 'static> Writer<'a, T> {
    pub fn to_file(name: &str) -> io::Result<Writer<T>> {
        Ok(Writer {
            i: 0,
            dst: Destination::Handle(try!(File::create(name))),
            _ghost: PhantomData
        })
    }

    pub fn to_files(pattern: fmt::Arguments<'a>) -> Writer<T> {
        Writer {
            i: 0,
            dst: Destination::Name(pattern),
            _ghost: PhantomData
        }
    }

    pub fn write(&mut self, data: T) {
        self.i += 1;
    }
}

/// Joins the worker thread, waiting for all outstanding writes to finish.
///
/// Do not call this function twice, as it will panic!
pub fn finish_writing() -> thread::Result<()> {
    let mut w = WORKER.lock().unwrap();
    drop(w.tx.take());
    w.thread.take().unwrap().join()
}

