#ifndef ALS_HID_SENSOR_RUST_BINDINGS_H
#define ALS_HID_SENSOR_RUST_BINDINGS_H

uint16_t adc_read_rs() {
    return adc_read();
}

bool add_repeating_timer_ms_rs(int32_t delay_ms, repeating_timer_callback_t callback, void *user_data, repeating_timer_t *out) {
    return add_repeating_timer_ms(delay_ms, callback, user_data, out);
}

bool cancel_repeating_timer_rs(repeating_timer_t *timer) {
    return cancel_repeating_timer(timer);
}

bool tud_hid_ready_rs() {
    return tud_hid_ready();
}

bool tud_hid_report_rs(uint8_t report_id, void const* report, uint16_t len) {
    return tud_hid_report(report_id, report, len);
}

#endif //ALS_HID_SENSOR_RUST_BINDINGS_H
