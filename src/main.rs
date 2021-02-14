#![feature(never_type)]
#![feature(iter_map_while)]

mod report;
use report::*;

use dsmr5;
use serial::core::SerialDevice;
use std::{io::Read, time::Duration};


#[derive(Debug)]
enum Error {
    SerialError(serial::Error),
    DSMR5Error(dsmr5::Error),
    EndOfReader(),
}

impl From<serial::Error> for Error {
    fn from(e: serial::Error) -> Self {
        Error::SerialError(e)
    }
}

impl From<dsmr5::Error> for Error {
    fn from(e: dsmr5::Error) -> Self {
        Error::DSMR5Error(e)
    }
}

fn main() {
    run().unwrap();
}

fn run() -> Result<!, Error> {
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    for readout in reader {
        let telegram = readout.to_telegram()?;

        let f = telegram.objects().filter_map(|o| o.ok());

        let report: RawReport = f.collect();

        dbg!(report.active_tariff);
    }

    // Reader should never be exhausted
    Err(Error::EndOfReader())
}
