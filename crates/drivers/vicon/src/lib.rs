//! Service to remote-control the Vicon ROS node

#[macro_use] extern crate utils;
#[macro_use] extern crate comms;
#[macro_use] extern crate guilt_by_association;

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate scribe;
    extern crate time;

    use comms::{Controllable, CmdFrom, Block};
    use scribe::Writer;
    use std::process::Command;
    use std::sync::mpsc::Sender;
    use std::str;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct Vicon {
        file: String,
        start: time::Tm,
    }

    fn rospub(filename: &str, targets: &[&str]) {
        assert!(Command::new("ssh")
                .arg("aburka@158.130.11.59")
                .arg(["source /opt/ros/indigo/setup.bash",
                      "export ROS_PACKAGE_PATH=/home/aburka/ros:$ROS_PACKAGE_PATH",
                      &format!(r#"rostopic pub -1 /vicon/targets vicon/Targets '{{filename: "{}", targets: [{}]}}'"#,
                               filename, targets.iter().map(|s| format!(r#""{}""#, s)).collect::<Vec<_>>().join(", "))]
                     .join(" && "))
                .status().unwrap()
                .success());
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
            const NAME: &'static str = "vicon",
            const BLOCK: Block = Block::Infinite,

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> Vicon {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let filename = format!("vicon_{}.{}.csv", now.as_secs(), now.subsec_nanos());

                rospub(&filename, &["proton:NewMarker",
                                    "proton:NewMarker1",
                                    "proton:NewMarker2",
                                    "proton:NewMarker3",
                                    "proton:NewMarker4",
                                    "proton:Root"]);

                Vicon { file: filename, start: time::now() }
            }

            fn step(&mut self, _: Option<String>) {
            }

            fn teardown(&mut self) {
                rospub("PAUSE", &[]);
                let readings = transfer(&self.file);
                let n = readings.iter().filter(|&&b| b == b'\n').count();

                Writer::<[u8]>::with_file("vicon.tsv").write(&readings);

                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} Vicon packets grabbed in {} s ({} FPS)!", n, millis/1000.0, 1000.0*(n as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Teensy);
