#![no_std]
#![no_main]

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Alias for our HAL crate
use rp235x_hal as hal;

// Some things we need
use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// const PWM_DIV_INT: u8 = 128;
// const PWM_TOP: u16 = 23_436;

const PWM_DIV_INT: u8 = 64;
const PWM_TOP: u16 = 46_874;

const TOP: u16 = PWM_TOP + 1;
const MIN_DUTY: u16 = (TOP as f64 * (2.5 / 100.)) as u16;
const HALF_DUTY: u16 = (TOP as f64 * (7.5 / 100.)) as u16;
const MAX_DUTY: u16 = (TOP as f64 * (12. / 100.)) as u16;

#[hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    // The default is to generate a 125 MHz system clock
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

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // The delay object lets us wait for specified amounts of time (in
    // milliseconds)
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Init PWMs
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    // Configure PWM4
    let pwm = &mut pwm_slices.pwm4;

    pwm.set_div_int(PWM_DIV_INT);
    pwm.set_div_frac(0);

    pwm.set_top(PWM_TOP);
    pwm.enable();

    let servo = &mut pwm.channel_b;
    servo.output_to(pins.gpio9);

    loop {
        servo.set_duty_cycle(MIN_DUTY).unwrap();
        timer.delay_ms(1000);
        servo.set_duty_cycle(HALF_DUTY).unwrap();
        timer.delay_ms(1000);
        servo.set_duty_cycle(MAX_DUTY).unwrap();
        timer.delay_ms(1000);
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Servo Example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

// End of file
