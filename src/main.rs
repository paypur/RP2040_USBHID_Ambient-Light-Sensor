#![cfg_attr(not(test), no_std)]
#![no_main]

mod types;

use crate::Option::Some;
use crate::Option::None;
use core::option::Option;
use core::cell::RefCell;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::delay::Delay;
use critical_section::Mutex;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{OutputPin, PinState, StatefulOutputPin};
use ws2812_pio::Ws2812;
use usb_device::LangID;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usb_device::prelude::StringDescriptors;
use usbd_hid::hid_class::HIDClass;
use waveshare_rp2040_zero::hal::pio::PIOExt;
use waveshare_rp2040_zero::{entry, XOSC_CRYSTAL_FREQ};
use waveshare_rp2040_zero::hal::{clocks, gpio, Clock, Sio, Timer, Watchdog};
use waveshare_rp2040_zero::hal::gpio::{FunctionNull, FunctionPio0, FunctionSio, Pin, PullDown, PullNone, SioInput};
use waveshare_rp2040_zero::pac::{interrupt, CorePeripherals, Peripherals};
use smart_leds_trait::{SmartLedsWrite, RGB8};
use waveshare_rp2040_zero::hal::adc::AdcPin;
use waveshare_rp2040_zero::hal::gpio::bank0::{Gpio26, Gpio27, Gpio28};
use waveshare_rp2040_zero::hal::timer::{Alarm, Alarm0};

static ALARM: Mutex<RefCell<Option<Alarm0>>> = Mutex::new(RefCell::new(None));
static ALARM_TRIGGERED: AtomicBool = AtomicBool::new(false);

// TODO: dont think this is needed anymore
// const DESCRIPTOR_CONFIG: [u8; 34] = [
//     // Configuration Descriptor
//     0x09, // bLength
//     0x02, // bDescriptorType
//     0x22, 0x00,  // wTotalLength
//     0x01,  // bNumInterfaces
//     0x01,  // bConfigurationValue
//     0x00,  // iConfiguration
//     0x20, // bmAttributes
//     0x32, // bMaxPower
//     // Interface Descriptor
//     0x09, // bLength
//     0x04, // bDescriptorType
//     0x00, // bInterfaceNumber
//     0x00, // bAlternateSetting
//     0x01, // bNumEndpoints
//     0x03, // bInterfaceClass
//     0x00, // bInterfaceSubClass
//     0x00, // bInterfaceProtocol (None)
//     0x00, // iInterface
//     // HID Descriptor
//     0x09,  // bLength
//     0x21, // bDescriptorType
//     0x11, 0x01, // bcdHID
//     0x00,    // bCountryCode
//     0x01,    // bNumDescriptors
//     0x22,   // bDescriptorType
//     0x6C, 0x00, // wDescriptorLength
//     // Endpoint Descriptor
//     0x07,    // bLength
//     0x05,    // bDescriptorType
//     0x81, // bEndpointAddress
//     0x03,    // bmAttributes
//     0x40, 0x00, // wMaxPacketSize
//     0x05, // bInterval
// ];

// @formatter:off
const DESC_HID_REPORT: [u8; 108] = [
    0x05, 0x20,                      // UsagePage(Sensors[0x0020])
    0x09, 0x01,                      // UsageId(Sensor[0x0001])
    0xA1, 0x01,                      // Collection(Application)
    0x09, 0x41,                      //     UsageId(Light: Ambient Light[0x0041])
    0xA1, 0x00,                      //     Collection(Physical)
    0x85, 0x01,                      //         ReportId(1)
    0x0A, 0xD1, 0x04,                //         UsageId(Data Field: Illuminance[0x04D1])
    0x15, 0x00,                      //         LogicalMinimum(0)
    0x27, 0xFF, 0xFF, 0x00, 0x00,    //         LogicalMaximum(65,535)
    0x95, 0x01,                      //         ReportCount(1)
    0x75, 0x10,                      //         ReportSize(16)
    0x81, 0x02,                      //         Input(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x0A, 0x02, 0x02,                //         UsageId(Event: Sensor Event[0x0202])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x10, 0x08,                //             UsageIdMin(Sensor Event: Unknown[0x0810])
    0x2A, 0x15, 0x08,                //             UsageIdMax(Sensor Event: Change Sensitivity[0x0815])
    0x15, 0x01,                      //             LogicalMinimum(1)
    0x25, 0x06,                      //             LogicalMaximum(6)
    0x75, 0x03,                      //             ReportSize(3)
    0x81, 0x00,                      //             Input(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0xC0,                            //         EndCollection()
    0x75, 0x05,                      //         ReportSize(5)
    0x81, 0x03,                      //         Input(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, BitField)
    0x85, 0x02,                      //         ReportId(2)
    0x0A, 0x16, 0x03,                //         UsageId(Property: Reporting State[0x0316])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x40, 0x08,                //             UsageIdMin(Reporting State: Report No Events[0x0840])
    0x2A, 0x41, 0x08,                //             UsageIdMax(Reporting State: Report All Events[0x0841])
    0x25, 0x02,                      //             LogicalMaximum(2)
    0x75, 0x02,                      //             ReportSize(2)
    0xB1, 0x00,                      //             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //         EndCollection()
    0x0A, 0x19, 0x03,                //         UsageId(Property: Power State[0x0319])
    0xA1, 0x02,                      //         Collection(Logical)
    0x1A, 0x50, 0x08,                //             UsageIdMin(Power State: Undefined[0x0850])
    0x2A, 0x55, 0x08,                //             UsageIdMax(Power State: D4 Power Off[0x0855])
    0x25, 0x06,                      //             LogicalMaximum(6)
    0x75, 0x03,                      //             ReportSize(3)
    0xB1, 0x00,                      //             Feature(Data, Array, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //         EndCollection()
    0x0A, 0x0E, 0x03,                //         UsageId(Property: Report Interval[0x030E])
    0x15, 0x00,                      //         LogicalMinimum(0)
    0x26, 0xFF, 0x0F,                //         LogicalMaximum(4,095)
    0x75, 0x0C,                      //         ReportSize(12)
    0xB1, 0x02,                      //         Feature(Data, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0x75, 0x07,                      //         ReportSize(7)
    0xB1, 0x03,                      //         Feature(Constant, Variable, Absolute, NoWrap, Linear, PreferredState, NoNullPosition, NonVolatile, BitField)
    0xC0,                            //     EndCollection()
    0xC0,                            // EndCollection()
];
// @formatter:on


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

#[interrupt]
unsafe fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        if let Some(ref mut alarm) = *ALARM.borrow(cs).borrow_mut() {
            alarm.clear_interrupt();
            ALARM_TRIGGERED.store(true, Ordering::Relaxed);
        }
    });
}

#[entry]
unsafe fn main() -> ! {
    // let mut sensor_state: SensorState = SensorState::default();
    // let mut report_timer: RepeatingTimer = unsafe { transmute(0) };

    /* Initialize hardware */
    let mut pac = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    /* Clock config */
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).unwrap();

    /* Timing config */
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    // let mut alarm = timer.alarm_0().unwrap();
    // alarm.enable_interrupt();
    // critical_section::with(|cs| ALARM.borrow(cs).replace(Some(alarm)));

    let sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // gpio_init_pins(pins.gpio27, pins.gpio28);

    let led = pins.gpio16.into_function::<FunctionPio0>();

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut ws2812 = Ws2812::new(
        led,
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    let mut hue: u8 = 0;

    loop {
        let color = hue_to_rgb(hue);
        ws2812.write(core::iter::once(color)).unwrap();
        hue = hue.wrapping_add(1);
        delay.delay_ms(10);
    }

    // // Initialize USB
    // let usb_allocator = UsbBusAllocator::new(UsbBus::new(
    //     pac.USBCTRL_REGS,
    //     pac.USBCTRL_DPRAM,
    //     clocks.usb_clock,
    //     true,
    //     &mut pac.RESETS,
    // ));
    //
    // let mut hid = HIDClass::new(&usb_allocator, &DESC_HID_REPORT, 10);
    //
    // let mut usb_dev = UsbDeviceBuilder::new(&usb_allocator, UsbVidPid(0x1209, 0x0001)).strings(&[
    //     StringDescriptors::new(LangID::EN).manufacturer("Raspberry Pi").product("RP2040 ALS HID Sensor"), // TODO: .serial_number()
    // ]).unwrap().device_class(0x00).build();
    //
    // let mut adc_pin_0: AdcPin<Pin<Gpio26, FunctionSio<SioInput>, PullNone>> = adc_init_sensor(pins.gpio26);
    // let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);
    //
    // let mut feature_buffer = [0u8; 3];
    //
    // sensor_state.read_illuminance(&mut adc, &mut adc_pin_0);
    // // RPi -> Host
    // // let _ = hid.push_input(&sensor_state);
    //
    // // ReSharper disable once CppDFAEndlessLoop
    // loop {
    //     if !usb_dev.poll(&mut [&mut hid]) {
    //         sensor_state.read_illuminance(&mut adc, &mut adc_pin_0);
    //         delay.delay_ms(1);
    //         continue;
    //     }
    //
    //     // Host -> RPi
    //     if let Ok(_info) = hid.pull_raw_report(&mut feature_buffer[..3]) {
    //         sensor_state.decode_feature_report(&feature_buffer);
    //     }
    //
    //     sensor_state.read_illuminance(&mut adc, &mut adc_pin_0);
    //     sensor_state.send_input_report(&hid);
    //
    //     delay.delay_ms(1);
    // }
}

fn hue_to_rgb(hue: u8) -> RGB8 {
    let sector = hue / 43;
    let r = hue % 43;
    let m = (r as u32 * 255 / 43) as u8;
    let n = 255 - m;

    match sector {
        0 => RGB8 { r: 255, g: m, b: 0 },
        1 => RGB8 { r: n, g: 255, b: 0 },
        2 => RGB8 { r: 0, g: 255, b: m },
        3 => RGB8 { r: 0, g: n, b: 255 },
        4 => RGB8 { r: m, g: 0, b: 255 },
        _ => RGB8 { r: 255, g: 0, b: n },
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // TODO
    }
}
