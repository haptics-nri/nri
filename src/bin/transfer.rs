#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate glob;
extern crate regex;
extern crate ssh2;

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use ssh2::Session;

macro_rules! path {
    ($begin:tt $(/ $component:tt)*) => {
        &[&*$begin.to_string_lossy(), $($component),*].iter().collect::<PathBuf>().to_string_lossy()
    }
}

error_chain! {
    errors {
        Io(op: &'static str, path: PathBuf) {
            description("I/O operation failed")
            display("Could not {} {}", op, path.display())
        }
    }

    foreign_links {
        Ssh(ssh2::Error);
        Glob(glob::GlobError);
        GlobPattern(glob::PatternError);
    }
}
use ErrorKind::*;

struct SshInfo<'a> {
    user: &'a str,
    pass: &'a str,
    host: &'a str,
    dir: &'a str
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

    let epdir = Path::new(matches.value_of("EPDIR").unwrap());
    let dest = matches.value_of("DEST").unwrap();

    println!("Checking arguments...");

    /* check epdir is a real directory */
    check_dir(epdir)?;

    /* check that dest is a dir or matches [username[:pw]@]hostname:dir */
    let scp_regex = Regex::new(r"^(?P<user>[^:]+):(?P<pass>[^@]+)@(?P<host>[^:]+):(?P<dir>.*)$").unwrap();
    let ssh_info = match scp_regex.captures(dest) {
        Some(captures) => {
            let user = captures.name("user").unwrap().as_str();
            let pass = captures.name("pass").unwrap().as_str();
            let host = captures.name("host").unwrap().as_str();
            let dir = captures.name("dir").unwrap().as_str();
            check_scp(user, pass, host, dir)?;
            Some(SshInfo { user, pass, host, dir })
        }
        None => {
            check_dir(dest)?;
            None
        }
    };

    /* process data if necessary */
    let dats = glob(path!(epdir / "**" / "*.dat"))?;
    if dats.len() != 0 {
        /* ./run.sh all $epdir */
        println!("Processing data...");
        let status = Command::new("./run.sh")
            .arg("all").arg(epdir)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status().chain_err(|| Io("run", "run.sh".into()))?;
        if !status.success() {
            bail!("run.sh failed");
        }
    }

    /* check PNG/CSV/DAT file counts */
    println!("Checking file counts...");
    let dats = glob(path!(epdir / "**" / "*.dat"))?;
    let flows = glob(path!(epdir / "**" / "*.flow"))?;
    let pngs = glob(path!(epdir / "**" / "*.png"))?;
    let csvs = glob(path!(epdir / "**" / "*.csv"))?;
    let bcsvs = glob(path!(epdir / "**" / "biotac.csv"))?;
    let ocsvs = glob(path!(epdir / "**" / "optoforce.csv"))?;
    println!("{} pngs, {} csvs, {} dats", pngs.len(), csvs.len(), dats.len());
    if csvs.len() != 9*flows.len() + bcsvs.len() + ocsvs.len() {
        bail!("Wrong number of CSV files!");
    }
    if dats.len() > 0 && pngs.len() != dats.len() - bcsvs.len() - ocsvs.len() - 2*flows.len() {
        bail!("Wrong number of PNG files!");
    }

    /* find $epdir -name '*.dat' -exec rm {} \; */
    println!("Deleting *.dat files...");
    for dat in dats {
        fs::remove_file(&dat).chain_err(|| Io("delete", dat))?;
    }

    /* rsync $epdir $dest */
    println!("Rsyncing...");
    let mut cmd = if let Some(ssh) = ssh_info {
        let mut cmd = Command::new("sshpass");
        cmd.arg("-p").arg(ssh.pass)
           .arg("rsync")
           .arg("-avhW")
           .arg(&*epdir.to_string_lossy())
           .arg(format!("{}@{}:{}", ssh.user, ssh.host, ssh.dir));
        cmd
    } else {
        let mut cmd = Command::new("rsync");
        cmd.arg("-avhW")
           .arg(&*epdir.to_string_lossy())
           .arg(dest);
        cmd
    };
    let status = cmd.status().chain_err(|| Io("run", "rsync".into()))?;
    if !status.success() {
        bail!("rsync failed");
    }

    println!("Done!");
    Ok(())
}

fn check_dir<P: AsRef<Path>>(p: P) -> Result<()> {
    let p = p.as_ref();
    if !fs::metadata(p).chain_err(|| Io("stat", p.into()))?.is_dir() {
        bail!("{} is not a directory", p.display());
    }
    Ok(())
}

fn check_scp(user: &str, pass: &str, host: &str, dir: &str) -> Result<()> {
    let tcp = TcpStream::connect((host, 22)).chain_err(|| Io("connect to", host.into()))?;
    let mut sess = Session::new().unwrap();
    sess.handshake(&tcp)?;
    sess.userauth_password(user, pass)?;
    if !sess.authenticated() { bail!("SSH authentication failed"); }

    if !sess.sftp()?.stat(Path::new(dir))?.is_dir() {
        bail!("{}@{}:{} is not a directory", user, host, dir);
    }

    Ok(())
}

fn glob(pattern: &str) -> Result<Vec<PathBuf>> {
    glob::glob(pattern)?
        .map(|r| r.map_err(|e| e.into()))
        .collect::<Result<Vec<_>>>()
}

