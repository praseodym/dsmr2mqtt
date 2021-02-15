# DSMR to MQTT
Reads out a dutch smart meter using the DSMRv5 protocol 
and publishes some of the stats out to an mqtt broker.

## DSMRv5 Parser
For parsing the data telegrams I use a modified version of the `dsmr5` crate,
you can view its source [here](github.com/NULLx76/dsmr5)
