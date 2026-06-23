use crate::Option::Some;
use crate::{ALARM, IS_ALARM_TRIGGERED};
use core::cmp::Ord;
use core::convert::From;
use core::default::Default;
use core::ops::{Deref, DerefMut};
use core::result::Result;
use core::result::Result::Ok;
use core::sync::atomic::Ordering;
use cortex_m::prelude::_embedded_hal_adc_OneShot;
use usb_device::class::ControlIn;
use usb_device::control;
use usbd_hid::descriptor::{AsInputReport, BufferOverflow};
use usbd_hid::hid_class::HIDClass;
use waveshare_rp2040_zero::hal::adc::AdcPin;
use waveshare_rp2040_zero::hal::fugit::MicrosDurationU32;
use waveshare_rp2040_zero::hal::gpio::bank0::Gpio26;
use waveshare_rp2040_zero::hal::gpio::{FunctionSio, Pin, PullNone, SioInput};
use waveshare_rp2040_zero::hal::timer::Alarm;
use waveshare_rp2040_zero::hal::usb::UsbBus;
use waveshare_rp2040_zero::hal::Adc;

#[repr(C)]
pub struct LightSensor {
    pub power_state: PowerState,
    pub reporting_state: ReportingState,
    pub report_interval: u16, // in milliseconds
    pub illuminance: IlluminanceSMA,
    pub last_report_time: u64,
    pub feature_report_updated: bool,
}

impl LightSensor {
    pub fn new() -> Self {
        Self {
            power_state: PowerState::default(),
            reporting_state: ReportingState::default(),
            report_interval: 10,
            illuminance: IlluminanceSMA::default(),
            last_report_time: 0,
            feature_report_updated: false,
        }
    }

    pub fn send_input_report(&mut self, hid: &HIDClass<UsbBus>) {
        /*        // Handle feature report updates
                if self.feature_report_updated {
                    self.feature_report_updated = false;

                    // self.send_feature_report(hid)
                }
        */

        // Handle periodic reporting
        if IS_ALARM_TRIGGERED.load(Ordering::Relaxed) {
            // reset alarm
            critical_section::with(|cs| {
                if let Some(ref mut alarm) = *ALARM.borrow(cs).borrow_mut() {
                    let _ = alarm.schedule(MicrosDurationU32::millis(self.report_interval as u32));
                }
            });

            // let _ = serial.write(b"reported\r\n");

            // if self.reporting_state == ReportingState::AllEvents && self.power_state == PowerState::Full {
            let _ = hid.push_input(self);
            // }

            IS_ALARM_TRIGGERED.store(false, Ordering::Relaxed);
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

    pub fn sample_illuminance(
        &mut self,
        adc: &mut Adc,
        adc_pin: &mut AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>>
    ) {
        let adc_value: u16 = adc.read(adc_pin).unwrap();
        self.illuminance.sample(adc_value);
    }

    pub fn encode_feature_report(&self, report: &mut [u8; 3]) {
        let data = (self.reporting_state as u32 & 0x3) | (self.power_state as u32 & 0x7 << 2) | (self.report_interval as u32 & 0xFFF << 5);

        report[0] = (data as u8) & 0xFF;
        report[1] = (data >> 8) as u8 & 0xFF;
        report[2] = (data >> 16) as u8 & 0xFF;
    }

    pub fn decode_feature_report(&mut self, report: &[u8]) {
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

impl AsInputReport for LightSensor {
    fn serialize(&self, buffer: &mut [u8]) -> Result<usize, BufferOverflow> {
        let illuminance = self.illuminance.value();
        buffer[0] = 1u8; // Report Id
        buffer[1] = illuminance as u8; // Illuminance bits 0-7
        buffer[2] = (illuminance >> 8) as u8; // Illuminance bits 8-15
        buffer[3] = SensorEvent::default() as u8; // Event bits + padding
        Ok(4)
    }
}

pub struct UsbLightSensor<> {
    sensor: LightSensor,
}

impl UsbLightSensor {
    pub fn new() -> Self {
        Self {
            sensor: LightSensor::new(),
        }
    }
}

impl Deref for UsbLightSensor {
    type Target = LightSensor;

    fn deref(&self) -> &Self::Target {
        &self.sensor
    }
}

impl DerefMut for UsbLightSensor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sensor
    }
}

impl<B: usb_device::bus::UsbBus> usb_device::class::UsbClass<B> for UsbLightSensor {
    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();

        // Is it an HID Class request targeting an Interface?
        if req.request_type == control::RequestType::Class && req.request == 0x01 { // 1 = GET_REPORT
            let report_type = (req.value >> 8) as u8;
            let report_id = (req.value & 0xFF) as u8;

            if report_type == 3 && report_id == 2 {
                let mut feature: [u8; 4] = [2, 0, 0, 0];

                self.sensor.encode_feature_report((&mut feature[1..4]).try_into().unwrap());

                xfer.accept_with(&feature).ok();
            }
        }
        // do nothing for other requests
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

pub struct IlluminanceSMA {
    value: f32,
    index: u8,
    array: [u16; 256],
}

impl IlluminanceSMA {
    // 256 sample Simple Moving Average
    fn sample(&mut self, raw_adc: u16) {
        let old = self.array[self.index as usize];
        if old != 0 { self.value -= (old as f32) / 256f32 };

        self.array[self.index as usize] = raw_adc;
        self.value += (raw_adc as f32) / 256f32;

        self.index = self.index.wrapping_add(1);
    }

    fn value(&self) -> u16 {
        // https://github.com/ParthaPRay/TEMT6000
        let resistor = 10000f32; // 10k ohms
        let k: f32 = 0.03162;
        let m: f32 = 1.5;

        let voltage = self.value * 3.3f32 / 4095f32;
        let current = (voltage / resistor) * 1E6; // micro amps
        let illuminance = libm::powf(current / k, 1f32 / m);

        (illuminance as u32).clamp(10, 1000) as u16
    }
}

impl Default for IlluminanceSMA {
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
        let mut sma = IlluminanceSMA::default();
        sma.sample(10);
        assert_eq!(sma.value(), 10);
    }
}