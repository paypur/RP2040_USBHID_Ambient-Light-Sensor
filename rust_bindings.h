#ifndef ALS_HID_SENSOR_RUST_BINDINGS_H
#define ALS_HID_SENSOR_RUST_BINDINGS_H
#include "hardware/adc.h"

uint16_t adc_read_rs() {
    return adc_read();
}

#endif //ALS_HID_SENSOR_RUST_BINDINGS_H
