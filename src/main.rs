//! STM32F103: simple I2C check (MPU6050 WHO_AM_I).

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::println;
use defmt_rtt as _;
use panic_probe as _;
use stm32f1xx_hal::{
    i2c::{self, blocking::BlockingI2c},
    pac,
    prelude::*,
};

const MPU6050_ADDR: u8 = 0x68;
const MPU6050_WHO_AM_I_REG: u8 = 0x75;
const MPU6050_WHO_AM_I_ID: u8 = 0x68;

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.constrain();
    let mut delay = cortex_m::delay::Delay::new(cp.SYST, rcc.clocks.sysclk().raw());

    // BlockingI2c timeouts use DWT::cycle_count(); CYCCNT only runs when trace is enabled.
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut gpioc = dp.GPIOC.split(&mut rcc);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    // // 100 ms timeouts — without DCB::enable_trace() these never fire and I2C hangs forever.
    let mut i2c = BlockingI2c::new(
        dp.I2C1,
        (gpiob.pb6, gpiob.pb7),
        i2c::Mode::standard(100.kHz()),
        &mut rcc,
        100_000,
        3,
        100_000,
        100_000,
    );

    let mut buffer = [0u8];

    loop {
        led.toggle();
        println!("before i2c");

        let result = i2c.write_read(MPU6050_ADDR, &[MPU6050_WHO_AM_I_REG], &mut buffer);
        if result.is_err() {
            i2c.reset();
        }

        match result {
            Ok(()) => {
                if buffer[0] == MPU6050_WHO_AM_I_ID {
                    println!("MPU6050 OK: WHO_AM_I = 0x{:02x}", buffer[0]);
                } else {
                    println!("unexpected WHO_AM_I: 0x{:02x}", buffer[0]);
                }
            }
            Err(e) => match e {
                i2c::Error::NoAcknowledge(_) => println!("no acknowledge"),
                i2c::Error::Timeout => println!("timeout"),
                i2c::Error::Crc => println!("crc error"),
                i2c::Error::ArbitrationLoss => println!("arbitration loss"),
                _ => println!("other error"),
            },
        }

        delay.delay_ms(500);
    }
}
