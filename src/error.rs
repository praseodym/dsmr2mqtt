use paho_mqtt as mqtt;

#[derive(Debug)]
pub enum Error {
    SerialError(serial::Error),
    DSMR5Error(dsmr5::Error),
    MqttError(mqtt::Error),
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

impl From<mqtt::Error> for Error {
    fn from(e: mqtt::Error) -> Self {
        Error::MqttError(e)
    }
}
