#![no_std]
use core::panic::PanicInfo;

unsafe extern "C" {
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // TODO
    }
}