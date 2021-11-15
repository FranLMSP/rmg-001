use std::fs::File;
use std::io::Read;

use crate::bus::{
    BANK_ZERO,
    BANK_SWITCHABLE,
    EXTERNAL_RAM,
};

pub const CARTRIDGE_TYPE_ADDRESS: u16 = 0x0147;
pub const CGB_FLAG_ADDRESS: u16 = 0x0143;
pub const SGB_FLAG_ADDRESS: u16 = 0x0146;
pub const RAM_SIZE_ADDRESS: u16 = 0x0149;
pub const ROM_SIZE_ADDRESS: u16 = 0x0148;
pub const DESTINATION_CODE_ADDRESS: u16 = 0x014A;

#[derive(Debug)]
enum Region {
    Japanese,
    NonJapanese,
}

#[derive(Debug, Copy, Clone)]
enum MBC {
    NoMBC,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    MBC6,
    MBC7,
    HuC1,
    HuC3,
    MMM01,
    PocketCamera,
    BandaiTIMA5,
}

#[derive(Debug)]
enum BankingMode {
    Simple,
    Advanced,
}

#[derive(Debug)]
pub struct ROMInfo {
    mbc: MBC,
    publisher: String,
    title: String,
    cgb_only: bool,
    sgb_features: bool,
    has_ram: bool,
    has_battery: bool,
    has_timer: bool,
    ram_banks: u8,
    rom_banks: u16,
    region: Region,
}

impl ROMInfo {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let rom_type = bytes[CARTRIDGE_TYPE_ADDRESS as usize];
        Self {
            mbc: match rom_type {
                0x00 => MBC::NoMBC,
                0x01 => MBC::MBC1,
                0x02 => MBC::MBC1,
                0x03 => MBC::MBC1,
                0x05 => MBC::MBC2,
                0x06 => MBC::MBC2,
                0x08 => MBC::NoMBC,
                0x09 => MBC::NoMBC,
                0x0B => MBC::MMM01,
                0x0C => MBC::MMM01,
                0x0F => MBC::MBC3,
                0x10 => MBC::MBC3,
                0x11 => MBC::MBC3,
                0x12 => MBC::MBC3,
                0x13 => MBC::MBC3,
                0x19 => MBC::MBC5,
                0x1A => MBC::MBC5,
                0x1B => MBC::MBC5,
                0x1C => MBC::MBC5,
                0x1D => MBC::MBC5,
                0x1E => MBC::MBC5,
                0x20 => MBC::MBC6,
                0x22 => MBC::MBC7,
                0xFC => MBC::PocketCamera,
                0xFD => MBC::BandaiTIMA5,
                0xFE => MBC::HuC3,
                0xFF => MBC::HuC1,
                _ => unreachable!(),
            },
            region: match bytes[DESTINATION_CODE_ADDRESS as usize] {
                0x00 => Region::Japanese,
                _ => Region::NonJapanese,
            },
            publisher: "".to_string(), // TODO: Extract publisher
            title: "".to_string(), // TODO: Extract the game title
            cgb_only: bytes[CGB_FLAG_ADDRESS as usize] == 0xC0,
            sgb_features: bytes[SGB_FLAG_ADDRESS as usize] == 0x03,
            has_ram: match rom_type {
                0x02 | 0x03 | 0x08 | 0x09 | 0x0C | 0x0D | 0x10 | 0x12 |
                0x13 | 0x1A | 0x1B | 0x1D | 0x1E | 0x22 | 0xFF => true,
                _ => false,
            },
            has_battery: match rom_type {
                0x03 | 0x06 | 0x09 | 0x0D | 0x0F | 0x10 |
                0x13 | 0x1B | 0x1E | 0x22 | 0xFF => true,
                _ => false,
            },
            has_timer: match rom_type {
                0x0F | 0x10 => true,
                _ => false,
            },
            ram_banks: match bytes[RAM_SIZE_ADDRESS as usize] {
                0x00 | 0x01 => 0,
                0x02 => 1,
                0x03 => 4,
                0x04 => 16,
                0x05 => 8,
                _ => unreachable!(),
            },
            rom_banks: match bytes[ROM_SIZE_ADDRESS as usize] {
                0x00 => 2,
                0x01 => 4,
                0x02 => 8,
                0x03 => 16,
                0x04 => 32,
                0x05 => 64,
                0x06 => 128,
                0x07 => 256,
                0x08 => 512,
                0x52 => 72,
                0x53 => 80,
                0x54 => 96,
                _ => unreachable!(),
            },
        }
    }

    pub fn ram_size(&self) -> usize {
        0x2000 * self.ram_banks as usize
    }
}

pub struct ROM {
    data: Vec<u8>,
    info: ROMInfo,
    ram: Vec<u8>,
    rom_bank: u16,
    ram_bank: u8,
    ram_enable: bool,
    banking_mode: BankingMode,
}

impl ROM {
    pub fn load_file(filename: &str) -> std::io::Result<Self> {
        let mut file = File::open(filename)?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;

        let info = ROMInfo::from_bytes(&data);
        println!("MBC {:?}", info.mbc);
        println!("Has RAM {}", info.has_ram);
        println!("ROM banks {}", info.rom_banks);
        println!("RAM banks {}", info.ram_banks);
        println!("Region {:?}", info.region);
        let ram = Vec::with_capacity(info.ram_size() as usize);

        Ok(Self {
            data,
            info,
            ram,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            banking_mode: BankingMode::Simple,
        })
    }

    pub fn read(&self, address: u16) -> u8 {
        match self.info.mbc {
            MBC::NoMBC => {
                return match self.data.get(address as usize) {
                    Some(data) => *data,
                    None => 0xFF,
                };
            },
            MBC::MBC1 => {
                if BANK_ZERO.contains(&address) {
                    return self.data[address as usize];
                } else if BANK_SWITCHABLE.contains(&address) {
                    return self.data[((self.rom_bank as usize * 0x4000) + (address as usize & 0x3FFF)) as usize];
                } else if EXTERNAL_RAM.contains(&address) {
                    if !self.info.has_ram {
                        return 0xFF;
                    }
                    return match self.ram.get((address - EXTERNAL_RAM.min().unwrap() + (0x2000 * self.ram_bank as u16)) as usize) {
                        Some(data) => *data,
                        None => 0xFF,
                    };
                }
                unreachable!("ROM read: Address {} not valid", address);
            },
            _ => unimplemented!(),
        }
    }

    pub fn write(&mut self, address: u16, data: u8) {
        match self.info.mbc {
            MBC::NoMBC => {},
            MBC::MBC1 => {
                if address <= 0x1FFF { // RAM enable register
                    if !self.info.has_ram {
                        return;
                    }
                    self.ram_enable = match data & 0x0F {
                        0x0A => true,
                        _ => false,
                    };
                    return;
                } else if address >= 0x2000 && address <= 0x3FFF { // ROM bank number register
                    // println!("Switch bank to {:02X}", data);
                    self.switch_rom_bank(data as u16 & 0b00011111);
                } else if address >= 0x4000 && address <= 0x5FFF { // ROM and RAM bank number register
                    // println!("RAM bank {:02X}", data);
                    self.ram_bank = data & 0b11;
                } else if address >= 0x6000 && address <= 0x7FFF { // Banking mode select
                    self.banking_mode = match data & 1 {
                        0 => BankingMode::Simple,
                        1 => BankingMode::Advanced,
                        _ => unreachable!(),
                    }
                } else if EXTERNAL_RAM.contains(&address) {
                    if !self.ram_enable || !self.info.has_ram {
                        return;
                    }
                    let address = address as usize - EXTERNAL_RAM.min().unwrap() as usize + (EXTERNAL_RAM.min().unwrap() as usize * self.ram_bank as usize);
                    if let Some(elem) = self.ram.get_mut(address) {
                        *elem = data;
                    }
                    self.switch_rom_bank(self.rom_bank + (data as u16 >> 5));
                }
            },
            _ => unimplemented!(),
        }
    }

    pub fn switch_rom_bank(&mut self, bank: u16) {
        self.rom_bank = bank;
        if self.rom_bank > self.info.rom_banks.saturating_sub(1) {
            self.rom_bank = self.info.rom_banks.saturating_sub(1);
        }
        if self.rom_bank == 0 {
            self.rom_bank = 1;
        }
    }
}
