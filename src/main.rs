use std::io::Write;
use std::{fs, io::Read, time::Duration};

use rumqttc::{AsyncClient, MqttOptions, Transport};
use serial::SerialPort;
use tokio::{select, task::JoinHandle};

use error::MyError;
use report::*;

mod error;
mod mqtt;
mod report;

#[derive(Debug, Clone)]
struct Config {
    pub mqtt_host: String,
    pub mqtt_topic_prefix: String,
    pub mqtt_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt_host: "::1".to_owned(),
            mqtt_topic_prefix: "dsmr".to_owned(),
            mqtt_port: 1883,
        }
    }
}

#[tokio::main]
async fn main() -> ! {
    let cfg = Config::default();

    let mut mqttoptions = MqttOptions::new("dsmr-reader", cfg.mqtt_host.clone(), cfg.mqtt_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_transport(Transport::Tcp);

    loop {
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions.clone(), 12);

        let eventloop: JoinHandle<_> = tokio::spawn(async move {
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

async fn run(cfg: &Config, client: &mut AsyncClient) -> Result<(), MyError> {
    // Open Serial
    let mut port = serial::open("/dev/ttyDSMR")?;
    let settings = serial::PortSettings {
        baud_rate: serial::BaudRate::Baud115200,
        char_size: serial::CharSize::Bits8,
        parity: serial::Parity::ParityNone,
        stop_bits: serial::StopBits::Stop1,
        flow_control: serial::FlowControl::FlowNone,
    };
    port.configure(&settings)?;
    port.set_timeout(Duration::from_secs(10))?;
    let reader = dsmr5::Reader::new(port.bytes().take_while(Result::is_ok).map(Result::unwrap));

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("measurements.tsv")
        .unwrap();
    let measurements_topic = format!("{}/{}", cfg.mqtt_topic_prefix.clone(), "measurements");

    for readout in reader {
        let telegram = readout.to_telegram().map_err(MyError::Dsmr5Error)?;
        let measurements: Measurements = telegram.objects().filter_map(Result::ok).collect();

        writeln!(&mut file, "{}", &measurements.report()).unwrap();
        let json = serde_json::to_string(&measurements).unwrap();

        let messages = measurements.into_mqtt_messages(cfg.mqtt_topic_prefix.clone());
        for msg in messages {
            msg.send(client).await?;
        }

        mqtt::Message::new(&measurements_topic, rumqttc::QoS::AtMostOnce, true, json)
            .send(client)
            .await?;
    }

    // Reader should never be exhausted
    Err(MyError::EndOfReader())
}
