use std::iter::FromIterator;

use dsmr5::{types::OctetString, Tariff, OBIS};
use paho_mqtt as mqtt;

#[derive(Debug)]
pub enum Measurement {
    ActiveTariff(Tariff),
    ElectricityUsedT1(f64),
    ElectricityUsedT2(f64),
    ElectricityDeliveredT1(f64),
    ElectricityDeliveredT2(f64),
    CurrentElectricityUsage(f64),
    CurrentElectricityDelivery(f64),
    CurrentElectricityDraw(f64),
    InstantaneousActivePowerPositive(f64),
    InstantaneousActivePowerNegative(f64),
    HourlyGasMeterReading(f64),
}

impl Measurement {
    fn parse_tariff<'a>(o: OctetString<'a>) -> Option<Measurement> {
        let xs: Result<Vec<_>, _> = o.as_octets().collect();
        let sum: Result<u8, _> = xs.map(|v| v.into_iter().sum());

        match sum {
            Ok(1) => Some(Measurement::ActiveTariff(Tariff::Tariff1)),
            Ok(2) => Some(Measurement::ActiveTariff(Tariff::Tariff2)),
            _ => None,
        }
    }
}

pub struct Measurements(Vec<Measurement>);

impl Measurements {
    fn build_f64_message(topic: String, qos: i32, value: f64) -> mqtt::Message {
        mqtt::MessageBuilder::new()
            .topic(topic)
            .payload(format!("{}", value))
            .qos(qos)
            .finalize()
    }

    pub fn to_mqtt_messages(self, topic: &str, qos: i32) -> Vec<mqtt::Message> {
        self.0
            .into_iter()
            .map(|m| match m {
                Measurement::ActiveTariff(t) => {
                    let payload = match t {
                        Tariff::Tariff1 => "1",
                        Tariff::Tariff2 => "2",
                    };

                    mqtt::MessageBuilder::new()
                        .topic(format!("{}/ELECTRICITY_ACTIVE_TARIFF", topic))
                        .payload(payload)
                        .qos(qos)
                        .finalize()
                }
                Measurement::ElectricityUsedT1(f) => {
                    Self::build_f64_message(format!("{}/ELECTRICITY_USED_TARIFF_1", topic), qos, f)
                }
                Measurement::ElectricityUsedT2(f) => {
                    Self::build_f64_message(format!("{}/ELECTRICITY_USED_TARIFF_2", topic), qos, f)
                }
                Measurement::ElectricityDeliveredT1(f) => Self::build_f64_message(
                    format!("{}/ELECTRICITY_DELIVERED_TARIFF_1", topic),
                    qos,
                    f,
                ),
                Measurement::ElectricityDeliveredT2(f) => Self::build_f64_message(
                    format!("{}/ELECTRICITY_DELIVERED_TARIFF_2", topic),
                    qos,
                    f,
                ),
                Measurement::CurrentElectricityUsage(f) => {
                    Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_USAGE", topic), qos, f)
                }
                Measurement::CurrentElectricityDelivery(f) => Self::build_f64_message(
                    format!("{}/CURRENT_ELECTRICITY_DELIVERY", topic),
                    qos,
                    f,
                ),
                Measurement::CurrentElectricityDraw(f) => {
                    Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_DRAW", topic), qos, f)
                }
                Measurement::InstantaneousActivePowerPositive(f) => Self::build_f64_message(
                    format!("{}/INSTANTANEOUS_ACTIVE_POWER_POSITIVE", topic),
                    qos,
                    f,
                ),
                Measurement::InstantaneousActivePowerNegative(f) => Self::build_f64_message(
                    format!("{}/INSTANTANEOUS_ACTIVE_POWER_NEGATIVE", topic),
                    qos,
                    f,
                ),
                Measurement::HourlyGasMeterReading(f) => {
                    Self::build_f64_message(format!("{}/HOURLY_GAS_METER_READING", topic), qos, f)
                }
            })
            .collect()
    }
}

impl<'a> FromIterator<OBIS<'a>> for Measurements {
    fn from_iter<I: IntoIterator<Item = OBIS<'a>>>(iter: I) -> Self {
        let mut res = Vec::with_capacity(10);

        let mut pos = 0.0;
        let mut neg = 0.0;
        let mut draw = 0.0;

        for object in iter {
            match object {
                OBIS::TariffIndicator(value) => {
                    if let Some(m) = Measurement::parse_tariff(value) {
                        res.push(m)
                    }
                }
                OBIS::MeterReadingTo(Tariff::Tariff1, value) => {
                    res.push(Measurement::ElectricityUsedT1(f64::from(&value)))
                }
                OBIS::MeterReadingTo(Tariff::Tariff2, value) => {
                    res.push(Measurement::ElectricityUsedT2(f64::from(&value)))
                }
                OBIS::MeterReadingBy(Tariff::Tariff1, value) => {
                    res.push(Measurement::ElectricityDeliveredT1(f64::from(&value)))
                }
                OBIS::MeterReadingBy(Tariff::Tariff2, value) => {
                    res.push(Measurement::ElectricityDeliveredT2(f64::from(&value)))
                }
                OBIS::PowerDelivered(value) => {
                    let f = f64::from(&value);
                    draw += f;
                    res.push(Measurement::CurrentElectricityUsage(f))
                }
                OBIS::PowerReceived(value) => {
                    let f = f64::from(&value);
                    draw -= f;
                    res.push(Measurement::CurrentElectricityDelivery(f))
                }
                OBIS::InstantaneousActivePowerPlus(_, value) => pos += f64::from(&value),
                OBIS::InstantaneousActivePowerNeg(_, value) => neg += f64::from(&value),
                OBIS::GasMeterReading(_, value) => {
                    res.push(Measurement::HourlyGasMeterReading(f64::from(&value)))
                }
                _ => {}
            }
        }

        res.push(Measurement::InstantaneousActivePowerPositive(pos));
        res.push(Measurement::InstantaneousActivePowerNegative(neg));
        res.push(Measurement::CurrentElectricityDraw(draw));

        Measurements(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_tarrif() {
        let t = OctetString::parse("(01)", 2).unwrap();
        let m = Measurement::parse_tariff(t).unwrap();
        assert!(matches!(m, Measurement::ActiveTariff(Tariff::Tariff1)));

        let t = OctetString::parse("(02)", 2).unwrap();
        let m = Measurement::parse_tariff(t).unwrap();
        assert!(matches!(m, Measurement::ActiveTariff(Tariff::Tariff2)));
    }
}
