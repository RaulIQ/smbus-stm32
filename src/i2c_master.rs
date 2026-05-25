//! I2C1 SMBus master — polls the external sensor.

use stm32f1xx_hal::{
    i2c::{blocking::BlockingI2c, Error},
    pac,
};

use crate::{config, data::SensorSnapshot};

pub struct SensorMaster {
    i2c: BlockingI2c<pac::I2C1>,
}

pub enum MasterError {
    I2c(Error),
}

impl From<Error> for MasterError {
    fn from(e: Error) -> Self {
        MasterError::I2c(e)
    }
}

impl SensorMaster {
    pub fn new(i2c: BlockingI2c<pac::I2C1>) -> Self {
        Self { i2c }
    }

    /// SMBus read word: write command byte, then read 2 data bytes.
    pub fn read_sensor(&mut self) -> Result<SensorSnapshot, MasterError> {
        let mut raw = [0u8; 2];
        self.i2c
            .write_read(config::SENSOR_ADDR, &[config::SENSOR_TEMP_REG], &mut raw)?;
        let mut snap = SensorSnapshot::from_lm75_raw(raw);
        crate::data::read(|prev| {
            snap.seq = prev.seq.wrapping_add(1);
            snap.status = 0;
        });
        Ok(snap)
    }
}
