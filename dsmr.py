from dsmr_parser import telegram_specifications
from dsmr_parser.clients import SerialReader, SERIAL_SETTINGS_V5, SERIAL_SETTINGS_V4
from dsmr_parser.objects import CosemObject, MBusObject, Telegram
from dsmr_parser.parsers import TelegramParser
import os
from enum import Enum
import paho.mqtt.client as mqtt
from dataclasses import dataclass


class DSMR_VERSION(Enum):
    """An Enum containing the various supported dsmr versions"""
    V4 = 4
    V5 = 5


def create_serial_reader(version: DSMR_VERSION, device: str) -> SerialReader:
    """Creates a SerialReader for the given dsmr version and device. Raises an exception if dsmr version is wrong"""
    # Add support for more DSMR versions here

    if version == DSMR_VERSION.V5:
        return SerialReader(
            device=device,
            serial_settings=SERIAL_SETTINGS_V5,
            telegram_specification=telegram_specifications.V5
        )
    elif version == DSMR_VERSION.V4:
        return SerialReader(
            device=device,
            serial_settings=SERIAL_SETTINGS_V4,
            telegram_specification=telegram_specifications.V4
        )
    else:
        raise ValueError("Invalid DSMR version")


if __name__ == "__main__":
    # Init serial
    serial_reader = create_serial_reader(DSMR_VERSION.V5, '/dev/ttyUSB1')

    # Connect to mqtt
    client = mqtt.Client()
    # connect(host, port, keepalive)
    client.connect("10.10.10.13", 1883, 60)

    for telegram in serial_reader.read_as_object():
        client.publish("dsmr/ELECTRICITY_ACTIVE_TARIFF", float(telegram.ELECTRICITY_ACTIVE_TARIFF.value))
        
        client.publish("dsmr/ELECTRICITY_USED_TARIFF_1", float(telegram.ELECTRICITY_USED_TARIFF_1.value)) # Low tariff
        client.publish("dsmr/ELECTRICITY_USED_TARIFF_2", float(telegram.ELECTRICITY_USED_TARIFF_2.value)) # High tariff

        client.publish("dsmr/ELECTRICITY_DELIVERED_TARIFF_1", float(telegram.ELECTRICITY_DELIVERED_TARIFF_1.value))
        client.publish("dsmr/ELECTRICITY_DELIVERED_TARIFF_2", float(telegram.ELECTRICITY_DELIVERED_TARIFF_2.value))

        client.publish("dsmr/CURRENT_ELECTRICITY_USAGE", float(telegram.CURRENT_ELECTRICITY_USAGE.value))
        client.publish("dsmr/CURRENT_ELECTRICITY_DELIVERY", float(telegram.CURRENT_ELECTRICITY_DELIVERY.value))
        
        INSTANTANEOUS_ACTIVE_POWER_POSITIVE = (telegram.INSTANTANEOUS_ACTIVE_POWER_L1_POSITIVE.value 
                                            +  telegram.INSTANTANEOUS_ACTIVE_POWER_L2_POSITIVE.value
                                            +  telegram.INSTANTANEOUS_ACTIVE_POWER_L3_POSITIVE.value)
        client.publish("dsmr/INSTANTANEOUS_ACTIVE_POWER_POSITIVE", float(INSTANTANEOUS_ACTIVE_POWER_POSITIVE))

        INSTANTANEOUS_ACTIVE_POWER_NEGATIVE = (telegram.INSTANTANEOUS_ACTIVE_POWER_L1_NEGATIVE.value 
                                            +  telegram.INSTANTANEOUS_ACTIVE_POWER_L2_NEGATIVE.value
                                            +  telegram.INSTANTANEOUS_ACTIVE_POWER_L3_NEGATIVE.value)
        client.publish("dsmr/INSTANTANEOUS_ACTIVE_POWER_NEGATIVE", float(INSTANTANEOUS_ACTIVE_POWER_NEGATIVE))

        client.publish("dsmr/HOURLY_GAS_METER_READING", float(telegram.HOURLY_GAS_METER_READING.value))
