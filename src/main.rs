#![feature(never_type)]
#![feature(iter_map_while)]
#![feature(backtrace)]

mod error;
mod report;
use error::MyError;
use report::*;

use dsmr5;
use paho_mqtt as mqtt;
use serial::core::SerialDevice;
use std::{io::Read, time::Duration};

const MQTT_PREFIX: &str = "dsmr";
const MQTT_QOS: i32 = 0;

fn main() {
    let _guard = sentry::init((
        "https://d28f574927f14a54bfa88a781ae298e9@sentry.xirion.net/3",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let host = "tcp://10.10.10.13:1883";
    let mut client = mqtt::Client::new(host).expect("Couldn't create the mqtt client");
    client.set_timeout(Duration::from_secs(5));

    loop {
        if let Err(e) = run(&client) {
            sentry::capture_error(&e);
        }
    }
}

fn run(client: &mqtt::Client) -> Result<!, MyError> {
    // Open Serial
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    // Connect to mqtt
    client.connect(None)?;

    for readout in reader {
        let telegram = readout.to_telegram().map_err(|e| MyError::DSMR5Error(e))?;
        let measurements: Measurements = telegram.objects().filter_map(Result::ok).collect();
        for msg in measurements.to_mqtt_messages(MQTT_PREFIX, MQTT_QOS) {
            client.publish(msg)?;
        }
    }

    // Reader should never be exhausted
    Err(MyError::EndOfReader())
}
