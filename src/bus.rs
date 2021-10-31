use crate::utils::{
    get_bit,
    set_bit,
    BitIndex,
    join_bytes
};
use crate::rom::ROM;
use crate::ppu::{PPU, LCDStatus, LCDStatusModeFlag, LCD_STATUS_ADDRESS, LCD_CONTROL_ADDRESS, LCD_Y_ADDRESS};
use crate::cpu::{Interrupt};
use crate::timer::{TIMER_DIVIDER_REGISTER_ADDRESS};

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
}

impl Bus {
    pub fn new() -> Self {
        let game_rom = match ROM::load_file("ignore/dr-mario.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/01-special.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/02-interrupts.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/03-op sp,hl.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/04-op r,imm.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/05-op rp.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/06-ld r,r.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/07-jr,jp,call,ret,rst.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/08-misc instrs.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/09-op r,r.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/10-bit ops.gb".to_string()) {
        // let game_rom = match ROM::load_file("roms/cpu_instrs_individual/11-op a,(hl).gb".to_string()) {
            Ok(rom) => rom,
            // _ => ROM::from_bytes(&[0; 0xFFFF])
            _ => panic!("Could not read ROM"),
        };
        Self {
            data: [0x00; 0x10000],
            game_rom,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if address == 0xFF00 {
            return 0xFF;
        }
        if BANK_ZERO.in_range(address) || BANK_SWITCHABLE.in_range(address) {
            return self.game_rom.read(address);
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

        if BANK_ZERO.in_range(address) || BANK_SWITCHABLE.in_range(address) {
            // println!("WRITING TO ROM");
        } else if WORK_RAM_1.in_range(address) || WORK_RAM_2.in_range(address) {
            self.data[address as usize] = data;
            // Copy to the ECHO RAM
            if address <= 0xDDFF {
                self.data[(ECHO_RAM.begin() + (address - WORK_RAM_1.begin())) as usize] = data;
            }
        } else if ECHO_RAM.in_range(address) {
            self.data[address as usize] = data;
            self.data[(WORK_RAM_1.begin() + (address - ECHO_RAM.begin())) as usize] = data; // Copy to the working RAM
        } else if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            self.data[address as usize] = 0x00;
        } else if address == LCD_STATUS_ADDRESS {
            // Prevent user from modifying LCD Status mode
            let byte = self.data[address as usize];
            self.data[address as usize] = (data & 0b1111_1100) | (byte & 0b0000_0011);
        } else if address == LCD_CONTROL_ADDRESS && get_bit(data, BitIndex::I7) {
            self.data[address as usize] = data;
            self.data[LCD_Y_ADDRESS as usize] = 0x00;
        } else {
            self.data[address as usize] = data;
        }
    }

    pub fn write_16bit(&mut self, address: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address.wrapping_add(1), bytes[1]);
    }

    pub fn set_interrupt_enable(&mut self, interrupt: Interrupt, val: bool) {
        let byte = self.read(INTERRUPT_ENABLE_ADDRESS);
        self.write(INTERRUPT_ENABLE_ADDRESS, interrupt.set(byte, val));
    }

    pub fn set_interrupt_flag(&mut self, interrupt: Interrupt, val: bool) {
        let byte = self.read(INTERRUPT_FLAG_ADDRESS);
        self.write(INTERRUPT_FLAG_ADDRESS, interrupt.set(byte, val));
    }

    pub fn get_interrupt(&mut self, interrupt: Interrupt) -> bool {
        let byte = self.read(INTERRUPT_ENABLE_ADDRESS) & self.read(INTERRUPT_FLAG_ADDRESS);
        interrupt.get(byte)
    }
}
