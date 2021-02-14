#![feature(never_type)]
#![feature(iter_map_while)]

mod error;
mod report;
use error::*;
use report::*;

use dsmr5;
use paho_mqtt as mqtt;
use serial::core::SerialDevice;
use std::{io::Read, time::Duration};

const MQTT_TOPIC: &str = "dsmr";

fn main() {
    let host = "tcp://10.10.10.13:1883";
    let mut client = mqtt::Client::new(host).expect("Couldn't create the mqtt client");
    client.set_timeout(Duration::from_secs(5));

    run(client).unwrap();
}

fn run(client: mqtt::Client) -> Result<!, Error> {
    // Open Serial
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    // Connect to mqtt
    client.connect(None)?;

    for readout in reader {
        let telegram = readout.to_telegram()?;
        let report: Report = telegram.objects().filter_map(Result::ok).collect();
        for msg in report.to_mqtt_messages(MQTT_TOPIC.to_owned(), 0) {
            client.publish(msg)?;
        }
    }

    // Reader should never be exhausted
    Err(Error::EndOfReader())
}
