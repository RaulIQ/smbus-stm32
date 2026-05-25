//! I2C2 SMBus slave — serves `SensorSnapshot` to an external host.

use core::cell::RefCell;

use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::NVIC;
use pac::interrupt;
use stm32f1xx_hal::{pac, rcc::{Enable, Reset}};

use crate::{config, data, smbus};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransferDir {
    Idle,
    /// Host writes (command / register pointer).
    HostWrite,
    /// Host reads (data from our register map).
    HostRead,
}

struct SlaveState {
    dir: TransferDir,
    reg_ptr: u8,
    command_received: bool,
}

impl Default for SlaveState {
    fn default() -> Self {
        Self {
            dir: TransferDir::Idle,
            reg_ptr: 0,
            command_received: false,
        }
    }
}

static I2C2: Mutex<RefCell<Option<pac::I2C2>>> = Mutex::new(RefCell::new(None));
static STATE: Mutex<RefCell<SlaveState>> = Mutex::new(RefCell::new(SlaveState {
    dir: TransferDir::Idle,
    reg_ptr: 0,
    command_received: false,
}));

pub fn start(i2c2: pac::I2C2, rcc: &mut stm32f1xx_hal::rcc::Rcc) {
    let pclk1 = rcc.clocks.pclk1().raw();
    let clc_mhz = (pclk1 / 1_000_000).max(2) as u8;

    pac::I2C2::enable(rcc);
    pac::I2C2::reset(rcc);

    let i2c = i2c2;

    i2c.cr1().write(|w| w.pe().clear_bit());
    i2c.cr2().write(|w| unsafe { w.freq().bits(clc_mhz) });
    i2c.trise()
        .write(|w| w.trise().set((clc_mhz + 1) as u8));

    let ccr = (pclk1 / (100_000 * 2)).max(4);
    i2c.ccr().write(|w| unsafe {
        w.f_s()
            .clear_bit()
            .duty()
            .clear_bit()
            .ccr()
            .bits(ccr as u16)
    });

    smbus::configure_smbus_device(&i2c);

    i2c.oar1().write(|w| unsafe {
        w.addmode()
            .clear_bit()
            .add()
            .bits((config::SLAVE_ADDR as u16) << 1)
    });

    i2c.cr1().modify(|_, w| w.ack().set_bit());

    i2c.cr2().write(|w| w.iterren().set_bit().itevten().set_bit().itbufen().set_bit());

    i2c.cr1().modify(|_, w| w.pe().set_bit());

    cortex_m::interrupt::free(|cs| {
        *I2C2.borrow(cs).borrow_mut() = Some(i2c);
    });

    unsafe {
        NVIC::unmask(pac::Interrupt::I2C2_EV);
        NVIC::unmask(pac::Interrupt::I2C2_ER);
    }
}

fn with_i2c<R>(f: impl FnOnce(&pac::I2C2) -> R) -> Option<R> {
    cortex_m::interrupt::free(|cs| {
        I2C2
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(|i2c| f(i2c))
    })
}

fn with_state<R>(f: impl FnOnce(&mut SlaveState) -> R) -> R {
    cortex_m::interrupt::free(|cs| f(&mut *STATE.borrow(cs).borrow_mut()))
}

fn next_tx_byte(state: &SlaveState) -> u8 {
    data::read(|snap| {
        let regs = snap.register_bytes();
        regs[(state.reg_ptr as usize) % regs.len()]
    })
}

fn handle_event(i2c: &pac::I2C2) {
    let sr1 = i2c.sr1().read();

    if sr1.addr().bit_is_set() {
        let sr2 = i2c.sr2().read();
        with_state(|st| {
            st.command_received = false;
            if sr2.tra().bit_is_set() {
                st.dir = TransferDir::HostRead;
            } else {
                st.dir = TransferDir::HostWrite;
            }
        });
    }

    if sr1.rx_ne().bit_is_set() {
        let byte = i2c.dr().read().dr().bits();
        with_state(|st| {
            if st.dir == TransferDir::HostWrite && !st.command_received {
                st.reg_ptr = byte;
                st.command_received = true;
            }
        });
    }

    if sr1.tx_e().bit_is_set() {
        with_state(|st| {
            if st.dir == TransferDir::HostRead {
                let byte = next_tx_byte(st);
                i2c.dr().write(|w| w.dr().set(byte));
                st.reg_ptr = st.reg_ptr.wrapping_add(1);
            }
        });
    }

    if sr1.stopf().bit_is_set() {
        clear_stop(i2c);
        with_state(|st| {
            st.dir = TransferDir::Idle;
            st.command_received = false;
        });
    }

    if sr1.btf().bit_is_set() && sr1.tx_e().bit_is_set() {
        with_state(|st| {
            if st.dir == TransferDir::HostRead {
                let byte = next_tx_byte(st);
                i2c.dr().write(|w| w.dr().set(byte));
                st.reg_ptr = st.reg_ptr.wrapping_add(1);
            }
        });
    }
}

fn clear_stop(i2c: &pac::I2C2) {
    let cr1 = i2c.cr1().read();
    let _sr1 = i2c.sr1().read();
    i2c.cr1().write(|w| unsafe { w.bits(cr1.bits()) });
}

fn handle_error(i2c: &pac::I2C2) {
    let sr1 = i2c.sr1().read();
    if sr1.af().bit_is_set() {
        i2c.sr1().write(|w| w.af().clear_bit());
    }
    if sr1.berr().bit_is_set() {
        i2c.sr1().write(|w| w.berr().clear_bit());
    }
    if sr1.arlo().bit_is_set() {
        i2c.sr1().write(|w| w.arlo().clear_bit());
    }
    if sr1.ovr().bit_is_set() {
        i2c.sr1().write(|w| w.ovr().clear_bit());
    }
    if sr1.timeout().bit_is_set() {
        i2c.sr1().write(|w| w.timeout().clear_bit());
    }
    with_state(|st| {
        st.dir = TransferDir::Idle;
        st.command_received = false;
    });
}

#[interrupt]
fn I2C2_EV() {
    if let Some(()) = with_i2c(|i2c| handle_event(i2c)) {}
}

#[interrupt]
fn I2C2_ER() {
    if let Some(()) = with_i2c(|i2c| handle_error(i2c)) {}
}
