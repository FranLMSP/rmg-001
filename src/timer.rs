use crate::cpu::{Interrupt, Cycles};
use crate::bus::Bus;
use crate::utils::{
    BitIndex,
    get_bit,
};

pub const TIMER_DIVIDER_REGISTER_ADDRESS: u16 = 0xFF04;
pub const TIMER_COUNTER_ADDRESS: u16          = 0xFF05;
pub const TIMER_MODULO_ADDRESS: u16           = 0xFF06;
pub const TIMER_CONTROL_ADDRESS: u16          = 0xFF07;

pub struct Timer;

impl Timer {
    
    pub fn do_cycles(bus: &mut Bus, cycles: Cycles) {
        let mut count = 0;
        while count < cycles.to_t() {
            Timer::cycle(bus);
            count += 1;
        }
    }

    fn cycle(bus: &mut Bus) {
        let div = bus.read(TIMER_DIVIDER_REGISTER_ADDRESS);
        bus.write(TIMER_DIVIDER_REGISTER_ADDRESS, div.wrapping_add(1));

        if Timer::is_timer_enabled(bus) {
            let tima = bus.read(TIMER_COUNTER_ADDRESS);
            let tima_increment = Timer::get_tima_increment(bus);
            if tima.checked_add(tima_increment) == None {
                bus.write(TIMER_COUNTER_ADDRESS, bus.read(TIMER_MODULO_ADDRESS));
                bus.set_interrupt(Interrupt::Timer, true);
            } else {
                bus.write(TIMER_COUNTER_ADDRESS, tima.wrapping_add(tima_increment));
            }
        }
    }

    fn is_timer_enabled(bus: &Bus) -> bool {
        get_bit(bus.read(TIMER_CONTROL_ADDRESS), BitIndex::I2)
    }

    fn get_tima_increment(bus: &Bus) -> u8 {
        let clock_select = bus.read(TIMER_CONTROL_ADDRESS) & 0b0000_0011;
        match clock_select {
            0b00 => (4096 as u16 / 1026 as u16 / 4 as u16).to_be_bytes()[1],
            0b01 => (4096 as u16 /   16 as u16 / 4 as u16).to_be_bytes()[1],
            0b10 => (4096 as u16 /   64 as u16 / 4 as u16).to_be_bytes()[1],
            0b11 => (4096 as u16 /  256 as u16 / 4 as u16).to_be_bytes()[1],
            _ => 1,
        }
    }
}
