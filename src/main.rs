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
use tokio::{io, select, task::JoinHandle};
use std::{io::Read, time::Duration};

#[derive(Debug, Clone)]
struct Config {
    pub mqtt_host: String,
    pub mqtt_topic_prefix: String,
    pub mqtt_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt_host: "10.10.10.13".to_owned(),
            mqtt_topic_prefix: "dsmr".to_owned(),
            mqtt_port: 1883,
        }
    }
}

#[tokio::main]
async fn main() -> ! {
    let cfg = Config::default();

    let mut mqttoptions = MqttOptions::new("dsmr-reader", cfg.mqtt_host.clone(), cfg.mqtt_port);
    mqttoptions.set_keep_alive(30);
    mqttoptions.set_transport(Transport::Tcp);
    
    loop {
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions.clone(), 12);

        let eventloop: JoinHandle<Result<!, io::Error>> = tokio::spawn(async move {
            loop {
                let _event = eventloop.poll().await.unwrap();
            }
        });

        println!("\nStarted dsmr2mqtt...\n");
        select! {
            handle = eventloop => {
                eprintln!("Eventloop stopped: {}", handle.unwrap_err());
            }
            run = run(&cfg, &mut client) => {
                eprintln!("Encountered error running: {}", run.unwrap_err());
            }
        }

        // Cleanup before reseting
        if let Err(e) = client.disconnect().await {
            eprintln!("Error disconnecting: {}", e)
        }

        // Wait a bit before retrying.
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run(cfg: &Config, mut client: &mut AsyncClient) -> Result<!, MyError> {
    // Open Serial
    let mut port = serial::open("/dev/ttyUSB1")?;
    port.set_timeout(Duration::from_secs(1))?;
    let reader = dsmr5::Reader::new(port.bytes().map_while(Result::ok));

    for readout in reader {
        let telegram = readout.to_telegram().map_err(MyError::Dsmr5Error)?;
        let measurements: Measurements = telegram.objects().filter_map(Result::ok).collect();

        let messages = measurements.into_mqtt_messages(cfg.mqtt_topic_prefix.clone());
        for msg in messages {
            msg.send(&mut client).await?;
        }
    }

    // Reader should never be exhausted
    Err(MyError::EndOfReader())
}
