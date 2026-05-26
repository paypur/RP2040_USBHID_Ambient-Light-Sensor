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