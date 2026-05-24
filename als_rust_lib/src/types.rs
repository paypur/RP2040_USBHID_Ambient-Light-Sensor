#[repr(C)]
pub struct SensorState {
    pub power_state: PowerState,
    pub reporting_state: ReportingState,
    pub report_interval: u16, // in milliseconds
    pub illuminance: u16,     // 0-65535 lux
    pub last_report_time: u64,
    pub feature_report_updated: bool,
}

// Power States
#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub enum PowerState {
    Undefined = 1,
    Full = 2,    // D0
    Low = 3,     // D1
    Standby = 4, // D2
    Sleep = 5,   // D3
    Off = 6,     // D4
}

// Reporting States
#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ReportingState {
    NoEvents = 1,
    AllEvents = 2,
}

// Sensor Events
#[derive(Default)]
#[repr(C)]
pub enum SensorEvent {
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