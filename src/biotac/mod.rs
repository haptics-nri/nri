//! Service to read data from the BioTac sensor

group_attr! {
    #[cfg(target_os = "linux")]

    use ::comms::{Controllable, CmdFrom, Block};
    use std::sync::mpsc::Sender;
    use std::default::Default;
    use std::mem;

    mod wrapper;

    pub struct Biotac {
        cheetah: wrapper::biotac::Cheetah,
    }

    guilty! {
        impl Controllable for Biotac {
            const NAME: &'static str = "biotac",
            const BLOCK: Block = Block::Infinite,

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> Biotac {
                // initialize Cheetah
                let info = wrapper::biotac::bt_info {
                    spi_clock_speed: 4400,
                    number_of_biotacs: 0,
                    sample_rate_Hz: 4400,
                    frame: Default::default(),
                    batch: wrapper::biotac::bt_info_batch {
                        batch_frame_count: 1,
                        batch_ms: 10,
                    },
                };

                let cheetah = unsafe {
                    let mut cheetah = mem::uninitialized();
                    println!("init: {:?}", wrapper::biotac::bt_cheetah_initialize(&info, &mut cheetah));
                    cheetah
                };

                Biotac { cheetah: cheetah }
            }

            fn step(&mut self, _: Option<String>) {
            }

            fn teardown(&mut self) {
                unsafe { wrapper::biotac::bt_cheetah_close(self.cheetah) };
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Biotac);

