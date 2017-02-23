//! Service to read data from the BioTac sensor

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate utils;
#[macro_use] extern crate comms;

group_attr! {
    #[cfg(target_os = "linux")]

    extern crate scribe;

    extern crate libc;
    extern crate time;

    use comms::{Controllable, CmdFrom, Block};
    use scribe::{Writer, Writable};
    use std::sync::mpsc::Sender;
    use std::default::Default;
    use std::{mem, str};

    mod wrapper;

    pub struct Biotac {
        cheetah: wrapper::biotac::Cheetah,
        info: wrapper::biotac::bt_info,
        finger: u8,
        file: Writer<Packet>,
        i: usize,
        start: time::Tm,
    }

    #[repr(packed)]
    struct Packet {
        stamp: time::Timespec,
        pdc: u32,
        pac: [u32; 22],
        tdc: u32,
        tac: u32,
        electrode: [u32; 19],
    }

    unsafe impl Writable for Packet {}

    guilty! {
        impl Controllable for Biotac {
            const NAME: &'static str = "biotac";
            const BLOCK: Block = Block::Period(10_000_000);

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> Biotac {
                // initialize Cheetah
                let mut info = wrapper::biotac::bt_info {
                    spi_clock_speed: 4400,
                    number_of_biotacs: 1,
                    sample_rate_Hz: 4400,
                    frame: Default::default(),
                    batch: wrapper::biotac::bt_info_batch {
                        batch_frame_count: 1,
                        batch_ms: 10,
                    },
                };

                let cheetah = unsafe {
                    let mut cheetah: wrapper::biotac::Cheetah = mem::zeroed::<wrapper::biotac::Cheetah>();
                    assert!(0 == utils::in_original_dir(|| wrapper::biotac::bt_cheetah_initialize(&info, &mut cheetah)).unwrap());
                    cheetah
                };

                // get properties
                let mut finger = None;
                for i in 1..(3+1) {
                    let props = unsafe {
                        let mut props: wrapper::biotac::bt_property = mem::zeroed::<wrapper::biotac::bt_property>();
                        assert!(0 == wrapper::biotac::bt_cheetah_get_properties(cheetah, i, &mut props));
                        props
                    };
                    if props.bt_connected == 1 {
                        assert!(finger.is_none());
                        finger = Some(i);
                        println!("finger #{} serial number = {}",
                                 i,
                                 str::from_utf8(&props.serial_number[..props.serial_number
                                                                       .iter()
                                                                       .position(|&c| c == 0)
                                                                       .unwrap()])
                                 .unwrap());
                    }
                }
                let finger = finger.unwrap() as u8;

                // configure batch
                unsafe {
                    assert!(0 == wrapper::biotac::bt_cheetah_configure_batch(cheetah, &mut info, 44));
                }

                Biotac { cheetah: cheetah, info: info, finger: finger, file: Writer::with_file("biotac.dat"), i: 0, start: time::now() }
            }

            fn step(&mut self, _: Option<String>) {
                static PARITY: [u8; 128] = [0x01, 0x02, 0x04, 0x07, 0x08, 0x0B, 0x0D, 0x0E,
                                            0x10, 0x13, 0x15, 0x16, 0x19, 0x1A, 0x1C, 0x1F,
                                            0x20, 0x23, 0x25, 0x26, 0x29, 0x2A, 0x2C, 0x2F,
                                            0x31, 0x32, 0x34, 0x37, 0x38, 0x3B, 0x3D, 0x3E,
                                            0x40, 0x43, 0x45, 0x46, 0x49, 0x4A, 0x4C, 0x4F,
                                            0x51, 0x52, 0x54, 0x57, 0x58, 0x5B, 0x5D, 0x5E,
                                            0x61, 0x62, 0x64, 0x67, 0x68, 0x6B, 0x6D, 0x6E,
                                            0x70, 0x73, 0x75, 0x76, 0x79, 0x7A, 0x7C, 0x7F,
                                            0x80, 0x83, 0x85, 0x86, 0x89, 0x8A, 0x8C, 0x8F,
                                            0x91, 0x92, 0x94, 0x97, 0x98, 0x9B, 0x9D, 0x9E,
                                            0xA1, 0xA2, 0xA4, 0xA7, 0xA8, 0xAB, 0xAD, 0xAE,
                                            0xB0, 0xB3, 0xB5, 0xB6, 0xB9, 0xBA, 0xBC, 0xBF,
                                            0xC1, 0xC2, 0xC4, 0xC7, 0xC8, 0xCB, 0xCD, 0xCE,
                                            0xD0, 0xD3, 0xD5, 0xD6, 0xD9, 0xDA, 0xDC, 0xDF,
                                            0xE0, 0xE3, 0xE5, 0xE6, 0xE9, 0xEA, 0xEC, 0xEF,
                                            0xF1, 0xF2, 0xF4, 0xF7, 0xF8, 0xFB, 0xFD, 0xFE];


                self.i += 1;

                let packet = unsafe {
                    let mut packet: Packet = mem::zeroed::<Packet>();
                    packet.stamp = time::get_time();

                    let spi_data_len: i32 = wrapper::cheetah::ch_spi_batch_length(self.cheetah);
                    assert!(spi_data_len == 352);
                    let mut bt_raw_data: Vec<u8> = vec![0u8; spi_data_len as usize];
                    assert!(spi_data_len == wrapper::cheetah::ch_spi_async_collect(self.cheetah, spi_data_len, bt_raw_data.as_mut_ptr()));
                    assert!(spi_data_len == wrapper::cheetah::ch_spi_async_submit(self.cheetah));

                    let byte_shift: i32 = 8;
                    let n_samples: i32 = spi_data_len / byte_shift;
                    let mut pac_index: u32 = 0;
                    for i in 0..n_samples {
                        let channel_id: i8 = (self.info.frame.frame_structure[(i % (self.info.frame.frame_size)) as usize] & 0x7E) >> 1;
                        for j in 0..3 {
                            let high = bt_raw_data[(i*byte_shift + j*2 + 2) as usize];
                            let low  = bt_raw_data[(i*byte_shift + j*2 + 3) as usize];
                            let spi_data: u32 = (high as u32 >> 1) * 32 + (low as u32 >> 3);
                            if (PARITY[(low >> 1) as usize] == low) && (PARITY[(high >> 1) as usize] == high) {
                                match channel_id {
                                    3 => packet.tdc = spi_data,
                                    2 => packet.tac = spi_data,
                                    1 => packet.pdc = spi_data,
                                    0 => packet.pac[pac_index as usize] = spi_data,
                                    c @ 17...35 => packet.electrode[(c - 17) as usize] = spi_data,
                                    _ => println!("bad channel ID at ({}, {})", i, j),
                                }
                            } else if (j+1) as u8 == self.finger {
                                println!("bad parity at ({}, {})", i, j);
                            }
                        }
                        if channel_id == 0 {
                            pac_index += 1;
                        }
                    }

                    packet
                };

                self.file.write(packet);
            }

            fn teardown(&mut self) {
                unsafe { wrapper::biotac::bt_cheetah_close(self.cheetah) };
                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} Biotac packets grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Biotac);

