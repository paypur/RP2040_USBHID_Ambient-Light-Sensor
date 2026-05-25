use core::{ptr, slice};
use crate::{decode_feature_report, g_sensor_state, read_illuminance};
use crate::types::{HIDReportID, HIDReportType, SensorEvent};

#[derive(Default)]
#[repr(C)]
struct PicoUniqueBoardId {
    id: [u8; 8]
}

unsafe extern "C" {
    fn pico_get_unique_board_id(id_out: *mut PicoUniqueBoardId);
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

// Invoked when received SET_REPORT control request or
// received data on OUT endpoint ( Report ID = 0, Type = 0 )
#[unsafe(no_mangle)]
pub extern "C" fn tud_hid_set_report_cb(_instance: u8, report_id: HIDReportID, report_type: HIDReportType, buffer: *const u8, buf_size: u16) {
    if report_type == HIDReportType::Feature && report_id == HIDReportID::Feature && buf_size == 3 {
        // Process feature report in interrupt context
        if decode_feature_report(buffer) {
            unsafe { g_sensor_state.feature_report_updated = true; }
        }
    }
}

// array of pointer to string descriptors
const STRING_DESC_ARR: [&[u8]; 3] = [
    b"\x09\x04",               // 0: is supported language is English (0x0409)
    b"Raspberry Pi",           // 1: Manufacturer
    b"RP2040 ALS HID Sensor",  // 2: Product
                               // 3: Serials, should use chip ID
];

static mut DESCRIPTOR: [u16; 32] = [0; 32];

// Invoked when received GET STRING DESCRIPTOR request
// Application return pointer to descriptor, whose contents must exist long enough for transfer to complete
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tud_descriptor_string_cb(index: u8, _langid: u16) -> *const u16 {
    unsafe {
        let char_count: u8;

        match index {
            0 => {
                let lang = STRING_DESC_ARR[0];
                DESCRIPTOR[1] = (lang[0] as u16) << 8 | lang[1] as u16;
                char_count = 1;
            }
            3 => {
                // Get unique serial number from RP2040 chip ID
                let mut board_id: PicoUniqueBoardId = PicoUniqueBoardId::default();
                pico_get_unique_board_id(&mut board_id);

                let dest_slice = &mut DESCRIPTOR[1..9];
                // let mut serial_str: [u8; 17];  // 8 bytes = 16 hex chars + null terminator
                for i in 0..board_id.id.len() {
                    dest_slice[i] = num_to_hex(board_id.id[i]);
                }

                char_count = 16;
            }
            4.. => { return ptr::null() }
            _ => {
                let str = STRING_DESC_ARR[index as usize];

                // Cap at max char
                char_count = (str.len() as u8).min(31);

                // Convert ASCII string into UTF-16
                for i in 0..char_count as usize {
                    DESCRIPTOR[1 + i] = str[i] as u16
                }
            }
        }

        // first byte is length (including header), second byte is string type
        // TODO: TUSB_DESC_STRING
        DESCRIPTOR[0] = (0x03u16) << 8 | (2 * char_count + 2) as u16;
        &raw const DESCRIPTOR as *const u16
    }
}

fn num_to_hex(n: u8) -> u16 {
    let low = n & 0xF;
    let high = n >> 4;
    (num_to_hex_char(high) as u16) << 8 | num_to_hex_char(low) as u16
}

fn num_to_hex_char(n: u8) -> u8 {
    if n >= 10 {
        n - 10 + b'A'
    } else {
        n + b'0'
    }
}
