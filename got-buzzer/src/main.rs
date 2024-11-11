#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;
use hal::block::ImageDef;
use music::Song;
use panic_halt as _;
use rp235x_hal as hal;
mod got;
mod music;
/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

const fn get_top(freq: f64, div_int: u8) -> u16 {
    let result = 150_000_000. / (freq * div_int as f64);
    result as u16 - 1
}

const PWM_DIV_INT: u8 = 64;

#[hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
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

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Init PWMs
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    // Configure PWM4
    let pwm = &mut pwm_slices.pwm7;
    pwm.enable();
    pwm.set_div_int(PWM_DIV_INT);
    pwm.channel_b.output_to(pins.gpio15);

    let song = Song::new(got::TEMPO);

    for (note, duration_type) in got::MELODY {
        let top = get_top(note, PWM_DIV_INT);
        pwm.set_top(top);

        let note_duration = song.calc_note_duration(duration_type);
        let pause_duration = note_duration / 10; // 10% of note_duration

        pwm.channel_b.set_duty_cycle_percent(50).unwrap(); // Set duty cycle to 50% to play the note

        timer.delay_ms(note_duration - pause_duration); // Play 90%
        pwm.channel_b.set_duty_cycle(0).unwrap(); // Stop tone
        timer.delay_ms(pause_duration); // Pause for 10%
    }

    loop {
        // we are not looping the song
        timer.delay_ms(500);
    }
}

// Program metadata for `picotool info`.
// This isn't needed, but it's recomended to have these minimal entries.
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"GotBuzzer"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

// End of file
