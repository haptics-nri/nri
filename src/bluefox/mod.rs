use super::comms::Controllable;

pub struct Bluefox;

impl Controllable<Bluefox> for Bluefox {
    fn setup() -> Bluefox {
        Bluefox
    }

    fn step(&mut self) -> bool {
        true
    }

    fn teardown(&mut self) {
    }
}

