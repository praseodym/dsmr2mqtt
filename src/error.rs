use std::backtrace::Backtrace;

use mqtt_async_client as mqtt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("serial connection failed")]
    SerialError {
        #[from]
        source: serial::Error,
        backtrace: Backtrace,
    },

    #[error("parsing dsmr failed")]
    Dsmr5Error(dsmr5::Error),

    #[error("mqtt error occured")]
    MqttError {
        #[from]
        source: mqtt::Error,
        backtrace: Backtrace,
    },

    #[error("serial readed reached unexpected end")]
    EndOfReader(),
}
