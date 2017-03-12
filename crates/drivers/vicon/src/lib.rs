//! Service to remote-control the Vicon ROS node

#[macro_use] extern crate utils;
#[macro_use] extern crate comms;
#[macro_use] extern crate guilt_by_association;

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate scribe;
    extern crate time;

    use comms::{Controllable, CmdFrom, Block};
    use scribe::Writer;
    use std::process::Command;
    use std::sync::mpsc::Sender;
    use std::{env, str};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct Vicon {
        tx: Sender<CmdFrom>,
        file: String,
        start: time::Tm,
    }

    fn roscmd(cmd: &str) -> bool {
        Command::new("ssh")
            .arg("aburka@158.130.11.59")
            .arg(["source /opt/ros/indigo/setup.bash",
                  "export ROS_PACKAGE_PATH=/home/aburka/ros:$ROS_PACKAGE_PATH",
                  cmd]
                 .join(" && "))
            .status().unwrap()
            .success()
    }

    fn rospub(filename: &str, targets: &[&str]) {
        assert!(roscmd(&format!(r#"rostopic pub -1 /vicon/targets vicon/Targets '{{filename: "{}", targets: [{}]}}'"#,
                                filename, targets.iter().map(|s| format!(r#""{}""#, s)).collect::<Vec<_>>().join(", "))))
    }

    fn roscheck(filename: &str) -> bool {
        roscmd(&format!(r#"wc -l '{}'"#, filename))
    }

    fn transfer(filename: &str) -> Vec<u8> {
        Command::new("ssh")
            .arg("aburka@158.130.11.59")
            .arg(format!("cat {}", filename))
            .output().unwrap()
            .stdout
    }

    guilty! {
        impl Controllable for Vicon {
            const NAME: &'static str = "vicon";
            const BLOCK: Block = Block::Infinite;

            fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Vicon {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let filename = format!("vicon_{}.{}.csv", now.as_secs(), now.subsec_nanos());

                rospub(&filename, &["proton:NewMarker",
                                    "proton:NewMarker1",
                                    "proton:NewMarker2",
                                    "proton:NewMarker3",
                                    "proton:NewMarker4",
                                    "proton:Root"]);

                Vicon { tx: tx, file: filename, start: time::now() }
            }

            fn step(&mut self, _: Option<String>) {
            }

            fn teardown(&mut self) {
                let dir = env::current_dir().unwrap();

                rospub("PAUSE", &[]);
                if !roscheck(&self.file) {
                    self.tx.send(CmdFrom::Data("send msg Vicon node crashed! No data received for latest dataset.".into())).unwrap();
                }
                let readings = transfer(&self.file);
                let n = readings.iter().filter(|&&b| b == b'\n').count();

                Writer::<[u8]>::with_file(dir.join("vicon.tsv").to_str().unwrap()).write(&readings);

                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} Vicon packets grabbed in {} s ({} FPS)!", n, millis/1000.0, 1000.0*(n as f64)/millis);
            }
        }
    }
}

#[cfg(not(feature = "hardware"))]
stub!(Vicon);
