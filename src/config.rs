//! Pin and address configuration (adjust for your board).
//!
//! - I2C1 master (sensor): PB6 SCL, PB7 SDA
//! - I2C2 slave (host):    PB10 SCL, PB11 SDA

/// 7-bit address of the temperature sensor (LM75 default = 0x48).
pub const SENSOR_ADDR: u8 = 0x48;

/// Register pointer for SMBus read word (LM75 temperature register).
pub const SENSOR_TEMP_REG: u8 = 0x00;

/// I2C2 (slave): PB10 SCL, PB11 SDA — host-facing SMBus.
/// 7-bit address presented to the SMBus host.
pub const SLAVE_ADDR: u8 = 0x42;

/// How often to poll the sensor (milliseconds).
pub const SENSOR_POLL_MS: u32 = 500;
