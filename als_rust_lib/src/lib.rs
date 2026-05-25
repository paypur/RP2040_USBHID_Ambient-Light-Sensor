#![no_std]

mod types;
pub mod tinyusb;

use crate::types::*;
use core::ffi::c_void;
use core::mem::transmute;
use core::panic::PanicInfo;
use core::ptr::null;

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
pub extern "C" fn decode_feature_report(report: *const u8) -> bool {
    unsafe {
        let data: u32 = (*report.add(0) as u32) | ((*report.add(1) as u32) << 8) | ((*report.add(2) as u32) << 16);

        let mut changed: bool = false;

        let received_reporting: ReportingState = transmute((data & 0x3) as u8);
        let received_power: PowerState = transmute(((data >> 2) & 0x7) as u8);
        let received_interval: u16 = ((data >> 5) & 0xFFF) as u16;

        if received_reporting != ReportingState::Invalid && received_reporting != g_sensor_state.reporting_state {
            g_sensor_state.reporting_state = received_reporting;
            changed = true;
        }

        if received_power != PowerState::Invalid && received_power != g_sensor_state.power_state {
            g_sensor_state.power_state = received_power;
            changed = true;
        }

        if received_interval != 0 && received_interval != g_sensor_state.report_interval {
            g_sensor_state.report_interval = received_interval;
            changed = true;
        }

        changed
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
