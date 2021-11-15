use crate::cpu::{Cycles};
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
    interrupt: bool,
    control: u8,
    io_registers: [u8; 4],
}

impl Timer {

    pub fn new() -> Self {
        Self {
            divider: 0,
            control: 0,
            prev_result: false,
            interrupt: false,
            is_enabled: false,
            io_registers: [0; 4],
        }
    }

    pub fn is_io_register(address: u16) -> bool {
        address >= 0xFF04 && address <= 0xFF07
    }

    pub fn get_register(&self, address: u16) -> u8 {
        if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            return self.read_divider();
        }
        self.io_registers[(address - 0xFF04) as usize]
    }

    pub fn set_register(&mut self, address: u16, data: u8) {
        if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            self.divider = 0;
            self.io_registers[(TIMER_DIVIDER_REGISTER_ADDRESS - 0xFF04) as usize] = 0;
        } else {
            self.io_registers[(address - 0xFF04) as usize] = data;
        }
    }

    pub fn get_interrupt(&self) -> bool {
        self.interrupt
    }

    pub fn set_interrupt(&mut self, val: bool) {
        self.interrupt = val
    }

    pub fn read_divider(&self) -> u8 {
        self.divider.to_be_bytes()[0]
    }

    pub fn reset(&mut self) {
        println!("Reset divider");
        self.divider = 0;
    }
    
    pub fn do_cycles(&mut self, cycles: Cycles) {
        self.is_enabled = self.is_timer_enabled();
        self.control = self.get_register(TIMER_CONTROL_ADDRESS);
        let mut count = 0;
        while count < cycles.0 {
            self.cycle();
            count += 1;
        }
    }

    fn cycle(&mut self) {
        self.divider = self.divider.wrapping_add(1);

        let result = self.is_enabled && self.get_tima_rate();

        if self.prev_result && !result {
            let tima = self.get_register(TIMER_COUNTER_ADDRESS).wrapping_add(1);
            if tima == 0 {
                self.set_register(TIMER_COUNTER_ADDRESS, self.get_register(TIMER_MODULO_ADDRESS));
                self.interrupt = true;
            } else {
                self.set_register(TIMER_COUNTER_ADDRESS, tima);
            }
        }

        self.prev_result = result;
    }

    fn is_timer_enabled(&self) -> bool {
        get_bit(self.get_register(TIMER_CONTROL_ADDRESS), BitIndex::I2)
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
