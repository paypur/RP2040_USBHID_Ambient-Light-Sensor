#include "als_hid_sensor.h"
#include "bsp/board.h"
#include "hardware/irq.h"
#include "pico/time.h"
#include "rust_bindings.h"
#include "tusb.h"

// Define HID report types manually since they might not be available
#define HID_REPORT_TYPE_INPUT       1
#define HID_REPORT_TYPE_OUTPUT      2
#define HID_REPORT_TYPE_FEATURE     3

// Global sensor state
volatile sensor_state_t g_sensor_state = {
    .power_state = DEFAULT_POWER_STATE,
    .reporting_state = DEFAULT_REPORTING_STATE,
    .report_interval = DEFAULT_REPORT_INTERVAL,
    .illuminance = 0,
    .last_report_time = 0,
    .feature_report_updated = false
};

// Timer for periodic reports
volatile repeating_timer_t report_timer;

// Volatile flag for timer interrupt
volatile bool timer_triggered = false;

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
// TinyUSB HID Callbacks (Running in interrupt context)
//--------------------------------------------------------------------+

// Invoked when received SET_REPORT control request or
// received data on OUT endpoint ( Report ID = 0, Type = 0 )
void tud_hid_set_report_cb(uint8_t instance, uint8_t report_id, hid_report_type_t report_type, uint8_t const* buffer, uint16_t bufsize) {
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

    // ReSharper disable once CppDFAEndlessLoop
    while (true) {
        tud_task(); // TinyUSB device task
        sensor_task();
        
        // Small delay to prevent overwhelming the system
        sleep_ms(1);
    }
}
