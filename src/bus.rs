use std::env;
use std::ops::RangeInclusive;
use crate::utils::join_bytes;
use crate::rom::{ROM, load_rom};
use crate::ram::{RAM, DMGRAM, CGBRAM, WRAM_BANK_SELECT_ADDRESS};
use crate::ppu::{
    PPU,
    DMA_ADDRESS,
    HDMA5_ADDRESS,
};
use crate::timer::Timer;
use crate::joypad::{Joypad, JOYPAD_ADDRESS};
use crate::sound::Sound;
use crate::interrupts::{
    Interrupts,
    INTERRUPT_ENABLE_ADDRESS,
    INTERRUPT_FLAG_ADDRESS,
};

pub const BANK_ZERO: RangeInclusive<u16>                 = 0x0000..=0x3FFF;
pub const BANK_SWITCHABLE: RangeInclusive<u16>           = 0x4000..=0x7FFF;
pub const VIDEO_RAM: RangeInclusive<u16>                 = 0x8000..=0x9FFF;
pub const EXTERNAL_RAM: RangeInclusive<u16>              = 0xA000..=0xBFFF;
pub const WORK_RAM_1: RangeInclusive<u16>                = 0xC000..=0xCFFF;
pub const WORK_RAM_2: RangeInclusive<u16>                = 0xD000..=0xDFFF;
pub const ECHO_RAM: RangeInclusive<u16>                  = 0xE000..=0xFDFF;
pub const SPRITE_ATTRIBUTE_TABLE: RangeInclusive<u16>    = 0xFE00..=0xFE9F;
pub const NOT_USABLE: RangeInclusive<u16>                = 0xFEA0..=0xFEFF;
pub const IO_REGISTERS: RangeInclusive<u16>              = 0xFF00..=0xFF7F;
pub const HIGH_RAM: RangeInclusive<u16>                  = 0xFF80..=0xFFFE;
pub const PREPARE_SPEED_SWITCH_ADDRESS: u16              = 0xFF4D;

enum MemoryMap {
    BankZero,
    BankSwitchable,
    VideoRam,
    ExternalRam,
    WorkRam1,
    WorkRam2,
    EchoRam,
    SpriteAttributeTable,
    NotUsable,
    IoRegisters,
    HighRam,
    InterruptEnable,
}

pub struct Bus {
    data: [u8; 0x10000],
    pub rom: Box<dyn ROM>,
    pub ram: Box<dyn RAM>,
    pub ppu: PPU,
    pub joypad: Joypad,
    pub timer: Timer,
    pub sound: Sound,
    pub interrupts: Interrupts,
    pub cgb_mode: bool,
    pub double_speed_mode: bool,
    pub prepare_double_speed_mode: bool,
}

impl Bus {
    pub fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        #[cfg(not(test))]
        if args.len() < 2 {
            eprintln!("Please, specify a ROM file");
            std::process::exit(1);
        }
        let rom = match load_rom(&args.get(1).unwrap_or(&"".to_string())) {
            Ok(rom) => rom,
            Err(err) => {
                eprintln!("Could not read ROM: {}", err);
                std::process::exit(1);
            },
        };
        let info = rom.info().clone();
        let force_dmg_mode = !env::var("FORCE_DMG").is_err();
        let cgb_mode = (info.cgb_features() || info.cgb_only()) && !force_dmg_mode;
        let mut bus = Self {
            data: [0x00; 0x10000],
            rom,
            ram: match cgb_mode {
                true => Box::new(CGBRAM::new()),
                false => Box::new(DMGRAM::new()),
            },
            ppu: PPU::new(cgb_mode),
            joypad: Joypad::new(),
            timer: Timer::new(),
            sound: Sound::new(),
            interrupts: Interrupts::new(),
            cgb_mode,
            double_speed_mode: false,
            prepare_double_speed_mode: false,
        };

        // Hardware registers after the bootrom
        bus.write(0xFF00, 0xCF);
        bus.write(0xFF01, 0x00);
        bus.write(0xFF02, 0x7E);
        bus.write(0xFF04, 0x18);
        bus.write(0xFF05, 0x00);
        bus.write(0xFF06, 0x00);
        bus.write(0xFF07, 0xF8);
        bus.write(0xFF0F, 0xE1);

        bus.write(0xFF40, 0x91);
        bus.write(0xFF41, 0x81);
        bus.write(0xFF42, 0x00);
        bus.write(0xFF43, 0x00);
        bus.write(0xFF44, 0x91);
        bus.write(0xFF45, 0x00);
        bus.write(0xFF46, 0xFF);
        bus.write(0xFF47, 0xFC);

        bus.write(0xFF4A, 0x00);
        bus.write(0xFF4B, 0x00);
        bus.write(0xFFFF, 0x00);

        bus
    }

    fn map_address(address: u16) -> MemoryMap {
        match address {
            0x0000..=0x3FFF => MemoryMap::BankZero,
            0x4000..=0x7FFF => MemoryMap::BankSwitchable,
            0x8000..=0x9FFF => MemoryMap::VideoRam,
            0xA000..=0xBFFF => MemoryMap::ExternalRam,
            0xC000..=0xCFFF => MemoryMap::WorkRam1,
            0xD000..=0xDFFF => MemoryMap::WorkRam2,
            0xE000..=0xFDFF => MemoryMap::EchoRam,
            0xFE00..=0xFE9F => MemoryMap::SpriteAttributeTable,
            0xFEA0..=0xFEFF => MemoryMap::NotUsable,
            0xFF00..=0xFF7F => MemoryMap::IoRegisters,
            0xFF80..=0xFFFE => MemoryMap::HighRam,
            INTERRUPT_ENABLE_ADDRESS => MemoryMap::InterruptEnable,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match Bus::map_address(address) {
            MemoryMap::BankZero | MemoryMap::BankSwitchable | MemoryMap::ExternalRam => self.rom.read(address),
            MemoryMap::WorkRam1 | MemoryMap::WorkRam2 | MemoryMap::EchoRam => self.ram.read(address),
            MemoryMap::VideoRam => self.ppu.read_vram_external(address),
            MemoryMap::SpriteAttributeTable => self.ppu.read_oam(address),
            MemoryMap::IoRegisters => {
                if self.cgb_mode && address == PREPARE_SPEED_SWITCH_ADDRESS {
                    let byte = self.data[address as usize];
                    let current_speed = (self.double_speed_mode as u8) << 7;
                    let prepare_speed_switch = self.prepare_double_speed_mode as u8;
                    return (byte & 0b0111_1110) | current_speed | prepare_speed_switch;
                } else if address == WRAM_BANK_SELECT_ADDRESS {
                    return self.ram.read(address);
                } else if address == INTERRUPT_FLAG_ADDRESS {
                    return self.interrupts.read(address);
                } else if PPU::is_io_register(address) {
                    return self.ppu.get_register(address);
                } else if Sound::is_io_register(address) {
                    return self.sound.get_register(address);
                } else if Timer::is_io_register(address) {
                    return self.timer.get_register(address);
                } else if address == JOYPAD_ADDRESS {
                    return self.joypad.read(self.data[address as usize]);
                }
                return self.data[address as usize];
            },
            MemoryMap::InterruptEnable => self.interrupts.read(address),
            _ => self.data[address as usize],
        }
    }

    pub fn read_16bit(&self, address: u16) -> u16 {
        join_bytes(self.read(address.wrapping_add(1)), self.read(address))
    }

    pub fn write(&mut self, address: u16, data: u8) {
        match Bus::map_address(address) {
            MemoryMap::BankZero | MemoryMap::BankSwitchable | MemoryMap::ExternalRam => self.rom.write(address, data),
            MemoryMap::WorkRam1 | MemoryMap::WorkRam2 | MemoryMap::EchoRam => self.ram.write(address, data),
            MemoryMap::VideoRam => self.ppu.write_vram_external(address, data),
            MemoryMap::SpriteAttributeTable => self.ppu.write_oam(address, data),
            MemoryMap::IoRegisters => {
                if self.cgb_mode && address == PREPARE_SPEED_SWITCH_ADDRESS {
                    let current_byte = self.data[address as usize];
                    self.prepare_double_speed_mode = (data & 1) == 1;
                    // bit 7 is read only on cgb mode
                    self.data[address as usize] = (current_byte & 0b1000_0000) | (data & 0b0111_1111);
                } else if address == WRAM_BANK_SELECT_ADDRESS {
                    self.ram.write(address, data);
                } else if address == INTERRUPT_FLAG_ADDRESS {
                    self.interrupts.write(address, data);
                } else if PPU::is_io_register(address) {
                    self.ppu.set_register(address, data);
                    match address {
                        DMA_ADDRESS => {
                            self.ppu.set_register(address, data);
                            self.dma_transfer(data);
                        },
                        HDMA5_ADDRESS => {
                            self.ppu.set_register(address, data);
                            self.hdma_transfer(data);
                        },
                        _ => {}
                    }
                } else if Sound::is_io_register(address) {
                    self.sound.set_register(address, data);
                } else if Timer::is_io_register(address) {
                    self.timer.set_register(address, data);
                } else if address == JOYPAD_ADDRESS {
                    let byte = self.data[address as usize];
                    self.data[address as usize] = (data & 0b11110000) | (byte & 0b00001111);
                } else {
                    self.data[address as usize] = data;
                }
            },
            MemoryMap::InterruptEnable => self.interrupts.write(address, data),
            _ => self.data[address as usize] = data,
        };
    }

    pub fn write_16bit(&mut self, address: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address.wrapping_add(1), bytes[1]);
    }

    pub fn prepare_double_speed_mode(&self) -> bool {
        self.cgb_mode && self.prepare_double_speed_mode
    }

    pub fn double_speed_mode(&self) -> bool {
        self.cgb_mode && self.double_speed_mode
    }

    pub fn set_double_speed_mode(&mut self, val: bool) {
        self.double_speed_mode = val;
    }

    fn dma_transfer(&mut self, data: u8) {
        let source = (data as u16) * 0x100;
        let mut count: u16 = 0;
        let oam_addr = SPRITE_ATTRIBUTE_TABLE.min().unwrap();
        while count < 160 {
            self.ppu.write_oam(oam_addr + count, self.read(source + count));
            count += 1;
        }
    }

    fn hdma_transfer(&mut self, data: u8) {
        let source = self.ppu.hdma_source() & 0xFFF0;
        let destination = (self.ppu.hdma_destination() & 0xFF0) + 0x8000;
        let length = (((data & 0x7F) as u16) + 1) * 0x10;
        let mut count: u16 = 0;
        while count < length {
            let byte = self.read(source + count);
            self.ppu.write_vram_external(destination + count, byte);
            count += 1;
        }
        self.ppu.set_register(HDMA5_ADDRESS, 0xFF);
    }
}
