use std::ops::RangeInclusive;

pub const NR10_ADDRESS: u16 = 0xFF10;
pub const NR11_ADDRESS: u16 = 0xFF11;
pub const NR12_ADDRESS: u16 = 0xFF12;
pub const NR13_ADDRESS: u16 = 0xFF13;
pub const NR14_ADDRESS: u16 = 0xFF14;

pub const NR21_ADDRESS: u16 = 0xFF16;
pub const NR22_ADDRESS: u16 = 0xFF17;
pub const NR23_ADDRESS: u16 = 0xFF18;
pub const NR24_ADDRESS: u16 = 0xFF19;

pub const NR30_ADDRESS: u16 = 0xFF1A;
pub const NR31_ADDRESS: u16 = 0xFF1B;
pub const NR32_ADDRESS: u16 = 0xFF1C;
pub const NR33_ADDRESS: u16 = 0xFF1D;
pub const NR34_ADDRESS: u16 = 0xFF1E;

pub const NR41_ADDRESS: u16 = 0xFF20;
pub const NR42_ADDRESS: u16 = 0xFF21;
pub const NR43_ADDRESS: u16 = 0xFF22;
pub const NR44_ADDRESS: u16 = 0xFF23;

pub const NR50_ADDRESS: u16 = 0xFF24;
pub const NR51_ADDRESS: u16 = 0xFF25;
pub const NR52_ADDRESS: u16 = 0xFF26;

pub const WAVE_PATTERN_RAM: RangeInclusive<u16> = 0xFF30..=0xFF3F;

pub struct Sound {
    io_registers: [u8; 48],
}

impl Sound {
    pub fn is_io_register(address: u16) -> bool {
        address >= 0xFF10 && address <= 0xFF3F
    }

    pub fn get_register(&self, address: u16) -> u8 {
        self.io_registers[(address - 0xFF10) as usize]
    }

    pub fn set_register(&mut self, address: u16, data: u8) {
        self.io_registers[(address - 0xFF10) as usize] = data;
    }
}
