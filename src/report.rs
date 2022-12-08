use std::iter::FromIterator;

use chrono::{DateTime, Local};
use dsmr5::{types::OctetString, Tariff, OBIS};
use serde::Serialize;
use serde_with::EnumMap;

use crate::mqtt;

#[derive(Serialize)]
#[serde(remote = "Tariff")]
pub enum TariffDef {
    Tariff1 = 0,
    Tariff2 = 1,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Measurement {
    #[serde(with = "TariffDef")]
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
    fn parse_tariff(o: &OctetString) -> Option<Measurement> {
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
        mqtt::Message::new(
            format!("{}/{}", prefix, self.to_topic()),
            rumqttc::QoS::AtMostOnce,
            true,
            self.to_vec(),
        )
    }
}

#[serde_with::serde_as]
#[derive(Debug, PartialEq, Serialize)]
pub struct Measurements {
    timestamp: DateTime<Local>,
    #[serde_as(as = "EnumMap")]
    #[serde(flatten)]
    measurements: Vec<Measurement>,
}

impl Measurements {
    pub fn into_mqtt_messages(self, prefix: String) -> Box<dyn Iterator<Item = mqtt::Message>> {
        Box::new(
            self.measurements
                .into_iter()
                .map(move |m| m.to_mqtt_messsage(&prefix)),
        )
    }

    pub fn report(&self) -> String {
        let mut tariff: u8 = 0;
        let mut used_t1: f64 = 0.0;
        let mut used_t2: f64 = 0.0;
        for m in &self.measurements {
            match m {
                Measurement::ActiveTariff(t) => match t {
                    Tariff::Tariff1 => tariff = 1,
                    Tariff::Tariff2 => tariff = 2,
                },
                Measurement::ElectricityUsedT1(e) => used_t1 = *e,
                Measurement::ElectricityUsedT2(e) => used_t2 = *e,
                _ => (),
            }
        }
        format!(
            "{}\t{}\t{}\t{}",
            Local::now().to_rfc3339(),
            tariff as u8,
            used_t1,
            used_t2
        )
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
                    if let Some(m) = Measurement::parse_tariff(&value) {
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

        res.push(Measurement::CurrentElectricityDraw(draw));
        res.push(Measurement::InstantaneousActivePowerPositive(pos));
        res.push(Measurement::InstantaneousActivePowerNegative(neg));

        Measurements {
            timestamp: Local::now(),
            measurements: res,
        }
    }
}

#[cfg(test)]
mod test {
    use dsmr5::{types::UFixedDouble, Line};

    use super::*;

    #[test]
    fn test_parse_tarrif() {
        let t = OctetString::parse("(01)", 2).unwrap();
        let m = Measurement::parse_tariff(&t).unwrap();
        assert!(matches!(m, Measurement::ActiveTariff(Tariff::Tariff1)));

        let t = OctetString::parse("(02)", 2).unwrap();
        let m = Measurement::parse_tariff(&t).unwrap();
        assert!(matches!(m, Measurement::ActiveTariff(Tariff::Tariff2)));

        let t = OctetString::parse("(04)", 2).unwrap();
        assert!(Measurement::parse_tariff(&t).is_none());
    }

    #[test]
    fn test_to_vec() {
        // Tariff
        assert_eq!(
            "1".as_bytes().to_vec(),
            Measurement::ActiveTariff(Tariff::Tariff1).to_vec()
        );
        assert_eq!(
            "2".as_bytes().to_vec(),
            Measurement::ActiveTariff(Tariff::Tariff2).to_vec()
        );

        // The Rest
        let num = 42.0;
        let numv = num.to_string().as_bytes().to_vec();

        assert_eq!(numv, Measurement::ElectricityUsedT1(num).to_vec());
        assert_eq!(numv, Measurement::ElectricityUsedT2(num).to_vec());
        assert_eq!(numv, Measurement::ElectricityDeliveredT1(num).to_vec());
        assert_eq!(numv, Measurement::ElectricityDeliveredT2(num).to_vec());
        assert_eq!(numv, Measurement::CurrentElectricityUsage(num).to_vec());
        assert_eq!(numv, Measurement::CurrentElectricityDelivery(num).to_vec());
        assert_eq!(numv, Measurement::CurrentElectricityDraw(num).to_vec());
        assert_eq!(
            numv,
            Measurement::InstantaneousActivePowerPositive(num).to_vec()
        );
        assert_eq!(
            numv,
            Measurement::InstantaneousActivePowerNegative(num).to_vec()
        );
        assert_eq!(numv, Measurement::HourlyGasMeterReading(num).to_vec());
    }

    #[test]
    fn test_obis_iterator() {
        let double1 = UFixedDouble::parse("(42.24)", 4, 2).unwrap();
        let double2 = UFixedDouble::parse("(236.1)", 4, 1).unwrap();
        let double3 = UFixedDouble::parse("(13.37)", 4, 2).unwrap();

        let f64_1 = 42.24;
        let f64_2 = 236.1;
        let f64_3 = 13.37;

        let tst = dsmr5::types::TST::parse("(190320181003W)").unwrap();

        let obis: Vec<OBIS<'_>> = vec![
            OBIS::TariffIndicator(OctetString::parse("(01)", 2).unwrap()),
            OBIS::MeterReadingTo(Tariff::Tariff1, double1.clone()),
            OBIS::MeterReadingTo(Tariff::Tariff2, double2.clone()),
            OBIS::MeterReadingBy(Tariff::Tariff1, double3.clone()),
            OBIS::MeterReadingBy(Tariff::Tariff2, double1.clone()),
            OBIS::InstantaneousActivePowerPlus(Line::Line1, double1.clone()),
            OBIS::InstantaneousActivePowerPlus(Line::Line2, double2.clone()),
            OBIS::InstantaneousActivePowerPlus(Line::Line3, double3.clone()),
            OBIS::InstantaneousActivePowerNeg(Line::Line1, double2.clone()),
            OBIS::InstantaneousActivePowerNeg(Line::Line2, double2.clone()),
            OBIS::InstantaneousActivePowerNeg(Line::Line3, double3.clone()),
            OBIS::PowerDelivered(double2),
            OBIS::PowerReceived(double1),
            OBIS::GasMeterReading(tst, double3),
        ];

        let expected = vec![
            Measurement::ActiveTariff(Tariff::Tariff1),
            Measurement::ElectricityUsedT1(f64_1),
            Measurement::ElectricityUsedT2(f64_2),
            Measurement::ElectricityDeliveredT1(f64_3),
            Measurement::ElectricityDeliveredT2(f64_1),
            // Sum of all lines
            Measurement::InstantaneousActivePowerPositive(f64_1 + f64_2 + f64_3),
            Measurement::InstantaneousActivePowerNegative(f64_2 + f64_2 + f64_3),
            Measurement::CurrentElectricityUsage(f64_2),
            Measurement::CurrentElectricityDelivery(f64_1),
            // Usage - Delivery
            Measurement::CurrentElectricityDraw(f64_2 - f64_1),
            Measurement::HourlyGasMeterReading(f64_3),
        ];

        let actual: Measurements = obis.into_iter().collect();

        assert_eq!(expected.clone().len(), actual.measurements.len());

        for item in expected {
            actual.measurements.iter().any(|el| el == &item);
        }
    }
}
