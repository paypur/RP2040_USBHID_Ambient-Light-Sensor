use core::sync::atomic::Ordering;
use rp2040_hal::Adc;
use rp2040_hal::adc::AdcPin;
use rp2040_hal::fugit::MicrosDurationU32;
use rp2040_hal::gpio::bank0::Gpio26;
use rp2040_hal::gpio::{FunctionSio, Pin, PullNone, SioInput};
use rp2040_hal::timer::Alarm;
use rp2040_hal::usb::UsbBus;
use usbd_hid::descriptor::{AsInputReport, BufferOverflow};
use usbd_hid::hid_class::HIDClass;
use crate::{ALARM, ALARM_TRIGGERED};

#[repr(C)]
#[derive(Default)]
pub struct SensorState {
    pub power_state: PowerState,
    pub reporting_state: ReportingState,
    pub report_interval: u16, // in milliseconds
    pub illuminance: U16SMA,     // 0-65535 lux
    pub last_report_time: u64,
    pub feature_report_updated: bool,
}

impl SensorState {
    pub fn send_input_report(
        &mut self,
        // report_timer: &RepeatingTimer,
        hid: &HIDClass<UsbBus>
    ) {
        /*        // Handle feature report updates
                if self.feature_report_updated {
                    self.feature_report_updated = false;

                    // self.send_feature_report(hid)
                }
        */

        // Handle periodic reporting
        if ALARM_TRIGGERED.load(Ordering::Relaxed) {
            ALARM_TRIGGERED.store(false, Ordering::Relaxed);

            // reset alarm
            critical_section::with(|cs| {
                if let Some(ref mut alarm) = *ALARM.borrow(cs).borrow_mut() {
                    let _ = alarm.schedule(MicrosDurationU32::millis(self.report_interval as u32));
                }
            });

            if self.reporting_state == ReportingState::AllEvents && self.power_state == PowerState::Full {
                let _ = hid.push_input(self);
            }
        }

        /*      TODO: not really sure why we should be sending input reports more often when not in full power state
                // Handle state transitions and edge cases
                if self.reporting_state == ReportingState::AllEvents && self.power_state != PowerState::Full {
                    self.power_state = PowerState::Full;
                    // self.send_feature_report(hid);

                    self.read_illuminance(adc, adc_pin);
                    let _ = hid.push_input(self);
                }*/
    }

    pub fn read_illuminance(
        &mut self,
        adc: &mut Adc,
        adc_pin: &mut AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>>,
    ) {
        let adc_value: u16 = adc.read(adc_pin).unwrap();
        // Scale using y = 0.6294*x - 117.47, clamp to uint16_t range
        let mut y: u32 = (adc_value as u32) * 1611u32 / 10000u32;
        y = y.min(u16::MAX as u32);
        self.illuminance.sample(y as u16);
    }

    // pub fn send_feature_report(&mut self, hid: &HIDClass<UsbBus>) {
    //     let mut buffer = [0u8; 3];
    //     self.encode_feature_report(&mut buffer);
    //     TODO: I don't think im supposed send feature reports with this function
    //     if let Ok(_) = hid.push_raw_input(&buffer[..3]) {
    //         self.feature_report_updated = false;
    //     }
    // }

    pub fn encode_feature_report(&self, report: &mut [u8; 3]) {
        let data = (self.reporting_state as u32 & 0x3) | (self.power_state as u32 & 0x7 << 2) | (self.report_interval as u32 & 0xFFF << 5);

        report[0] = (data as u8) & 0xFF;
        report[1] = (data >> 8) as u8 & 0xFF;
        report[2] = (data >> 16) as u8 & 0xFF;
    }

    pub fn decode_feature_report(&mut self, report: &[u8; 3]) {
        let data: u32 = (report[0] as u32) | ((report[1] as u32) << 8) | ((report[2] as u32) << 16);

        let mut changed: bool = false;

        let received_reporting = ReportingState::from((data & 0x3) as u8);
        let received_power = PowerState::from(((data >> 2) & 0x7) as u8);
        let received_interval: u16 = ((data >> 5) & 0xFFF) as u16;

        if received_reporting != ReportingState::Invalid && received_reporting != self.reporting_state {
            self.reporting_state = received_reporting;
            changed = true;
        }

        if received_power != PowerState::Invalid && received_power != self.power_state {
            self.power_state = received_power;
            changed = true;
        }

        if received_interval != 0 && received_interval != self.report_interval {
            self.report_interval = received_interval;
            changed = true;
        }

        self.feature_report_updated = changed;
    }
}

impl AsInputReport for SensorState {
    fn serialize(&self, buffer: &mut [u8]) -> Result<usize, BufferOverflow> {
        let data: u32 = self.illuminance.value() | ((SensorEvent::default() as u32) << 16);

        buffer[0] = (data & 0xFF) as u8; // Illuminance bits 0-7
        buffer[1] = ((data >> 8) & 0xFF) as u8; // Illuminance bits 8-15
        buffer[2] = ((data >> 16) & 0xFF) as u8; // Event bits + padding

        Ok(3)
    }
}

// Power States
#[derive(Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub enum PowerState {
    Invalid = 0,
    Undefined = 1,
    Full = 2,    // D0
    Low = 3,     // D1
    Standby = 4, // D2
    Sleep = 5,   // D3
    #[default]
    Off = 6,     // D4
}

impl From<u8> for PowerState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Invalid,
            1 => Self::Undefined,
            2 => Self::Full,
            3 => Self::Low,
            4 => Self::Standby,
            5 => Self::Sleep,
            6 => Self::Off,
            _ => Self::default()
        }
    }
}

// Reporting States
#[derive(Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub enum ReportingState {
    Invalid = 0,
    #[default]
    NoEvents = 1,
    AllEvents = 2,
}

impl From<u8> for ReportingState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Invalid,
            1 => Self::NoEvents,
            2 => Self::AllEvents,
            _ => Self::default()
        }
    }
}

// Sensor Events
#[derive(Default)]
#[repr(C)]
pub enum SensorEvent {
    Invalid = 0,
    Unknown = 1,
    Changed = 2,
    PropertyChanged = 3,
    #[default]
    DataUpdated = 4,
    PollResponse = 5,
    Sensitivity = 6,
}

// 256 sample u16 Simple Moving Average
pub struct U16SMA {
    value: f32,
    index: u8,
    array: [u16; 256],
}

impl U16SMA {
    fn sample(&mut self, value: u16) {
        let old = self.array[self.index as usize];
        if old != 0 { self.value -= (value as f32) / 256f32 };

        self.array[self.index as usize] = value;
        self.value += (value as f32) / 256f32;

        self.index += 1;
    }

    fn value(&self) -> u32 {
        self.value as u32
    }
}

impl Default for U16SMA {
    fn default() -> Self {
        Self {
            value: 0.0,
            index: 0,
            array: [0; 256]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_eq;

    #[test]
    fn test_add() {
        let mut sma = U16SMA::default();
        sma.sample(10);
        assert_eq!(sma.value(), 10);
    }
}