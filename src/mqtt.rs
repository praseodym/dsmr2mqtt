use rumqttc::{self, QoS};

pub struct Message {
    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new<T, P>(topic: T, qos: QoS, retain: bool, payload: P) -> Self
    where
        T: Into<String>,
        P: Into<Vec<u8>>,
    {
        Self {
            topic: topic.into(),
            payload: payload.into(),
            qos,
            retain,
        }
    }

    pub async fn send(self, client: &mut rumqttc::AsyncClient) -> Result<(), rumqttc::ClientError>{
        client.publish(self.topic, self.qos, self.retain, self.payload).await
    }
}
