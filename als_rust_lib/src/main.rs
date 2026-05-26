#![no_std]
#![no_main]

pub mod tinyusb;
mod types;

use crate::types::*;
use core::ffi::c_void;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use rp2040_hal::adc::AdcPin;
use rp2040_hal::gpio::bank0::{Gpio26, Gpio27, Gpio28};
use rp2040_hal::gpio::{FunctionNull, FunctionSio, Pin, PinState, PullDown, PullNone, SioInput};
use rp2040_hal::usb::UsbBus;
use rp2040_hal::{Adc, Clock, Sio, Watchdog, clocks, gpio, pac, entry};
use usb_device::LangID;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usb_device::prelude::StringDescriptors;

static timer_triggered: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn add_repeating_timer_ms_rs(
        delay_ms: i32,
        callback: extern "C" fn(*const RepeatingTimer) -> bool,
        user_data: *const c_void,
        out: *const RepeatingTimer,
    ) -> bool;
    fn cancel_repeating_timer_rs(timer: *const RepeatingTimer) -> bool;

    fn tud_hid_ready_rs() -> bool;
    fn tud_hid_report_rs(report_id: HIDReportID, report: *const c_void, len: u16) -> bool;
}

#[unsafe(no_mangle)]
pub extern "C" fn read_illuminance(
    adc: &mut Adc,
    adc_pin: &mut AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>>,
) -> u16 {
    let adc_value: u16 = adc.read(adc_pin).unwrap();
    // Scale using y = 0.6294*x - 117.47, clamp to uint16_t range
    let mut y: u32 = (adc_value as u32) * 1611u32 / 10000u32;
    y = y.min(u16::MAX as u32);
    y as u16
}

// pub extern "C" fn timer_callback(_rt: *const RepeatingTimer) -> bool {
//     timer_triggered.store(true, Ordering::Relaxed);
//     true // Keep repeating
// }

#[unsafe(no_mangle)]
pub extern "C" fn send_input_report(illuminance: u16, sensor_event: SensorEvent) {
    if unsafe { !tud_hid_ready_rs() } {
        return;
    }

    // Pack data: 16-bit illuminance + 3-bit event + 5-bit padding
    let data: u32 = illuminance as u32 | ((sensor_event as u32 & 0x7) << 16);

    unsafe {
        tud_hid_report_rs(HIDReportID::Input, &data as *const u32 as *const c_void, 3);
    }
}

fn sensor_task(
    sensor_state: &mut SensorState,
    // report_timer: &RepeatingTimer,
    adc: &mut Adc,
    adc_pin: &mut AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>>,
) {
    // Handle feature report updates
    if sensor_state.feature_report_updated {
        sensor_state.feature_report_updated = false;
        send_feature_report(sensor_state);

        unsafe {
            // Restart timer with new interval if needed
            // TODO: cancel_repeating_timer_rs(report_timer);
            if sensor_state.power_state == PowerState::Full
                && sensor_state.reporting_state == ReportingState::AllEvents
            {
                // TODO:
                // add_repeating_timer_ms_rs(
                //     sensor_state.report_interval as i32,
                //     timer_callback,
                //     null(),
                //     report_timer,
                // );
            }
        }
    }

    // Handle periodic reporting
    if timer_triggered.load(Ordering::Relaxed) {
        timer_triggered.store(true, Ordering::Relaxed);
        if sensor_state.power_state == PowerState::Full
            && sensor_state.reporting_state == ReportingState::AllEvents
        {
            sensor_state.illuminance = read_illuminance(adc, adc_pin);
            send_input_report(sensor_state.illuminance, SensorEvent::default());
        }
    }

    // Handle state transitions and edge cases
    if sensor_state.reporting_state == ReportingState::AllEvents
        && sensor_state.power_state != PowerState::Full
    {
        sensor_state.power_state = PowerState::Full;
        send_feature_report(sensor_state);

        sensor_state.illuminance = read_illuminance(adc, adc_pin);
        send_input_report(sensor_state.illuminance, SensorEvent::default());
    }
}

fn gpio_init_pins(
    pin27: Pin<Gpio27, FunctionNull, PullDown>,
    pin28: Pin<Gpio28, FunctionNull, PullDown>,
) {
    // Initialize Pin 28 (HIGH) and Pin 27 (LOW) as per spec
    pin27.into_push_pull_output_in_state(PinState::Low);
    pin28.into_push_pull_output_in_state(PinState::High);
}

fn adc_init_sensor(
    pin26: Pin<Gpio26, FunctionNull, PullDown>,
) -> AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>> {
    AdcPin::new(pin26.into_floating_input()).unwrap()
}

const desc_configuration: [u8; 34] = [
    // Configuration Descriptor
    9u8, // bLength
    2u8, // bDescriptorType
    34u8, 0u8,  // wTotalLength
    1u8,  // bNumInterfaces
    1u8,  // bConfigurationValue
    0u8,  // iConfiguration
    32u8, // bmAttributes
    50u8, // bMaxPower
    // Interface Descriptor
    9u8, // bLength
    4u8, // bDescriptorType
    0u8, // bInterfaceNumber
    0u8, // bAlternateSetting
    1u8, // bNumEndpoints
    3u8, // bInterfaceClass
    0u8, // bInterfaceSubClass
    0u8, // bInterfaceProtocol (None)
    0u8, // iInterface
    // HID Descriptor
    9u8,  // bLength
    33u8, // bDescriptorType
    0x11u8, 0x01u8, // bcdHID
    0u8,    // bCountryCode
    1u8,    // bNumDescriptors
    34u8,   // bDescriptorType
    108u8, 0u8, // wDescriptorLength
    // Endpoint Descriptor
    7u8,    // bLength
    5u8,    // bDescriptorType
    0x81u8, // bEndpointAddress
    3u8,    // bmAttributes
    64u8, 0u8, // wMaxPacketSize
    5u8, // bInterval
];

#[entry]
unsafe fn main() -> ! {
    let mut sensor_state: SensorState = SensorState::default();
    // let mut report_timer: RepeatingTimer = unsafe { transmute(0) };

    /* Initialize hardware */
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = clocks::init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The delay object lets us wait for specified amounts of time
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    gpio_init_pins(pins.gpio27, pins.gpio28);

    // Initialize USB
    let usb_allocator = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut hid = usbd_hid::hid_class::HIDClass::new_ep_in(&usb_allocator, &desc_configuration, 10);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_allocator, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::new(LangID::EN)
            .manufacturer("Raspberry Pi")
            .product("RP2040 ALS HID Sensor")
            // TODO: .serial_number()
        ])
        .unwrap()
        .device_class(0x00)
        .build();

    let mut adc_pin_0: AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>> =
        adc_init_sensor(pins.gpio26);
    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);



    let mut feature_buffer = [0u8; 3];

    sensor_state.illuminance = read_illuminance(&mut adc, &mut adc_pin_0);
    send_input_report(sensor_state.illuminance, SensorEvent::default());

    // ReSharper disable once CppDFAEndlessLoop
    loop {
        if !usb_dev.poll(&mut [&mut hid]) {
            sensor_task(&mut sensor_state, &mut adc, &mut adc_pin_0);
            delay.delay_ms(1);
            continue;
        }

        // TODO: check data from Host to determine when to send feature report
        // Host -> RPi
        if let Ok(_) = hid.pull_raw_report(&mut feature_buffer[..3]) {
            sensor_state.decode_feature_report(&feature_buffer);
        }

        if sensor_state.feature_report_updated {
            sensor_state.encode_feature_report(&mut feature_buffer);
            if let Ok(_) = hid.push_raw_input(&feature_buffer[..3]) {
                sensor_state.feature_report_updated = false;
            }
        }

        // RPi -> Host
        let _ = hid.push_input(&sensor_state);

        delay.delay_ms(1);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // TODO
    }
}
