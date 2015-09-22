//! Service to read data from the BioTac sensor

group_attr! {
    #[cfg(target_os = "linux")]

    mod wrapper;

    stub!(Biotac);
}

#[cfg(not(target_os = "linux"))]
stub!(Biotac);

