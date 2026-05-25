//! STM32F103: SMBus master (sensor) + SMBus slave (host-facing).

#![no_std]
#![no_main]

mod config;
mod data;
mod i2c_master;
mod i2c_slave;
mod smbus;

use cortex_m_rt::entry;
use defmt::{error, info};
use defmt_rtt as _;
use panic_probe as _;
use stm32f1xx_hal::{
    i2c::{blocking::BlockingI2c, Mode},
    pac,
    prelude::*,
    rcc,
};

use crate::i2c_master::SensorMaster;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut cp = cp;
    cp.DWT.enable_cycle_counter();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.freeze(
        rcc::Config::hse(8.MHz())
            .sysclk(72.MHz())
            .pclk1(36.MHz()),
        &mut flash.acr,
    );

    let mut gpiob = dp.GPIOB.split(&mut rcc);

    // I2C1 — SMBus master (sensor on PB6/PB7)
    let scl_m = gpiob.pb6.into_alternate_open_drain(&mut gpiob.crl);
    let sda_m = gpiob.pb7.into_alternate_open_drain(&mut gpiob.crl);
    let i2c1 = BlockingI2c::new(
        dp.I2C1,
        (scl_m, sda_m),
        Mode::standard(100.kHz()),
        &mut rcc,
        1000,
        10,
        1000,
        1000,
    );
    smbus::enable_smbus_host(unsafe { &*pac::I2C1::ptr() });
    let mut master = SensorMaster::new(i2c1);

    // I2C2 — SMBus slave (host on PB10/PB11)
    let _scl_s = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
    let _sda_s = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);
    i2c_slave::start(dp.I2C2, &mut rcc);

    let mut gpioc = dp.GPIOC.split(&mut rcc);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    let mut delay = cortex_m::delay::Delay::new(cp.SYST, rcc.clocks.sysclk().raw());

    let mut tick_ms: u32 = 0;

    loop {
        if tick_ms == 0 {
            match master.read_sensor() {
                Ok(snap) => {
                    data::with(|cache| *cache = snap);
                    info!(
                        "sensor: temp={} cC seq={}",
                        snap.temperature_centi_c,
                        snap.seq
                    );
                }
                Err(_) => {
                    data::with(|cache| cache.status = 0xFF);
                    error!("sensor read failed");
                }
            }
        }

        led.toggle();
        delay.delay_ms(100);
        tick_ms = tick_ms.wrapping_add(100);
        if tick_ms >= config::SENSOR_POLL_MS {
            tick_ms = 0;
        }
    }
}
