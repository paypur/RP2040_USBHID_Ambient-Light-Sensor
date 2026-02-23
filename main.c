#include "als_hid_sensor.h"
#include "bsp/board.h"
#include "hardware/irq.h"
#include "pico/time.h"
#include "tusb.h"

// Define HID report types manually since they might not be available
#define HID_REPORT_TYPE_INPUT       1
#define HID_REPORT_TYPE_OUTPUT      2
#define HID_REPORT_TYPE_FEATURE     3

// Global sensor state
static sensor_state_t g_sensor_state = {
    .power_state = DEFAULT_POWER_STATE,
    .reporting_state = DEFAULT_REPORTING_STATE,
    .report_interval = DEFAULT_REPORT_INTERVAL,
    .illuminance = 0,
    .last_report_time = 0,
    .feature_report_updated = false
};

// Timer for periodic reports
static repeating_timer_t report_timer;

// Volatile flag for timer interrupt
static volatile bool timer_triggered = false;

//--------------------------------------------------------------------+
// Hardware Initialization
//--------------------------------------------------------------------+

void gpio_init_pins(void) {
    // Initialize Pin 28 (HIGH) and Pin 27 (LOW) as per spec
    gpio_init(PIN_28_HIGH);
    gpio_set_dir(PIN_28_HIGH, GPIO_OUT);
    gpio_put(PIN_28_HIGH, true);

    gpio_init(PIN_27_LOW);
    gpio_set_dir(PIN_27_LOW, GPIO_OUT);
    gpio_put(PIN_27_LOW, false);
}

void adc_init_sensor(void) {
    adc_init();
    adc_gpio_init(ADC_PIN_GP26);
    adc_select_input(ADC_CHANNEL);
}



//--------------------------------------------------------------------+
// Sensor Reading
//--------------------------------------------------------------------+

uint16_t read_illuminance(void) {
    uint16_t adc_value = adc_read();
    // Scale using y = 0.6294*x - 117.47, clamp to uint16_t range
    float y = 0.1611f * (float)adc_value;
    if (y < 0.0f) y = 0.0f;
    if (y > 65535.0f) y = 65535.0f;
    return (uint16_t)(y + 0.5f);
}

//--------------------------------------------------------------------+
// HID Report Functions
//--------------------------------------------------------------------+

void send_input_report(uint16_t illuminance, sensor_event_t event) {
    if (!tud_hid_ready()) return;

    uint8_t report[3];
    // Pack data: 16-bit illuminance + 3-bit event + 5-bit padding
    uint32_t data = (illuminance & 0xFFFF) | ((event & 0x7) << 16);
    
    report[0] = data & 0xFF;        // Illuminance bits 0-7
    report[1] = (data >> 8) & 0xFF; // Illuminance bits 8-15
    report[2] = (data >> 16) & 0xFF; // Event bits + padding

    tud_hid_report(REPORT_ID_INPUT, report, sizeof(report));
}

void send_feature_report(sensor_state_t *state) {
    if (!tud_hid_ready()) return;

    uint8_t report[3];
    // Pack: reporting_state(2) + power_state(3) + report_interval(12) + padding(7)
    uint32_t data = (state->reporting_state & 0x3) |
                   ((state->power_state & 0x7) << 2) |
                   ((state->report_interval & 0xFFF) << 5);

    report[0] = data & 0xFF;
    report[1] = (data >> 8) & 0xFF;
    report[2] = (data >> 16) & 0xFF;

    tud_hid_report(REPORT_ID_FEATURE, report, sizeof(report));
}

bool decode_feature_report(const uint8_t *report, sensor_state_t *state) {
    uint32_t data = report[0] | (report[1] << 8) | (report[2] << 16);
    
    uint8_t received_reporting = data & 0x3;
    uint8_t received_power = (data >> 2) & 0x7;
    uint16_t received_interval = (data >> 5) & 0xFFF;

    bool changed = false;
    
    if (received_reporting != 0 && received_reporting != state->reporting_state) {
        state->reporting_state = received_reporting;
        changed = true;
    }
    
    if (received_power != 0 && received_power != state->power_state) {
        state->power_state = received_power;
        changed = true;
    }
    
    if (received_interval != 0 && received_interval != state->report_interval) {
        state->report_interval = received_interval;
        changed = true;
    }

    return changed;
}

//--------------------------------------------------------------------+
// Timer Interrupt Handler
//--------------------------------------------------------------------+

bool timer_callback(repeating_timer_t *rt) {
    (void)rt;
    timer_triggered = true;
    return true; // Keep repeating
}

//--------------------------------------------------------------------+
// TinyUSB HID Callbacks (Running in interrupt context)
//--------------------------------------------------------------------+

// Invoked when received GET_REPORT control request
// Application must fill buffer report's content and return its length.
// Return zero will cause the stack to STALL request
uint16_t tud_hid_get_report_cb(uint8_t instance, uint8_t report_id, uint8_t report_type, uint8_t* buffer, uint16_t reqlen) {
    (void) instance;
    (void) reqlen;

    if (report_type == HID_REPORT_TYPE_INPUT && report_id == REPORT_ID_INPUT) {
        // Return current input report with fresh sensor data
        uint16_t current_illuminance = read_illuminance();
        g_sensor_state.illuminance = current_illuminance;  // Update cached value
        
        uint32_t data = (current_illuminance & 0xFFFF) | ((DEFAULT_SENSOR_EVENT & 0x7) << 16);
        
        buffer[0] = data & 0xFF;        // Illuminance bits 0-7
        buffer[1] = (data >> 8) & 0xFF; // Illuminance bits 8-15
        buffer[2] = (data >> 16) & 0xFF; // Event bits + padding
        
        return 3;
    }
    else if (report_type == HID_REPORT_TYPE_FEATURE && report_id == REPORT_ID_FEATURE) {
        // Return current feature report
        uint32_t data = (g_sensor_state.reporting_state & 0x3) |
                       ((g_sensor_state.power_state & 0x7) << 2) |
                       ((g_sensor_state.report_interval & 0xFFF) << 5);

        buffer[0] = data & 0xFF;
        buffer[1] = (data >> 8) & 0xFF;
        buffer[2] = (data >> 16) & 0xFF;
        
        return 3;
    }

    return 0;
}

// Invoked when received SET_REPORT control request or
// received data on OUT endpoint ( Report ID = 0, Type = 0 )
void tud_hid_set_report_cb(uint8_t instance, uint8_t report_id, uint8_t report_type, uint8_t const* buffer, uint16_t bufsize) {
    (void) instance;
    
    if (report_type == HID_REPORT_TYPE_FEATURE && report_id == REPORT_ID_FEATURE && bufsize == 3) {
        // Process feature report in interrupt context
        bool changed = decode_feature_report(buffer, &g_sensor_state);
        if (changed) {
            g_sensor_state.feature_report_updated = true;
        }
    }
}

//--------------------------------------------------------------------+
// Main Sensor Task
//--------------------------------------------------------------------+

void sensor_task(void) {
    // Handle feature report updates
    if (g_sensor_state.feature_report_updated) {
        g_sensor_state.feature_report_updated = false;
        send_feature_report(&g_sensor_state);
        
        // Restart timer with new interval if needed
        cancel_repeating_timer(&report_timer);
        if (g_sensor_state.power_state == POWER_STATE_FULL && 
            g_sensor_state.reporting_state == REPORTING_ALL_EVENTS) {
            add_repeating_timer_ms(g_sensor_state.report_interval, timer_callback, NULL, &report_timer);
        }
    }

    // Handle periodic reporting
    if (timer_triggered) {
        timer_triggered = false;
        
        if (g_sensor_state.power_state == POWER_STATE_FULL && 
            g_sensor_state.reporting_state == REPORTING_ALL_EVENTS) {
            
            g_sensor_state.illuminance = read_illuminance();
            send_input_report(g_sensor_state.illuminance, DEFAULT_SENSOR_EVENT);
        }
    }

    // Handle state transitions and edge cases
    if (g_sensor_state.reporting_state == REPORTING_ALL_EVENTS && 
        g_sensor_state.power_state != POWER_STATE_FULL) {
        
        g_sensor_state.power_state = POWER_STATE_FULL;
        send_feature_report(&g_sensor_state);
        
        g_sensor_state.illuminance = read_illuminance();
        send_input_report(g_sensor_state.illuminance, DEFAULT_SENSOR_EVENT);
    }
}

//--------------------------------------------------------------------+
// Main Function
//--------------------------------------------------------------------+

int main(void) {
    // Initialize board and USB
    board_init();
    tusb_init();

    // Initialize hardware
    gpio_init_pins();
    adc_init_sensor();
    
    // Send initial reports to prime the USB buffers
    send_feature_report(&g_sensor_state);
    g_sensor_state.illuminance = read_illuminance();
    send_input_report(g_sensor_state.illuminance, DEFAULT_SENSOR_EVENT);

    while (1) {
        tud_task(); // TinyUSB device task
        sensor_task();
        
        // Small delay to prevent overwhelming the system
        sleep_ms(1);
    }

    return 0;
}