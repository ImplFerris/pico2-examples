#![no_std]
#![no_main]

use embedded_hal::digital::{InputPin, OutputPin};
use panic_halt as _;

use rp235x_hal as hal;

use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let mut pwm_silces = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    let pwm = &mut pwm_silces.pwm1;
    pwm.enable();

    let led = &mut pwm.channel_b;
    led.output_to(pins.gpio3);

    let mut echo = pins.gpio16.into_pull_down_input();
    let mut trigger = pins.gpio17.into_push_pull_output();

    led.set_duty_cycle(0).unwrap();
    loop {
        timer.delay_ms(5);

        trigger.set_low().ok().unwrap();
        timer.delay_us(2);
        trigger.set_high().ok().unwrap();
        timer.delay_us(10);
        trigger.set_low().ok().unwrap();

        let mut time_low = 0;
        let mut time_high = 0;
        while echo.is_low().ok().unwrap() {
            time_low = timer.get_counter().ticks();
        }
        while echo.is_high().ok().unwrap() {
            time_high = timer.get_counter().ticks();
        }
        let time_passed = time_high - time_low;

        let distance = time_passed as f64 * 0.0343 / 2.0;

        let duty_cycle = if distance < 30.0 {
            let step = 30.0 - distance;
            (step * 1500.) as u16 + 1000
        } else {
            0
        };
        led.set_duty_cycle(duty_cycle).unwrap();
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"PWM Blinky Example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

