#ifndef ALS_HID_SENSOR_H
#define ALS_HID_SENSOR_H

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "hardware/gpio.h"
#include "hardware/adc.h"
#include "hardware/timer.h"
#include "tusb.h"

#define PIN_28_HIGH         28  // Must be HIGH for TEMT6000 Sensor
#define PIN_27_LOW          27  // Must be LOW for TEMT6000 Sensor
#define ADC_PIN_GP26        26  // ADC input for light sensor

// ADC configuration
#define ADC_CHANNEL         0   // GP26 is ADC channel 0

// HID Report IDs
#define REPORT_ID_INPUT     1   // Input report (illuminance data)
#define REPORT_ID_FEATURE   2   // Feature report (settings)

// Power States 
typedef enum {
    POWER_STATE_UNDEFINED = 1,
    POWER_STATE_FULL      = 2,  // D0
    POWER_STATE_LOW       = 3,  // D1
    POWER_STATE_STANDBY   = 4,  // D2
    POWER_STATE_SLEEP     = 5,  // D3
    POWER_STATE_OFF       = 6   // D4
} power_state_t;

// Reporting States
typedef enum {
    REPORTING_NO_EVENTS   = 1,
    REPORTING_ALL_EVENTS  = 2
} reporting_state_t;

// Sensor Events
typedef enum {
    SENSOR_EVENT_UNKNOWN          = 1,
    SENSOR_EVENT_STATE_CHANGED    = 2,
    SENSOR_EVENT_PROPERTY_CHANGED = 3,
    SENSOR_EVENT_DATA_UPDATED     = 4,
    SENSOR_EVENT_POLL_RESPONSE    = 5,
    SENSOR_EVENT_CHANGE_SENSITIVITY = 6
} sensor_event_t;


// Global sensor state structure
typedef struct {
    power_state_t power_state;
    reporting_state_t reporting_state;
    uint16_t report_interval;       // in milliseconds
    uint16_t illuminance;           // 0-65535 lux
    absolute_time_t last_report_time;
    bool feature_report_updated;
} sensor_state_t;

// Function prototypes
void gpio_init_pins(void);
void adc_init_sensor(void);
uint16_t read_illuminance(void);
void send_input_report(uint16_t illuminance, sensor_event_t event);
void send_feature_report();
bool decode_feature_report(const uint8_t *report);
bool timer_callback(repeating_timer_t *rt);
void sensor_task(void);

// Default values
#define DEFAULT_POWER_STATE         POWER_STATE_OFF
#define DEFAULT_REPORTING_STATE     REPORTING_NO_EVENTS
#define DEFAULT_REPORT_INTERVAL     100  // ms
#define DEFAULT_SENSOR_EVENT        SENSOR_EVENT_DATA_UPDATED

#endif // ALS_HID_SENSOR_H