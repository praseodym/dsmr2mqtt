#![feature(never_type)]
#![feature(iter_map_while)]
#![feature(backtrace)]

mod error;
mod report;
mod mqtt;
use error::MyError;
use report::*;

use rumqttc::{AsyncClient, MqttOptions, Transport};
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

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();

    let mut mqttoptions = MqttOptions::new("dsmr-reader", "10.10.10.13", 1883);
    mqttoptions.set_keep_alive(30);
    mqttoptions.set_transport(Transport::Tcp);
    
    loop {
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions.clone(), 10);

        tokio::spawn(async move {
            loop {
                let notification = eventloop.poll().await.unwrap();
                println!("Received = {:?}", notification);
            }
        });

        if let Err(e) = run(&cfg, &mut client).await {
            eprintln!("Error occured: {}", &e);
        }

        if let Err(e) = client.disconnect().await {
            eprintln!("Error disconnecting: {}", e)
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run(cfg: &Config, mut client: &mut AsyncClient) -> Result<!, MyError> {
    // Open Serial
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    for readout in reader {
        let telegram = readout.to_telegram().map_err(|e| MyError::Dsmr5Error(e))?;
        let measurements: Measurements = telegram.objects().filter_map(Result::ok).collect();

        let messages = measurements.to_mqtt_messages(cfg.mqtt_topic_prefix.clone());
        for msg in messages {
            msg.send(&mut client).await?;
        }
    }

    // Reader should never be exhausted
    Err(MyError::EndOfReader())
}
