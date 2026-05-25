//! Sensor data cached for the SMBus slave interface.

use core::cell::RefCell;

use cortex_m::interrupt::Mutex;

/// Cached sensor values exposed on the SMBus slave register map.
#[derive(Clone, Copy, Debug, Default)]
pub struct SensorSnapshot {
    /// Temperature in 0.01 °C (centi-degrees), big-endian in registers 0–1.
    pub temperature_centi_c: i16,
    /// Device / read status (register 2).
    pub status: u8,
    /// Increments on each successful sensor poll (register 3).
    pub seq: u8,
}

impl SensorSnapshot {
    /// Register map for SMBus read transactions (host sets pointer via write).
    pub fn register_bytes(&self) -> [u8; 8] {
        let t = self.temperature_centi_c.to_be_bytes();
        [t[0], t[1], self.status, self.seq, 0, 0, 0, 0]
    }

    /// Parse raw LM75-style temperature word (MSB first).
    pub fn from_lm75_raw(raw: [u8; 2]) -> Self {
        let temperature_centi_c = i16::from_be_bytes(raw);
        Self {
            temperature_centi_c,
            status: 0,
            seq: 0,
        }
    }
}

static SNAPSHOT: Mutex<RefCell<SensorSnapshot>> = Mutex::new(RefCell::new(SensorSnapshot {
    temperature_centi_c: 0,
    status: 0,
    seq: 0,
}));

pub fn with<R>(f: impl FnOnce(&mut SensorSnapshot) -> R) -> R {
    cortex_m::interrupt::free(|cs| f(&mut SNAPSHOT.borrow(cs).borrow_mut()))
}

pub fn read<R>(f: impl FnOnce(&SensorSnapshot) -> R) -> R {
    cortex_m::interrupt::free(|cs| f(&SNAPSHOT.borrow(cs).borrow()))
}
