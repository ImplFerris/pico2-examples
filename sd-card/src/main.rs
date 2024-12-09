#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use hal::block::ImageDef;
use panic_halt as _;
use rp235x_hal::{self as hal, Clock};

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

use hal::fugit::RateExtU32;
use heapless::String;

use core::fmt::Write;

use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// A dummy timesource, which is mostly important for creating files.
#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

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
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("implRust")
            .product("Ferris")
            .serial_number("TEST")])
        .unwrap()
        .device_class(2) // 2 for the CDC, from: https://www.usb.org/defined-class-codes
        .build();

    let spi_cs = pins.gpio1.into_push_pull_output();
    let spi_sck = pins.gpio2.into_function::<hal::gpio::FunctionSpi>();
    let spi_mosi = pins.gpio3.into_function::<hal::gpio::FunctionSpi>();
    let spi_miso = pins.gpio4.into_function::<hal::gpio::FunctionSpi>();
    let spi_bus = hal::spi::Spi::<_, _, _, 8>::new(pac.SPI0, (spi_mosi, spi_miso, spi_sck));

    let spi = spi_bus.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        400.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    let spi = ExclusiveDevice::new(spi, spi_cs, timer).unwrap();
    let sdcard = SdCard::new(spi, timer);
    let mut buff: String<64> = String::new();

    let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());

    let mut is_read = false;
    loop {
        let _ = usb_dev.poll(&mut [&mut serial]);
        if !is_read && timer.get_counter().ticks() >= 2_000_000 {
            is_read = true;
            serial
                .write("Init SD card controller and retrieve card size...".as_bytes())
                .unwrap();
            match volume_mgr.device().num_bytes() {
                Ok(size) => {
                    write!(buff, "card size is {} bytes\r\n", size).unwrap();
                    serial.write(buff.as_bytes()).unwrap();
                }
                Err(e) => {
                    write!(buff, "Error: {:?}", e).unwrap();
                    serial.write(buff.as_bytes()).unwrap();
                }
            }
            buff.clear();

            let Ok(mut volume0) = volume_mgr.open_volume(VolumeIdx(0)) else {
                let _ = serial.write("err in open_volume".as_bytes());
                continue;
            };

            let Ok(mut root_dir) = volume0.open_root_dir() else {
                serial.write("err in open_root_dir".as_bytes()).unwrap();
                continue;
            };

            let Ok(mut my_file) =
                root_dir.open_file_in_dir("RUST.TXT", embedded_sdmmc::Mode::ReadOnly)
            else {
                serial.write("err in open_file_in_dir".as_bytes()).unwrap();
                continue;
            };

            while !my_file.is_eof() {
                let mut buffer = [0u8; 32];
                let num_read = my_file.read(&mut buffer).unwrap();
                for b in &buffer[0..num_read] {
                    write!(buff, "{}", *b as char).unwrap();
                }
            }
            serial.write(buff.as_bytes()).unwrap();
        }
        buff.clear();

        timer.delay_ms(50);
    }
}

#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"USB Fun"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];
