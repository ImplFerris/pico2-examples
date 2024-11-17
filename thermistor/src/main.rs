#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as hal;
use embassy_rp::block::ImageDef;
use embassy_rp::gpio::Pull;
use embassy_time::Timer;
use heapless::String;
use ssd1306::mode::DisplayConfig;
use ssd1306::prelude::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306};
use {defmt_rtt as _, panic_probe as _};

use embassy_rp::adc::{Adc, Channel};
use embassy_rp::peripherals::I2C1;
use embassy_rp::{adc, bind_interrupts, i2c};

use core::fmt::Write;

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = hal::block::ImageDef::secure_exe();

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
});
const fn calculate_adc_max(adc_bits: u8) -> u16 {
    (1 << adc_bits) - 1
}
const ADC_BITS: u8 = 12; // 12-bit ADC in Pico
const ADC_MAX: u16 = calculate_adc_max(ADC_BITS); // 4095 for 12-bit ADC

const B_VALUE: f64 = 3950.0;
const REF_RES: f64 = 10_000.0; // Reference resistance in ohms (10kΩ)
const REF_TEMP: f64 = 25.0; // Reference temperature 25°C
                            // We have already covered about this formula in ADC chpater
fn adc_to_resistance(adc_value: u16, ref_res: f64) -> f64 {
    let x: f64 = (ADC_MAX as f64 / adc_value as f64) - 1.0;
    // ref_res * x // If you connected thermistor to power supply
    ref_res / x
}

// B Equation to convert resistance to temperature
fn calculate_temperature(current_res: f64, ref_res: f64, ref_temp: f64, b_val: f64) -> f64 {
    let ln_value = libm::log(current_res / ref_res); // Use libm for `no_std`
    let inv_t = (1.0 / ref_temp) + ((1.0 / b_val) * ln_value);
    1.0 / inv_t
}

fn kelvin_to_celsius(kelvin: f64) -> f64 {
    kelvin - 273.15
}

fn celsius_to_kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    // ADC to read the Vout value
    let mut adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    let mut p26 = Channel::new_pin(p.PIN_26, Pull::None);

    // Setting up I2C send text to OLED display
    let sda = p.PIN_18;
    let scl = p.PIN_19;
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, i2c::Config::default());
    let interface = I2CDisplayInterface::new(i2c);

    let mut display =
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0).into_terminal_mode();
    display.init().unwrap();
    let mut buff: String<64> = String::new();
    let ref_temp = celsius_to_kelvin(REF_TEMP);
    loop {
        let adc_value = adc.read(&mut p26).await.unwrap();
        writeln!(buff, "ADC: {}", adc_value).unwrap();

        let current_res = adc_to_resistance(adc_value, REF_RES);
        writeln!(buff, "R: {:.2}", current_res).unwrap();

        let temperature_kelvin = calculate_temperature(current_res, REF_RES, ref_temp, B_VALUE);
        let temperature_celsius = kelvin_to_celsius(temperature_kelvin);

        writeln!(buff, "Temp: {:.2} °C", temperature_celsius).unwrap();
        display.write_str(&buff).unwrap();
        Timer::after_secs(1).await;
    }
}

// Program metadata for `picotool info`.
// This isn't needed, but it's recomended to have these minimal entries.
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Blinky Example"),
    embassy_rp::binary_info::rp_program_description!(
        c"This example tests the RP Pico on board LED, connected to gpio 25"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

// End of file
