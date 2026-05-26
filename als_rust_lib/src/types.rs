use usbd_hid::descriptor::{AsInputReport, BufferOverflow};

#[repr(C)]
#[derive(Default)]
pub struct SensorState {
    pub power_state: PowerState,
    pub reporting_state: ReportingState,
    pub report_interval: u16, // in milliseconds
    pub illuminance: u16,     // 0-65535 lux
    pub last_report_time: u64,
    pub feature_report_updated: bool,
}

impl SensorState {
    pub fn encode_feature_report(&self, report: &mut [u8; 3]) {
        let data = (self.reporting_state as u32 & 0x3) | (self.power_state as u32 & 0x7 << 2) | (self.report_interval as u32 & 0xFFF << 5);

        report[0] = (data as u8) & 0xFF;
        report[1] = (data as u8) >> 8 & 0xFF;
        report[2] = (data as u8) >> 16 & 0xFF;
    }
    
    pub fn decode_feature_report(&mut self, report: &[u8; 3]) {
        let data: u32 = (report[0] as u32) | ((report[1] as u32) << 8) | ((report[2] as u32) << 16);
    
        let mut changed: bool = false;
    
        let received_reporting = ReportingState::from((data & 0x3) as u8);
        let received_power = PowerState::from(((data >> 2) & 0x7) as u8);
        let received_interval: u16 = ((data >> 5) & 0xFFF) as u16;
    
        if received_reporting != ReportingState::Invalid 
        && received_reporting != self.reporting_state {
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


impl SensorState {
    pub const fn new() -> Self {
        SensorState {
            power_state: PowerState::Off,
            reporting_state: ReportingState::NoEvents,
            report_interval: 0,
            illuminance: 0,
            last_report_time: 0,
            feature_report_updated: false,
        }
    }
}

impl AsInputReport for SensorState {
    fn serialize(&self, buffer: &mut [u8]) -> Result<usize, BufferOverflow> {
        let data: u32 = self.illuminance as u32 | ((SensorEvent::default() as u32) << 16);

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

#[repr(u8)]
#[derive(PartialEq)]
pub enum HIDReportID {
    Input = 1, // Input report (illuminance data)
    Feature,   // Feature report (settings)
}

#[repr(C)]
#[derive(PartialEq)]
pub enum HIDReportType {
    Invalid = 0,
    Input,
    Output,
    Feature,
}

#[repr(C)]
pub struct RepeatingTimer {
    _unused: [u8; 0],
}