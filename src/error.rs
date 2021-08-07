use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("serial connection failed")]
    SerialError {
        #[from]
        source: serial::Error
    },

    #[error("parsing dsmr failed")]
    Dsmr5Error(dsmr5::Error),

    #[error("mqtt error occured")]
    MqttError {
        #[from]
        source: rumqttc::ClientError
    },

    #[error("serial readed reached unexpected end")]
    EndOfReader(),
}
