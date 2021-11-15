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
    divider: u16,
    prev_result: bool,
    is_enabled: bool,
    control: u8,
}

impl Timer {

    pub fn new() -> Self {
        Self {
            divider: 0,
            control: 0,
            prev_result: false,
            is_enabled: false,
        }
    }

    pub fn read_divider(&self) -> u8 {
        self.divider.to_be_bytes()[0]
    }

    pub fn reset(&mut self) {
        println!("Reset divider");
        self.divider = 0;
    }
    
    pub fn do_cycles(&mut self, bus: &mut Bus, cycles: Cycles) {
        self.is_enabled = Timer::is_timer_enabled(bus);
        self.control = bus.read(TIMER_CONTROL_ADDRESS);
        let mut count = 0;
        while count < cycles.0 {
            self.cycle(bus);
            count += 1;
        }
    }

    fn cycle(&mut self, bus: &mut Bus) {
        self.divider = self.divider.wrapping_add(1);

        let result = self.is_enabled && self.get_tima_rate();

        if self.prev_result && !result {
            let tima = bus.read(TIMER_COUNTER_ADDRESS).wrapping_add(1);
            if tima == 0 {
                bus.write(TIMER_COUNTER_ADDRESS, bus.read(TIMER_MODULO_ADDRESS));
                bus.set_interrupt_flag(Interrupt::Timer, true);
            } else {
                bus.write(TIMER_COUNTER_ADDRESS, tima);
            }
        }

        self.prev_result = result;
    }

    fn is_timer_enabled(bus: &Bus) -> bool {
        get_bit(bus.read(TIMER_CONTROL_ADDRESS), BitIndex::I2)
    }

    fn get_tima_rate(&self) -> bool {
        let clock_select = self.control & 0b0000_0011;
        match clock_select {
            0b00 => ((self.divider >> 9) & 1) == 1,
            0b01 => ((self.divider >> 3) & 1) == 1,
            0b10 => ((self.divider >> 5) & 1) == 1,
            0b11 => ((self.divider >> 7) & 1) == 1,
            _ => unreachable!(),
        }
    }
}
