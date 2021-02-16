use std::iter::FromIterator;

use dsmr5::{types::OctetString, Tariff, OBIS};
use crate::mqtt;

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

    // Returns the bytes of the _string_ representation of the measurement
    fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::ActiveTariff(Tariff::Tariff1) => "1".as_bytes().to_vec(),
            Self::ActiveTariff(Tariff::Tariff2) => "2".as_bytes().to_vec(),
            Self::ElectricityUsedT1(v)
            | Self::ElectricityUsedT2(v)
            | Self::ElectricityDeliveredT1(v)
            | Self::ElectricityDeliveredT2(v)
            | Self::CurrentElectricityUsage(v)
            | Self::CurrentElectricityDelivery(v)
            | Self::CurrentElectricityDraw(v)
            | Self::InstantaneousActivePowerPositive(v)
            | Self::InstantaneousActivePowerNegative(v)
            | Self::HourlyGasMeterReading(v) => v.to_string().as_bytes().to_vec(),
        }
    }

    fn to_topic(&self) -> &str {
        match self {
            Self::ActiveTariff(_) => "ELECTRICITY_ACTIVE_TARIFF",
            Self::ElectricityUsedT1(_) => "ELECTRICITY_USED_TARIFF_1",
            Self::ElectricityUsedT2(_) => "ELECTRICITY_USED_TARIFF_2",
            Self::ElectricityDeliveredT1(_) => "ELECTRICITY_DELIVERED_TARIFF_1",
            Self::ElectricityDeliveredT2(_) => "ELECTRICITY_DELIVERED_TARIFF_2",
            Self::CurrentElectricityUsage(_) => "CURRENT_ELECTRICITY_USAGE",
            Self::CurrentElectricityDelivery(_) => "CURRENT_ELECTRICITY_DELIVERY",
            Self::CurrentElectricityDraw(_) => "CURRENT_ELECTRICITY_DRAW",
            Self::InstantaneousActivePowerPositive(_) => "INSTANTANEOUS_ACTIVE_POWER_POSITIVE",
            Self::InstantaneousActivePowerNegative(_) => "INSTANTANEOUS_ACTIVE_POWER_NEGATIVE",
            Self::HourlyGasMeterReading(_) => "HOURLY_GAS_METER_READING",
        }
    }

    pub fn to_mqtt_messsage(&self, prefix: &str) -> mqtt::Message {
        mqtt::Message::new(format!("{}/{}", prefix, self.to_topic()), rumqttc::QoS::AtMostOnce, false,  self.to_vec())
    }
}

pub struct Measurements(Vec<Measurement>);

impl Measurements {
    pub fn to_mqtt_messages(self, prefix: String) -> Box<dyn Iterator<Item = mqtt::Message>> {
        Box::new(self.0.into_iter().map(move |m| m.to_mqtt_messsage(&prefix)))
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
