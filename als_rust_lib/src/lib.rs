#![no_std]

mod types;

use core::ffi::c_void;
use crate::types::*;
use core::panic::PanicInfo;
use core::ptr::null;
use core::slice;

unsafe extern "C" {
    pub static mut g_sensor_state: SensorState;
    pub static mut report_timer: RepeatingTimer;
    pub static mut timer_triggered: bool;

    fn adc_read_rs() -> u16;

    fn add_repeating_timer_ms_rs(delay_ms: i32, callback: extern "C" fn(*const RepeatingTimer) -> bool, user_data: *const c_void, out: *const RepeatingTimer) -> bool;
    fn cancel_repeating_timer_rs(timer: *const RepeatingTimer) -> bool;

    fn tud_hid_ready_rs() -> bool;
    fn tud_hid_report_rs(report_id: HIDReportID, report: *const c_void, len: u16) -> bool;
}

#[unsafe(no_mangle)]
pub extern "C" fn read_illuminance() -> u16 {
    let adc_value: u16 = unsafe { adc_read_rs() };
    // Scale using y = 0.6294*x - 117.47, clamp to uint16_t range
    let mut y: u32 = (adc_value as u32) * 1611u32 / 10000u32;
    y = y.min(u16::MAX as u32);
    y as u16
}

#[unsafe(no_mangle)]
pub extern "C" fn timer_callback(_rt: *const RepeatingTimer) -> bool {
    unsafe { timer_triggered = true };
    true // Keep repeating
}

// Invoked when received GET_REPORT control request
// Application must fill buffer report's content and return its length.
// Return zero will cause the stack to STALL request
#[allow(private_interfaces)]
#[unsafe(no_mangle)]
pub extern "C" fn tud_hid_get_report_cb(
    _instance: u8,
    report_id: HIDReportID,
    report_type: HIDReportType,
    buffer: *mut u8,
    req_len: u16,
) -> u16 {
    let slice = unsafe { slice::from_raw_parts_mut(buffer, req_len as usize) };

    if report_type == HIDReportType::Input && report_id == HIDReportID::Input {
        // Return current input report with fresh sensor data
        let current_illuminance: u16 = read_illuminance();
        unsafe { g_sensor_state.illuminance = current_illuminance; } // Update cached value

        let data: u32 = current_illuminance as u32 | ((SensorEvent::default() as u32) << 16);

        slice[0] = (data & 0xFF) as u8; // Illuminance bits 0-7
        slice[1] = ((data >> 8) & 0xFF) as u8; // Illuminance bits 8-15
        slice[2] = ((data >> 16) & 0xFF) as u8; // Event bits + padding

        return 3;
    } else if report_type == HIDReportType::Feature && report_id == HIDReportID::Feature {
        // Return current feature report
        let data: u32 = unsafe {
            (g_sensor_state.reporting_state as u32 & 0x3)
                | ((g_sensor_state.power_state as u32 & 0x7) << 2)
                | ((g_sensor_state.report_interval as u32 & 0xFFF) << 5)
        };

        slice[0] = (data & 0xFF) as u8;
        slice[1] = ((data >> 8) & 0xFF) as u8;
        slice[2] = ((data >> 16) & 0xFF) as u8;

        return 3;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn send_input_report(illuminance: u16, sensor_event: SensorEvent) {
    if unsafe { !tud_hid_ready_rs() } { return; }

    // Pack data: 16-bit illuminance + 3-bit event + 5-bit padding
    let data: u32 = illuminance as u32 | ((sensor_event as u32 & 0x7) << 16);

    unsafe { tud_hid_report_rs(HIDReportID::Input, &data as *const u32 as *const c_void, 3); }
}

#[unsafe(no_mangle)]
pub extern "C" fn send_feature_report() {
    unsafe {
        if !tud_hid_ready_rs() { return; }

        // Pack: reporting_state(2) + power_state(3) + report_interval(12) + padding(7)
        let data: u32 = (g_sensor_state.reporting_state as u32 & 0x3) | ((g_sensor_state.power_state as u32 & 0x7) << 2) | ((g_sensor_state.report_interval as u32 & 0xFFF) << 5);

        tud_hid_report_rs(HIDReportID::Feature, data as *const u32 as *const c_void, 3);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sensor_task() {
    unsafe {
        // Handle feature report updates
        if g_sensor_state.feature_report_updated {
            g_sensor_state.feature_report_updated = false;
            send_feature_report();

            // Restart timer with new interval if needed
            cancel_repeating_timer_rs(&raw const report_timer);
            if g_sensor_state.power_state == PowerState::Full && g_sensor_state.reporting_state == ReportingState::AllEvents {
                add_repeating_timer_ms_rs(g_sensor_state.report_interval as i32, timer_callback, null(), &raw const report_timer);
            }
        }

        // Handle periodic reporting
        if timer_triggered {
            timer_triggered = false;
            if g_sensor_state.power_state == PowerState::Full && g_sensor_state.reporting_state == ReportingState::AllEvents {
                g_sensor_state.illuminance = read_illuminance();
                send_input_report(g_sensor_state.illuminance, SensorEvent::default());
            }
        }

        // Handle state transitions and edge cases
        if g_sensor_state.reporting_state == ReportingState::AllEvents && g_sensor_state.power_state != PowerState::Full {
            g_sensor_state.power_state = PowerState::Full;
            send_feature_report();

            g_sensor_state.illuminance = read_illuminance();
            send_input_report(g_sensor_state.illuminance, SensorEvent::default());
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // TODO
    }
}
