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

pub struct Timer {
    cycles: Cycles,
}

impl Timer {

    pub fn new() -> Self {
        Self {
            cycles: Cycles(0),
        }
    }

    fn increment_cycles(&mut self, cycles: Cycles) {
        self.cycles.0 += cycles.0;
    }

    fn reset_cycles(&mut self) {
        self.cycles.0 = 0;
    }
    
    pub fn do_cycles(&mut self, bus: &mut Bus, cycles: Cycles) {
        let mut count = 0;
        while count < cycles.to_t() {
            self.cycle(bus);
            count += 1;
        }
    }

    fn cycle(&mut self, bus: &mut Bus) {
        let div = bus.read(TIMER_DIVIDER_REGISTER_ADDRESS);
        bus.write(TIMER_DIVIDER_REGISTER_ADDRESS, div.wrapping_add(1));

        if Timer::is_timer_enabled(bus) {
            let tima = bus.read(TIMER_COUNTER_ADDRESS);
            let tima_rate = Timer::get_tima_rate(bus);
            if self.cycles.0 >= tima_rate {
                if tima.checked_add(1) == None {
                    bus.write(TIMER_COUNTER_ADDRESS, bus.read(TIMER_MODULO_ADDRESS));
                    bus.set_interrupt_flag(Interrupt::Timer, false);
                } else {
                    bus.write(TIMER_COUNTER_ADDRESS, tima.wrapping_add(1));
                }
                self.reset_cycles();
            }
        }

        self.increment_cycles(Cycles(1));
    }

    fn is_timer_enabled(bus: &Bus) -> bool {
        get_bit(bus.read(TIMER_CONTROL_ADDRESS), BitIndex::I2)
    }

    fn get_tima_rate(bus: &Bus) -> usize {
        let clock_select = bus.read(TIMER_CONTROL_ADDRESS) & 0b0000_0011;
        match clock_select {
            0b00 => 16,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => 1,
        }
    }
}
