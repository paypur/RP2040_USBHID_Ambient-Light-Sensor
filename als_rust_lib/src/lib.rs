#![no_std]

mod types;

use crate::types::*;
use core::panic::PanicInfo;
use core::slice;

unsafe extern "C" {
    pub static mut g_sensor_state: SensorState;
    pub static mut timer_triggered: bool;

    fn adc_read_rs() -> u16;
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

        let data: u32 = current_illuminance as u32 | ((SensorEvent::DataUpdated as u32) << 16);

        slice[0] = (data & 0xFF) as u8; // Illuminance bits 0-7
        slice[1] = ((data >> 8) & 0xFF) as u8; // Illuminance bits 8-15
        slice[2] = ((data >> 16) & 0xFF) as u8; // Event bits + padding

        return 3;
    } else if report_type == HIDReportType::Feature && report_id == HIDReportID::Feature {
        // Return current feature report
        let data: u32 = unsafe {
            (g_sensor_state.reporting_state as u32)
                | ((g_sensor_state.power_state as u32) << 2)
                | ((g_sensor_state.report_interval as u32) << 5)
        };

        slice[0] = (data & 0xFF) as u8;
        slice[1] = ((data >> 8) & 0xFF) as u8;
        slice[2] = ((data >> 16) & 0xFF) as u8;

        return 3;
    }

    0
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // TODO
    }
}
