//! Interact with a Wii extension controller via the wii-ext crate on a Pico board
//!
//! It will light the LED on GP25, based on the state of the B button on the controller.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::{fixed_point::FixedPoint, rate::Extensions};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::FunctionI2C,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use wii_ext::classic::Classic;
// use wii_ext::nunchuk::Nunchuk;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();

    let sda_pin = pins.gpio4.into_mode::<FunctionI2C>();
    let scl_pin = pins.gpio5.into_mode::<FunctionI2C>();

    let i2c = bsp::hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        100u32.kHz(),
        &mut pac.RESETS,
        clocks.peripheral_clock,
    );

    // Create, initialise and calibrate the controller
    let mut controller = Classic::new(i2c, &mut delay).unwrap();
    // Enable hi-resolution mode. This also updates calibration
    controller.enable_hires(&mut delay).unwrap();

    // If you have a Nunchuk controller, use this instead.
    // let mut controller = Nunchuk::new(i2c, &mut delay).unwrap();
    loop {
        // Capture the current button and axis values
        let input = controller.read_blocking(&mut delay).unwrap();

        // Set the LED off or on depending on the state of the B button
        // (switch this to input.button_c if using a nunchuk)
        if input.button_b {
            led_pin.set_high().unwrap();
        } else {
            led_pin.set_low().unwrap();
        }

        // Print inputs from the controller
        info!("{:?}", input);
    }
}

// End of file
