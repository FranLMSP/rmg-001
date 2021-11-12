use crate::utils::{
    get_bit,
    BitIndex,
    join_bytes
};
use crate::rom::ROM;
use crate::ppu::{
    LCD_STATUS_ADDRESS,
    LCD_CONTROL_ADDRESS,
    LCD_Y_ADDRESS,
    DMA_ADDRESS,
};
use crate::cpu::{Interrupt};
use crate::timer::{TIMER_DIVIDER_REGISTER_ADDRESS};
use crate::joypad::{Joypad, JOYPAD_ADDRESS};

pub struct AddressRange {
    begin: u16,
    end: u16,
}

impl AddressRange {
    pub fn begin(&self) -> u16 {
        self.begin
    }

    pub fn end(&self) -> u16 {
        self.end
    }

    pub fn in_range(&self, address: u16) -> bool {
        address >= self.begin && address <= self.end
    }
}

pub const BANK_ZERO: AddressRange                 = AddressRange{begin: 0x0000, end: 0x3FFF};
pub const BANK_SWITCHABLE: AddressRange           = AddressRange{begin: 0x4000, end: 0x7FFF};
pub const VIDEO_RAM: AddressRange                 = AddressRange{begin: 0x8000, end: 0x9FFF};
pub const EXTERNAL_RAM: AddressRange              = AddressRange{begin: 0xA000, end: 0xBFFF};
pub const WORK_RAM_1: AddressRange                = AddressRange{begin: 0xC000, end: 0xCFFF};
pub const WORK_RAM_2: AddressRange                = AddressRange{begin: 0xD000, end: 0xDFFF};
pub const ECHO_RAM: AddressRange                  = AddressRange{begin: 0xE000, end: 0xFDFF};
pub const SPRITE_ATTRIBUTE_TABLE: AddressRange    = AddressRange{begin: 0xFE00, end: 0xFE9F};
pub const NOT_USABLE: AddressRange                = AddressRange{begin: 0xFEA0, end: 0xFEFF};
pub const IO_REGISTERS: AddressRange              = AddressRange{begin: 0xFF00, end: 0xFF7F};
pub const HIGH_RAM: AddressRange                  = AddressRange{begin: 0xFF80, end: 0xFFFE};
pub const INTERRUPT_ENABLE_REGISTER: AddressRange = AddressRange{begin: 0xFFFF, end: 0xFFFF};
pub const INTERRUPT_ENABLE_ADDRESS: u16 = 0xFFFF;
pub const INTERRUPT_FLAG_ADDRESS: u16 = 0xFF0F;

pub struct Bus {
    game_rom: ROM,
    data: [u8; 0x10000],
    pub reset_timer: bool,
    pub joypad: Joypad,
}

impl Bus {
    pub fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            panic!("Please, specify a ROM");
        }
        let game_rom = match ROM::load_file(&args[1]) {
            Ok(rom) => rom,
            Err(_) => panic!("Could not read ROM"),
        };
        let mut data = [0x00; 0x10000];
        // Hardware registers after the bootrom
        data[0xFF00] = 0xCF;
        data[0xFF01] = 0x00;
        data[0xFF02] = 0x7E;
        data[0xFF04] = 0x18;
        data[0xFF05] = 0x00;
        data[0xFF06] = 0x00;
        data[0xFF07] = 0xF8;
        data[0xFF0F] = 0xE1;

        data[0xFF40] = 0x91;
        data[0xFF41] = 0x81;
        data[0xFF42] = 0x00;
        data[0xFF43] = 0x00;
        data[0xFF44] = 0x91;
        data[0xFF45] = 0x00;
        data[0xFF46] = 0xFF;
        data[0xFF47] = 0xFC;

        data[0xFF4A] = 0x00;
        data[0xFF4B] = 0x00;
        data[0xFFFF] = 0x00;

        Self {
            data,
            game_rom,
            reset_timer: false,
            joypad: Joypad::new(),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if BANK_ZERO.in_range(address) || BANK_SWITCHABLE.in_range(address)  || EXTERNAL_RAM.in_range(address) {
            return self.game_rom.read(address);
        } else if address == INTERRUPT_ENABLE_ADDRESS || address == INTERRUPT_FLAG_ADDRESS {
            return 0b11100000 | self.data[address as usize];
        } else if address == JOYPAD_ADDRESS {
            return self.joypad.read(self.data[address as usize]);
        }
        self.data[address as usize]
    }

    pub fn read_16bit(&self, address: u16) -> u16 {
        join_bytes(self.read(address.wrapping_add(1)), self.read(address))
    }

    pub fn write(&mut self, address: u16, data: u8) {
        if address == 0xFF01 {
            // print!("{}", data as char); 
        }

        if BANK_ZERO.in_range(address) || BANK_SWITCHABLE.in_range(address) || EXTERNAL_RAM.in_range(address) {
            self.game_rom.write(address, data);
        } else if WORK_RAM_1.in_range(address) || WORK_RAM_2.in_range(address) {
            self.data[address as usize] = data;
            // Copy to the ECHO RAM
            if address <= 0xDDFF {
                self.data[(ECHO_RAM.begin() + (address - WORK_RAM_1.begin())) as usize] = data;
            }
        } else if EXTERNAL_RAM.in_range(address) {
            // self.game_rom.write(address, data);
        } else if ECHO_RAM.in_range(address) {
            self.data[address as usize] = data;
            self.data[(WORK_RAM_1.begin() + (address - ECHO_RAM.begin())) as usize] = data; // Copy to the working RAM
        } else if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            self.reset_timer = true;
        } else if address == LCD_CONTROL_ADDRESS {
            self.data[address as usize] = data;
            // Check if LCD is being turned on 
            if get_bit(data, BitIndex::I7) && !get_bit(self.data[address as usize], BitIndex::I7) {
                self.data[LCD_Y_ADDRESS as usize] = 0x00;
            }
        } else if address == LCD_Y_ADDRESS {
            // println!("Write to LCD_Y not allowed");
        } else if address == LCD_STATUS_ADDRESS {
            let byte = self.data[address as usize];
            self.data[address as usize] = (data & 0b11111000) | (byte & 0b00000111);
        } else if address == JOYPAD_ADDRESS {
            let byte = self.data[address as usize];
            self.data[address as usize] = (data & 0b11110000) | (byte & 0b00001111);
        } else if address == DMA_ADDRESS {
            // the idea is: when something gets written to $FF46, multiply it by 0x100, then copy 160 bytes starting from that memory location into OAM
            self.data[address as usize] = data;
            let source = (data as usize) * 0x100;
            let mut count = 0;
            let oam_addr = SPRITE_ATTRIBUTE_TABLE.begin() as usize;
            while count < 160 {
                self.data[oam_addr + count] = self.data[source + count];
                count += 1;
            }
        } else {
            self.data[address as usize] = data;
        }
    }

    pub fn force_write(&mut self, address: u16, data: u8) {
        self.data[address as usize] = data;
    }

    pub fn write_16bit(&mut self, address: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address.wrapping_add(1), bytes[1]);
    }

    pub fn set_interrupt_flag(&mut self, interrupt: Interrupt, val: bool) {
        let byte = self.read(INTERRUPT_FLAG_ADDRESS);
        self.write(INTERRUPT_FLAG_ADDRESS, interrupt.set(byte, val));
    }
}
