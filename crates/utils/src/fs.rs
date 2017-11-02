use errno::errno;
use libc;

use std::{env, fs, io, mem, thread};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, RwLock};

pub fn in_original_dir<F: FnOnce() -> R, R>(action: &str, f: F) -> io::Result<R> {
    lazy_static! {
        static ref ORIGINAL_DIR: PathBuf = env::current_dir().unwrap();
        static ref CRITICAL_SECTION: Mutex<()> = Mutex::new(());
    }

    // ensure that two threads don't fight over the current dir
    let _guard = CRITICAL_SECTION.lock().unwrap();

    let before = env::current_dir()?;
    println!("Executing {} in {} (currently in {})", action, ORIGINAL_DIR.display(), before.display());
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

/// Check free space on the volume containing a local path
pub fn df(path: &Path) -> io::Result<u64> {
    unsafe {
        let mut stat: libc::statfs = mem::zeroed();
        if libc::statfs(CString::new(path.as_os_str().as_bytes()).unwrap().as_ptr(), &mut stat) == 0 {
            Ok(stat.f_bavail * stat.f_bsize as u64)
        } else {
            Err(io::Error::from_raw_os_error(errno().0))
        }
    }
}

pub fn watch<T, U, F>(mut thing: T,
                  global: &'static U,
                  root: &Path,
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
                  root: &Path,
                  ext: &'static str| {
        fs::read_dir(root).expect(&format!("could not read directory {:?}", root))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| match p.extension() { Some(x) if x == ext => true, _ => false })
            .map(|p| f(thingref, p))
            .count();
    };

    update(&mut thing, &mut f, root, ext);

    let root = root.to_owned();
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut w = watcher(tx, Duration::from_millis(100)).expect("failed to crate watcher");
        w.watch(&root, Recursive).expect("watcher refused to watch");

        for evt in rx {
            match evt {
                Create(ref path) | Write(ref path) | Remove(ref path) | Rename(_, ref path) => {
                    if let Some(x) = path.extension() {
                        if x == ext {
                            print!("Updating... ({:?})", evt);
                            let mut thing = global.write().expect("couldn't get a write lock");
                            update(&mut *thing, &mut f, &root, ext);
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

