//! Interact with a Wii extension controller via the wii-ext crate on a Pico board
//!
//! It will enumerate as a USB joystick, which you can use to control a game
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use solderparty_rp2040_stamp_carrier as bsp;

use bsp::hal::{
    self,
    clocks::{init_clocks_and_plls, Clock},
    gpio::FunctionI2C,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use fugit::RateExtU32;
use wii_ext::classic::{Classic, ClassicReadingCalibrated};

use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::device::joystick::JoystickReport;
use usbd_human_interface_device::prelude::*;

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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut joy = UsbHidClassBuilder::new()
        .add_interface(
            usbd_human_interface_device::device::joystick::JoystickInterface::default_config(),
        )
        .build(&usb_bus);

    //https://pid.codes
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .manufacturer("usbd-human-interface-device")
        .product("Rusty joystick")
        .serial_number("TEST")
        .build();

    let sda_pin = pins.sda.into_mode::<FunctionI2C>();
    let scl_pin = pins.scl.into_mode::<FunctionI2C>();

    let i2c = bsp::hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        100.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    // Create, initialise and calibrate the controller
    let mut controller = Classic::new(i2c, &mut delay).unwrap();

    // Enable hi-resolution mode. This also updates calibration
    // Don't really need it for this single stick mode. Plus it might make recovery easier...
    //controller.enable_hires(&mut delay).unwrap();

    // If you have a Nunchuk controller, use this instead.
    // let mut controller = Nunchuk::new(i2c, &mut delay).unwrap();
    loop {
        // Need some delay here or things get unhappy.
        // TODO: investigate if it's a bug...
        delay.delay_ms(1);
        // Capture the current button and axis values
        let input = controller.read_blocking(&mut delay);

        // Poll every 10ms
        if let Ok(input) = input {
            match joy.interface().write_report(&get_report(&input)) {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write joystick report: {:?}", e)
                }
            }
            // Print inputs from the controller
            // info!("{:?}", input);
        } else {
            // re-init controller on failure
            let _ = controller.init(&mut delay);
            //let _ = controller.enable_hires(&mut delay);
        }

        if usb_dev.poll(&mut [&mut joy]) {}
    }
}

fn get_report(input: &ClassicReadingCalibrated) -> JoystickReport {
    // Read out buttons first
    let mut buttons = 0;

    buttons += (input.button_b as u8) << 0;
    buttons += (input.button_a as u8) << 1;
    buttons += (input.button_y as u8) << 2;
    buttons += (input.button_x as u8) << 3;
    buttons += (input.button_trigger_l as u8) << 4;
    buttons += (input.button_trigger_r as u8) << 5;
    buttons += (input.button_minus as u8) << 6;
    buttons += (input.button_plus as u8) << 7;

    JoystickReport {
        buttons,
        x: input.joystick_left_x,
        y: -input.joystick_left_y,
    }
}

// End of file
