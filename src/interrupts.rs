use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
};

pub const INTERRUPT_ENABLE_ADDRESS: u16 = 0xFFFF;
pub const INTERRUPT_FLAG_ADDRESS: u16 = 0xFF0F;

#[derive(Debug, Copy, Clone)]
pub enum Interrupt {
    VBlank,
    LCDSTAT,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    fn get_bit_index(&self) -> BitIndex {
        match self {
           Interrupt::VBlank  => BitIndex::I0,
           Interrupt::LCDSTAT => BitIndex::I1,
           Interrupt::Timer   => BitIndex::I2,
           Interrupt::Serial  => BitIndex::I3,
           Interrupt::Joypad  => BitIndex::I4,
        }
    }

    pub fn get(&self, byte: u8) -> bool {
        get_bit(byte, self.get_bit_index())
    }

    pub fn set(&self, byte: u8, val: bool) -> u8 {
        set_bit(byte, val, self.get_bit_index())
    }
    
    pub fn get_vector(&self) -> u16 {
        match self {
           Interrupt::VBlank  => 0x40,
           Interrupt::LCDSTAT => 0x48,
           Interrupt::Timer   => 0x50,
           Interrupt::Serial  => 0x58,
           Interrupt::Joypad  => 0x60,
        }
    }
}

pub struct Interrupts {
    interrupt_enable: u8,
    interrupt_flag: u8,
}

impl Interrupts {
    pub fn new() -> Self {
        Self {
            interrupt_enable: 0,
            interrupt_flag: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        let byte = match address {
            INTERRUPT_ENABLE_ADDRESS => self.interrupt_enable,
            INTERRUPT_FLAG_ADDRESS => self.interrupt_flag,
            _ => unreachable!(),
        };
        0b11100000 | byte
    }

    pub fn write(&mut self, address: u16, data: u8) {
        match address {
            INTERRUPT_ENABLE_ADDRESS => self.interrupt_enable = data,
            INTERRUPT_FLAG_ADDRESS => self.interrupt_flag = data,
            _ => unreachable!(),
        };
    }

    pub fn set(&mut self, interrupt: Interrupt, val: bool) {
        self.interrupt_flag = interrupt.set(self.interrupt_flag, val);
    }

    pub fn get(&self, interrupt: Interrupt) -> bool {
        interrupt.get(self.interrupt_flag)
    }

    pub fn request(&mut self, interrupt: Interrupt) {
        self.set(interrupt, true)
    }
}
