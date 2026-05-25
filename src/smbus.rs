//! SMBus mode helpers for STM32F1 I2C peripherals.

use stm32f1xx_hal::pac::i2c1::RegisterBlock;

/// Enable SMBus host mode (I2C master on the sensor bus).
pub fn enable_smbus_host(i2c: &RegisterBlock) {
    i2c.cr1().modify(|_, w| w.smbus().smbus().smbtype().host());
}

/// Configure peripheral as SMBus device (slave).
pub fn configure_smbus_device(i2c: &RegisterBlock) {
    i2c.cr1().modify(|_, w| {
        w.smbus()
            .smbus()
            .smbtype()
            .device()
            .enarp()
            .disabled()
            .enpec()
            .disabled()
    });
}
