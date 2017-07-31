#[macro_use] extern crate clap;
extern crate errno;
#[macro_use] extern crate error_chain;
extern crate fallible_iterator;
extern crate indicatif;
extern crate globset;
extern crate libc;
extern crate regex;
extern crate ssh2;
extern crate walkdir;

extern crate nri;
#[macro_use] extern crate closet;

use std::{fs, mem, thread};
use std::ffi::CString;
use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use errno::errno;
use fallible_iterator::{Convert, FallibleIterator};
use globset::Glob;
use indicatif::HumanBytes as Bytes;
use regex::Regex;
use ssh2::{Session, Sftp};
use walkdir::WalkDir;

use nri::{MultiProgress, make_bar, make_bar_bytes};

error_chain! {
    errors {
        Io(op: &'static str, path: PathBuf) {
            description("I/O operation failed")
            display("Could not {} {}", op, path.display())
        }
    }

    foreign_links {
        Ssh(ssh2::Error);
        Glob(globset::Error);
        WalkDir(walkdir::Error);
    }
}
use ErrorKind::*;
use std::result::Result as StdResult;

/// Extension trait to convert an Iterator to a FallibleIterator
trait FallibleConverter: Iterator + Sized {
    fn fallible(self) -> Convert<Self>;
}

impl<T, E, I: Iterator<Item=StdResult<T, E>>> FallibleConverter for I {
    fn fallible(self) -> Convert<Self> {
        fallible_iterator::convert(self)
    }
}

/// Parameters for making an SSH connection
#[derive(Clone)]
struct SshInfo {
    user: String,
    pass: String,
    host: String,
    dir: PathBuf
}

fn main() {
    if let Err(ref e) = try_main() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let matches = clap_app! { nri_transfer =>
        (version: crate_version!())
        (author: crate_authors!("\n"))
        (about: "Helper for processing/transferring dataset files")

        (@arg EPDIR: +required "Episode directory")
        (@arg DEST: +required "Destination (path or SSH address)")
    }.get_matches();

    let epdir = Path::new(matches.value_of("EPDIR").unwrap()).to_owned();
    let dest = matches.value_of("DEST").unwrap().to_owned();

    println!("Checking arguments...");

    /* check epdir is a real directory */
    check_dir(&epdir)?;

    /* check that dest is a dir or matches username:pw@hostname:dir */
    let scp_regex = Regex::new(r"^(?P<user>[^:]+):(?P<pass>[^@]+)@(?P<host>[^:]+):(?P<dir>.*)$").unwrap();
    let ssh_info = match scp_regex.captures(&dest) {
        Some(captures) => {
            let info = SshInfo {
                user: captures.name("user").unwrap().as_str().to_owned(),
                pass: captures.name("pass").unwrap().as_str().to_owned(),
                host: captures.name("host").unwrap().as_str().to_owned(),
                dir: PathBuf::from(captures.name("dir").unwrap().as_str()),
            };
            check_scp(info.clone())?;
            Some(info)
        }
        None => {
            check_dir(&dest)?;
            None
        }
    };

    /* process data if necessary */
    let dats = glob(&epdir, "*.dat")?;
    if dats.len() != 0 {
        /* ./run.sh all $epdir */
        println!("Processing data...");
        let status = Command::new("./run.sh")
            .arg("all").arg(&epdir)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status().chain_err(|| Io("run", "run.sh".into()))?;
        if !status.success() {
            bail!("run.sh failed");
        }
    }

    /* check PNG/CSV/DAT file counts */
    println!("Checking file counts...");
    let dats  = glob(&epdir, "*.dat")?;
    let flows = glob(&epdir, "*.flow")?;
    let pngs  = glob(&epdir, "*.png")?;
    let csvs  = glob(&epdir, "*.csv")?;
    let bcsvs = glob(&epdir, "biotac.csv")?;
    let ocsvs = glob(&epdir, "optoforce.csv")?;
    println!("{} pngs, {} csvs, {} dats", pngs.len(), csvs.len(), dats.len());
    if csvs.len() != 9*flows.len() + bcsvs.len() + ocsvs.len() {
        bail!("Wrong number of CSV files!");
    }
    if dats.len() > 0 && pngs.len() != dats.len() - bcsvs.len() - ocsvs.len() - 2*flows.len() {
        bail!("Wrong number of PNG files!");
    }

    /* find $epdir -name '*.dat' -exec rm {} \; */
    if dats.len() > 0 {
        println!("Deleting *.dat files...");
        for dat in dats {
            fs::remove_file(&dat).chain_err(|| Io("delete", dat))?;
        }
    }

    /* check disk space */
    let size = local_du(&epdir)?;
    let free = ssh_info.clone().map(remote_df).unwrap_or_else(|| local_df(Path::new(&dest)))?;
    println!("{} required, {} free", size, free);
    if size.0 > free.0 {
        bail!("not enough space at destination");
    }

    /* rsync -avh $epdir $dest (with progress bars!) */
    println!("Rsyncing...");
    let rsync_args = &["-avh"];
    let mut cmd = if let Some(ssh) = ssh_info.clone() {
        let mut cmd = Command::new("sshpass");
        cmd.arg("-p").arg(ssh.pass)
           .arg("rsync")
           .args(rsync_args)
           .arg(&epdir)
           .arg(format!("{}@{}:{}", ssh.user, ssh.host, ssh.dir.display()));
        cmd
    } else {
        let mut cmd = Command::new("rsync");
        cmd.args(rsync_args)
           .arg(&epdir)
           .arg(&dest);
        cmd
    };
    cmd.stdout(Stdio::piped()); // capture stdout for the progress bar
    let mut child = cmd.spawn().chain_err(|| Io("run", "rsync".into()))?;

    let bars = MultiProgress::new();
    let num_bar = bars.add(make_bar((flows.len() + pngs.len() + csvs.len()) as u64)); // the first progress bar counts files transferred/evaluated
    let size_bar = bars.add(make_bar_bytes(size.0 as u64)); // the second progress bar measures disk space
    num_bar.set_message("Files");
    size_bar.set_message("Bytes");

    let num_progress = thread::spawn(move || -> Result<ExitStatus> {
        num_bar.set_position(0);
        for line in BufReader::new(child.stdout.as_mut().unwrap()).lines() {
            let line = line.chain_err(|| Io("read output", "rsync".into()))?;
            if line.ends_with(".flow") || line.ends_with(".png") || line.ends_with(".csv") {
                num_bar.inc(1);
            }
        }
        num_bar.finish();

        // this shouldn't wait at all because the process is definitely done (it closed stdout)
        Ok(child.wait().chain_err(|| Io("wait on", "rsync".into()))?)
    });

    let rsync_done = Arc::new(AtomicBool::new(false)); // signal for telling size progress bar to stop
    let size_progress = thread::spawn(clone_army!([rsync_done, ssh_info] move || -> Result<()> {
        size_bar.set_position(0);
        while !rsync_done.load(atomic::Ordering::SeqCst) {
            size_bar.set_position(ssh_info.clone()
                                          .map(|SshInfo { user, pass, host, dir }| {
                                              let mut dir = PathBuf::from(dir);
                                              dir.push(epdir.file_name().unwrap()); // du -sh $REMOTE_DIR/$(basename $LOCAL_DIR)
                                              remote_du(SshInfo { user, pass, host, dir })
                                          })
                                          .unwrap_or_else(|| local_du(Path::new(&dest)))?.0);
        }
        size_bar.finish();
        Ok(())
    }));

    // and one thread to bind them
    let progress = thread::spawn(move || -> Result<()> {
        bars.join().chain_err(|| Io("draw", "progress bar".into()))
    });

    let status = num_progress.join().unwrap()?; // first thread exits when rsync does
    rsync_done.store(true, atomic::Ordering::SeqCst); // signal second thread to exit
    size_progress.join().unwrap()?; // wait until it does (could be in the middle of a du)
    progress.join().unwrap()?; // progress bar drawer thread should exit right away

    if !status.success() {
        bail!("rsync failed");
    }

    println!("Done!");
    Ok(())
}

/// Set up an SSH connection
fn ssh(user: &str, pass: &str, host: &str) -> Result<(TcpStream, Session)> {
    let tcp = TcpStream::connect((host, 22)).chain_err(|| Io("connect to", host.into()))?;
    let mut sess = Session::new().unwrap();
    sess.handshake(&tcp)?;
    sess.userauth_password(user, pass)?;
    if !sess.authenticated() { bail!("SSH authentication failed"); }
    Ok((tcp, sess))
}

/// Make sure a given local path represents a directory
fn check_dir<P: AsRef<Path>>(p: P) -> Result<()> {
    let p = p.as_ref();
    // Path::is_dir doesn't differentiate between an error and a non-directory, so do it separately
    if !fs::metadata(p).chain_err(|| Io("stat", p.into()))?.is_dir() {
        bail!("{} is not a directory", p.display());
    }
    Ok(())
}

/// Make sure a given remote path represents a directory
fn check_scp(SshInfo { user, pass, host, dir }: SshInfo) -> Result<()> {
    let (_tcp, sess) = ssh(&user, &pass, &host)?;

    if !sess.sftp()?.stat(&dir)?.is_dir() {
        bail!("{}@{}:{} is not a directory", user, host, dir.display());
    }

    Ok(())
}

/// find $dir -name $name
fn glob<P: AsRef<Path>>(dir: P, name: &str) -> Result<Vec<PathBuf>> {
    let pattern = Glob::new(name)?.compile_matcher();
    Ok(WalkDir::new(dir).into_iter().fallible()
        .filter(|entry| pattern.is_match(entry.file_name()))
        .map(|entry| entry.path().to_owned())
        .collect()?)
}

/// Measure the size-on-disk of a local directory
fn local_du<P: AsRef<Path>>(path: P) -> Result<Bytes> {
    Ok(WalkDir::new(path).into_iter().fallible()
        .and_then(|entry| Ok(entry.metadata()?.len()))
        .fold(0, |a, b| a + b)
        .map(Bytes)?)
}

/// Measure the size-on-disk of a remote directory
// TODO combine walkdir + ssh2
fn remote_du(SshInfo { user, pass, host, dir }: SshInfo) -> Result<Bytes> {
    fn subdir(sftp: &Sftp, host: &str, path: &Path) -> Result<Bytes> {
        Ok(sftp.readdir(path)?
               .into_iter()
               .map(|(path, stat)| {
                   if stat.is_dir() {
                       subdir(sftp, host, &path).unwrap_or_else(|e| {
                           println!("warning: could not list directory {}:{} (skipping): {}",
                                    host, path.display(), e);
                           Bytes(0)
                       })
                   } else {
                       Bytes(stat.size.unwrap_or_else(|| {
                           println!("warning: could not read file size of {}:{} (skipping)",
                                    host, path.display());
                           0
                       }))
                   }
               })
               .fold(Bytes(0), |Bytes(a), Bytes(b)| Bytes(a+b)))
    }

    let (_tcp, sess) = ssh(&user, &pass, &host)?;
    let sftp = sess.sftp()?;
    subdir(&sftp, &host, &dir)
}

/// Check free space on the volume containing a local path
fn local_df(path: &Path) -> Result<Bytes> {
    unsafe {
        let mut stat: libc::statfs = mem::zeroed();
        if libc::statfs(CString::new(path.as_os_str().as_bytes()).unwrap().as_ptr(), &mut stat) == 0 {
            Ok(Bytes(stat.f_bavail * stat.f_bsize as u64))
        } else {
            Err(Error::with_chain(io::Error::from_raw_os_error(errno().0), Io("measure", path.into())))
        }
    }
}

/// Check free space on the volume containing a remote path
fn remote_df(SshInfo { user, pass, host, dir }: SshInfo) -> Result<Bytes> {
    let (_tcp, sess) = ssh(&user, &pass, &host)?;
    let sftp = sess.sftp()?;
    let attrs = sftp.open(&dir)?.statvfs()?;
    Ok(Bytes(attrs.f_bavail * attrs.f_bsize))
}

