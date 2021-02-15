#![feature(never_type)]
#![feature(iter_map_while)]
#![feature(backtrace)]

mod error;
mod report;
use error::MyError;
use report::*;

use paho_mqtt as mqtt;
use serial::core::SerialDevice;
use std::{env, io::Read, time::Duration};

struct Config {
    pub mqtt_host: String,
    pub mqtt_topic_prefix: String,
    pub mqtt_qos: i32,
}

impl Config {
    fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            mqtt_host: env::var("MQTT_HOST").unwrap_or(defaults.mqtt_host),
            mqtt_topic_prefix: env::var("MQTT_TOPIC").unwrap_or(defaults.mqtt_topic_prefix),
            mqtt_qos: env::var("MQTT_QOS")
                .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
                .unwrap_or(defaults.mqtt_qos),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt_host: "tcp://10.10.10.13:1883".to_owned(),
            mqtt_topic_prefix: "dsmr".to_owned(),
            mqtt_qos: 0,
        }
    }
}

fn main() {
    let _guard = sentry::init((
        "https://d28f574927f14a54bfa88a781ae298e9@sentry.xirion.net/3",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let cfg = Config::from_env();

    let mut client =
        mqtt::Client::new(cfg.mqtt_host.clone()).expect("Couldn't create the mqtt client");
    client.set_timeout(Duration::from_secs(5));

    loop {
        if let Err(e) = run(&cfg, &client) {
            sentry::capture_error(&e);
        }
    }
}

fn run(cfg: &Config, client: &mqtt::Client) -> Result<!, MyError> {
    // Open Serial
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    // Connect to mqtt
    client.connect(None)?;

    for readout in reader {
        let telegram = readout.to_telegram().map_err(|e| MyError::Dsmr5Error(e))?;
        let measurements: Measurements = telegram.objects().filter_map(Result::ok).collect();

        measurements
            .to_mqtt_messages(&cfg.mqtt_topic_prefix, cfg.mqtt_qos)
            .into_iter()
            .try_for_each(|m| client.publish(m))?;
    }

    // Reader should never be exhausted
    Err(MyError::EndOfReader())
}
