use std::{iter::FromIterator};

use dsmr5::{
    types::{OctetString, UFixedDouble},
    Line, Tariff, OBIS,
};

use mqtt::Message;
use paho_mqtt as mqtt;

#[derive(Debug, Default)]
pub struct RawInstantaneousActivePowerAggregate {
    line1: Option<UFixedDouble>,
    line2: Option<UFixedDouble>,
    line3: Option<UFixedDouble>
}

impl RawInstantaneousActivePowerAggregate {
    pub fn sum(self) -> Option<f64> {
        *&[self.line1, self.line2, self.line3].iter().fold(Some(0.0), |acc, line|acc.map(|acc| line.as_ref().map(|line | acc + f64::from(line))).flatten())
    }
}

#[derive(Debug, Default)]
pub struct RawReport<'a> {
    pub active_tariff: Option<OctetString<'a>>,

    pub electricity_used_t1: Option<UFixedDouble>,
    pub electricity_used_t2: Option<UFixedDouble>,

    pub electricity_delivered_t1: Option<UFixedDouble>,
    pub electricity_delivered_t2: Option<UFixedDouble>,

    pub current_electricity_usage: Option<UFixedDouble>,
    pub current_electricity_delivery: Option<UFixedDouble>,

    pub instantaneous_active_power_plus: RawInstantaneousActivePowerAggregate,
    pub instantaneous_active_power_negative: RawInstantaneousActivePowerAggregate,

    pub hourly_gas_meter_reading: Option<UFixedDouble>,
}

impl<'a> FromIterator<OBIS<'a>> for RawReport<'a> {
    fn from_iter<I: IntoIterator<Item = OBIS<'a>>>(iter: I) -> Self {
        iter.into_iter().fold(RawReport::default(), |acc, object| {
            match object {
                OBIS::TariffIndicator(indicator) => RawReport{active_tariff: Some(indicator), ..acc},
                OBIS::MeterReadingTo(Tariff::Tariff1, value) => RawReport{electricity_used_t1: Some(value), ..acc},
                OBIS::MeterReadingTo(Tariff::Tariff2, value) => RawReport{electricity_used_t2: Some(value), ..acc},
                OBIS::MeterReadingBy(Tariff::Tariff1, value) => RawReport{electricity_delivered_t1: Some(value), ..acc},
                OBIS::MeterReadingBy(Tariff::Tariff2, value) => RawReport{electricity_delivered_t2: Some(value), ..acc},
                OBIS::PowerDelivered(value) => RawReport{current_electricity_usage: Some(value), ..acc},
                OBIS::PowerReceived(value) => RawReport{current_electricity_delivery: Some(value), ..acc},
                OBIS::InstantaneousActivePowerPlus(line, value) => match line {
                    Line::Line1 => RawReport{instantaneous_active_power_plus: RawInstantaneousActivePowerAggregate{line1: Some(value), ..acc.instantaneous_active_power_plus}, ..acc},
                    Line::Line2 => RawReport{instantaneous_active_power_plus: RawInstantaneousActivePowerAggregate{line2: Some(value), ..acc.instantaneous_active_power_plus}, ..acc},
                    Line::Line3 => RawReport{instantaneous_active_power_plus: RawInstantaneousActivePowerAggregate{line3: Some(value), ..acc.instantaneous_active_power_plus}, ..acc},
                },
                OBIS::InstantaneousActivePowerNeg(line, value) => match line {
                    Line::Line1 => RawReport{instantaneous_active_power_negative: RawInstantaneousActivePowerAggregate{line1: Some(value), ..acc.instantaneous_active_power_negative}, ..acc},
                    Line::Line2 => RawReport{instantaneous_active_power_negative: RawInstantaneousActivePowerAggregate{line2: Some(value), ..acc.instantaneous_active_power_negative}, ..acc},
                    Line::Line3 => RawReport{instantaneous_active_power_negative: RawInstantaneousActivePowerAggregate{line3: Some(value), ..acc.instantaneous_active_power_negative}, ..acc},
                },
                OBIS::GasMeterReading(_, value) => RawReport{hourly_gas_meter_reading: Some(value), ..acc},
                _ => acc
            }
        })
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub active_tariff: Option<Tariff>,

    pub electricity_used_t1: Option<f64>,
    pub electricity_used_t2: Option<f64>,

    pub electricity_delivered_t1: Option<f64>,
    pub electricity_delivered_t2: Option<f64>,

    pub current_electricity_usage: Option<f64>,
    pub current_electricity_delivery: Option<f64>,
    pub current_electricity_draw: Option<f64>,

    pub instantaneous_active_power_plus: Option<f64>,
    pub instantaneous_active_power_negative: Option<f64>,

    pub hourly_gas_meter_reading: Option<f64>,
}

impl Report {
    fn octet_to_tariff<'a>(o: OctetString<'a>) -> Option<Tariff> {
        let yeet: Result<Vec<_>, _> = o.as_octets().collect();
        let k: Result<u8, _> = yeet.map(|v| v.into_iter().sum());
    
        match k {
            Ok(1) => Some(Tariff::Tariff1),
            Ok(2) => Some(Tariff::Tariff2),
            _ => None
        }
    }

    fn build_f64_message(topic: String, qos: i32, value: f64) -> mqtt::Message {
        mqtt::MessageBuilder::new()
            .topic(topic)
            .payload(format!("{}", value))
            .qos(qos)
            .finalize()
    }

    pub fn to_mqtt_messages(self, topic: String, qos: i32) -> Vec<mqtt::Message> {
        let mut messages = Vec::with_capacity(10);

        if let Some(tariff) = self.active_tariff {
            let payload = match tariff {
                Tariff::Tariff1 => "1",
                Tariff::Tariff2 => "2",
            };

            let msg = mqtt::MessageBuilder::new()
                .topic(format!("{}/ELECTRICITY_ACTIVE_TARIFF", topic))
                .payload(payload)
                .qos(qos)
                .finalize();

            messages.push(msg);
        }

        if let Some(value) = self.electricity_delivered_t1 {
            messages.push(Self::build_f64_message(format!("{}/ELECTRICITY_DELIVERED_TARIFF_1", topic), qos, value));
        }

        if let Some(value) = self.electricity_delivered_t2 {
            messages.push(Self::build_f64_message(format!("{}/ELECTRICITY_DELIVERED_TARIFF_2", topic), qos, value));
        }

        if let Some(value) = self.electricity_used_t1 {
            messages.push(Self::build_f64_message(format!("{}/ELECTRICITY_USED_TARIFF_1", topic), qos, value));
        }

        if let Some(value) = self.electricity_used_t2 {
            messages.push(Self::build_f64_message(format!("{}/ELECTRICITY_USED_TARIFF_2", topic), qos, value));
        }

        if let Some(value) = self.current_electricity_usage {
            messages.push(Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_USAGE", topic), qos, value));
        }

        if let Some(value) = self.current_electricity_delivery {
            messages.push(Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_DELIVERY", topic), qos, value));
        }

        if let Some(value) = self.current_electricity_draw {
            messages.push(Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_DRAW", topic), qos, value));
        }

        if let Some(value) = self.instantaneous_active_power_plus {
            messages.push(Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_POSITIVE", topic), qos, value));
        }

        if let Some(value) = self.instantaneous_active_power_negative {
            messages.push(Self::build_f64_message(format!("{}/CURRENT_ELECTRICITY_NEGATIVE", topic), qos, value));
        }

        if let Some(value) = self.hourly_gas_meter_reading {
            messages.push(Self::build_f64_message(format!("{}/HOURLY_GAS_METER_READING", topic), qos, value));
        }

        messages
    }
}

impl<'a> From<RawReport<'a>> for Report {
    fn from(report: RawReport<'a>) -> Self {
        let usage = report.current_electricity_usage.as_ref().map(f64::from);
        let delivery = report.current_electricity_delivery.as_ref().map(f64::from);
        let draw = usage.map(|usage| delivery.map(|delivery| usage + delivery)).flatten();

        Report{
            active_tariff: report.active_tariff.map(Report::octet_to_tariff).flatten(),
            electricity_delivered_t1: report.electricity_delivered_t1.as_ref().map(f64::from),
            electricity_delivered_t2: report.electricity_delivered_t2.as_ref().map(f64::from),
            electricity_used_t1: report.electricity_used_t1.as_ref().map(f64::from),
            electricity_used_t2: report.electricity_used_t2.as_ref().map(f64::from),
            current_electricity_usage: usage,
            current_electricity_delivery: delivery,
            current_electricity_draw: draw,
            instantaneous_active_power_plus: report.instantaneous_active_power_plus.sum(),
            instantaneous_active_power_negative: report.instantaneous_active_power_negative.sum(),
            hourly_gas_meter_reading: report.hourly_gas_meter_reading.as_ref().map(f64::from),
        }
    }
}

impl<'a> FromIterator<OBIS<'a>> for Report {
    fn from_iter<I: IntoIterator<Item = OBIS<'a>>>(iter: I) -> Self {
        RawReport::from_iter(iter).into()
    }
}

#[cfg(test)]
mod test {
    use dsmr5::types::OctetString;

    #[test]
    fn test_parse_tarrif() {
        let t = OctetString::parse("(01)", 2).unwrap();
        let octets: u8 = t.as_octets().map(|o| o.unwrap()).sum();
        assert_eq!(1, octets);

        let t = OctetString::parse("(02)", 2).unwrap();
        let octets: u8 = t.as_octets().map(|o| o.unwrap()).sum();
        assert_eq!(2, octets);
    }
}
