use std::{iter::FromIterator, todo};

use dsmr5::{
    types::{OctetString, UFixedDouble},
    Line, Tariff, OBIS,
};

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

#[derive(Debug)]
pub struct Report {
    active_tariff: Option<Tariff>,

    electricity_used_t1: Option<f64>,
    electricity_used_t2: Option<f64>,

    electricity_delivered_t1: Option<f64>,
    electricity_delivered_t2: Option<f64>,

    current_electricity_usage: Option<f64>,
    current_electricity_delivery: Option<f64>,

    instantaneous_active_power_plus: Option<f64>,
    instantaneous_active_power_negative: Option<f64>,

    hourly_gas_meter_reading: Option<f64>,
}
