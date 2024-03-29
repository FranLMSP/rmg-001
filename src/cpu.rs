use std::env;
use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
    join_bytes,
    add_half_carry,
    sub_half_carry,
    add_half_carry_16bit,
};
use crate::bus::{
    Bus,
    PREPARE_SPEED_SWITCH_ADDRESS,
};
use crate::interrupts::{
    Interrupt,
    INTERRUPT_ENABLE_ADDRESS,
    INTERRUPT_FLAG_ADDRESS,
};

#[derive(Debug, Copy, Clone)]
pub enum Register {
    A, // Accumulator
    F, // Flags
    B, 
    C,
    D,
    E,
    H,
    L,
    // This registers are just the same as above but combined to get a 16 bits register
    AF,
    BC,
    DE,
    HL,

    SP, // Stack pointer
    PC, // Program counter
}

type Rg = Register;

impl Register {
    pub fn is_8bit(&self) -> bool {
        match self {
            Rg::A | Rg::F | Rg::B | Rg::C | Rg::D | Rg::E | Rg::H | Rg::L => true,
            Rg::AF | Rg::BC | Rg::DE | Rg::HL | Rg::SP | Rg::PC => false,
        }
    }

    pub fn is_16bit(&self) -> bool {
        !self.is_8bit()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FlagRegister {
    Zero, // Set when the result of a math operation is zero or if two values matches using the CP instruction
    Substract, // Set if a substraction was performed in the last math instruction
    HalfCarry, // Set if a carry ocurred from the lower nibble in the last math operation
    Carry, // Set if a carry was ocurrend from the last math operation or if register A is the smaller value when executing the CP instruction
}

pub struct Registers {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0x01,
            f: 0xB0, // The first 4 lower bits are always set to 0
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100, // On power up, the Gameboy executes the instruction at hex 100
        }
    }

    pub fn new_cgb() -> Self {
        Self {
            a: 0x11,
            f: 0x00,
            b: 0x00,
            c: 0x00,
            d: 0xFF,
            e: 0x56,
            h: 0x00,
            l: 0x0D,
            sp: 0xFFFE,
            pc: 0x0100,
        }
    }

    pub fn get(&self, register: Register) -> u16 {
        match register {
            Register::A => self.a as u16,
            Register::B => self.b as u16,
            Register::C => self.c as u16,
            Register::D => self.d as u16,
            Register::E => self.e as u16,
            Register::F => self.f as u16,
            Register::H => self.h as u16,
            Register::L => self.l as u16,
            Register::AF => join_bytes(self.a, self.f),
            Register::BC => join_bytes(self.b, self.c),
            Register::DE => join_bytes(self.d, self.e),
            Register::HL => join_bytes(self.h, self.l),
            Register::SP => self.sp,
            Register::PC => self.pc,
        }
    }
    
    pub fn get_8bit(&self, register: Register) -> u8 {
        self.get(register).to_be_bytes()[1]
    }

    pub fn set(&mut self, register: Register, val: u16) {
        let bytes = val.to_be_bytes();
        match register {
            Register::A  => self.a = bytes[1],
            Register::B  => self.b = bytes[1],
            Register::C  => self.c = bytes[1],
            Register::D  => self.d = bytes[1],
            Register::E  => self.e = bytes[1],
            Register::F  => self.f = bytes[1],
            Register::H  => self.h = bytes[1],
            Register::L  => self.l = bytes[1],
            Register::AF => {self.a = bytes[0];self.f = bytes[1];},
            Register::BC => {self.b = bytes[0];self.c = bytes[1];},
            Register::DE => {self.d = bytes[0];self.e = bytes[1];},
            Register::HL => {self.h = bytes[0];self.l = bytes[1];},
            Register::SP => self.sp = val,
            Register::PC => self.pc = val,
        }
    }

    pub fn get_flag(&self, flag: FlagRegister) -> bool {
        match flag {
            FlagRegister::Zero      => get_bit(self.f, BitIndex::I7),
            FlagRegister::Substract => get_bit(self.f, BitIndex::I6),
            FlagRegister::HalfCarry => get_bit(self.f, BitIndex::I5),
            FlagRegister::Carry     => get_bit(self.f, BitIndex::I4),
        }
    }

    pub fn set_flag(&mut self, flag: FlagRegister, val: bool) {
        match flag {
            FlagRegister::Zero      => self.f = set_bit(self.f, val, BitIndex::I7),
            FlagRegister::Substract => self.f = set_bit(self.f, val, BitIndex::I6),
            FlagRegister::HalfCarry => self.f = set_bit(self.f, val, BitIndex::I5),
            FlagRegister::Carry     => self.f = set_bit(self.f, val, BitIndex::I4),
        }
    }

    pub fn increment(&mut self, register: Register, times: u16) {
        let val = self.get(register);
        self.set(register, val.wrapping_add(times));
    }

    pub fn decrement(&mut self, register: Register, times: u16) {
        let val = self.get(register);
        self.set(register, val.wrapping_sub(times));
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum OpcodeParameter {
    Register(Register),
    Register_U8(Register, u8),
    Register_U16(Register, u16),
    Register_I8(Register, i8),
    Register_I16(Register, u16),
    U8_Register(u8, Register),
    U16_Register(u16, Register),
    I8_Register(u8, Register),
    I16_Register(u16, Register),
    Register_16BitAddress(Register, u16),
    Register_Register(Register, Register),

    Register_RegisterDecrement(Register, Register),
    RegisterDecrement_Register(Register, Register),

    Register_RegisterIncrement(Register, Register),
    RegisterIncrement_Register(Register, Register),

    Register_FF00plusRegister(Register, Register),
    FF00plusRegister_Register(Register, Register),
    Register_FF00plusU8(Register, u8),
    FF00plusU8_Register(u8, Register),

    Register_RegisterPlusI8(Register, Register, i8),

    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    FlagRegisterReset(FlagRegister),
    FlagRegisterSet(FlagRegister),
    FlagRegisterReset_U16(FlagRegister, u16),
    FlagRegisterSet_U16(FlagRegister, u16),
    FlagRegisterReset_I16(FlagRegister, i16),
    FlagRegisterSet_I16(FlagRegister, i16),
    FlagRegisterReset_U8(FlagRegister, u8),
    FlagRegisterSet_U8(FlagRegister, u8),
    FlagRegisterReset_I8(FlagRegister, i8),
    FlagRegisterSet_I8(FlagRegister, i8),

    NoParam,
}

#[derive(Debug, Copy, Clone)]
pub struct OpcodeParameterBytes(u8, u8, u8, u8);

impl OpcodeParameterBytes {

    pub fn from_address(address: u16, bus: &Bus)-> OpcodeParameterBytes {
        OpcodeParameterBytes(
            bus.read(address),
            bus.read(address.wrapping_add(1)),
            bus.read(address.wrapping_add(2)),
            bus.read(address.wrapping_add(3)),
        )
    }

    pub fn parse_opcode(&self) -> (Opcode, Cycles) {
        let opcode = self.0;
        let two_byte_param = join_bytes(self.2, self.1);
        match opcode {
            0x06 => (Opcode::LD(OpcodeParameter::Register_U8(Register::B, self.1)), Cycles(2.0)),
            0x0E => (Opcode::LD(OpcodeParameter::Register_U8(Register::C, self.1)), Cycles(2.0)),
            0x16 => (Opcode::LD(OpcodeParameter::Register_U8(Register::D, self.1)), Cycles(2.0)),
            0x1E => (Opcode::LD(OpcodeParameter::Register_U8(Register::E, self.1)), Cycles(2.0)),
            0x26 => (Opcode::LD(OpcodeParameter::Register_U8(Register::H, self.1)), Cycles(2.0)),
            0x2E => (Opcode::LD(OpcodeParameter::Register_U8(Register::L, self.1)), Cycles(2.0)),
            0x7F => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0x78 => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0x79 => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0x7A => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0x7B => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0x7C => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0x7D => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0x7E => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0x40 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::B)), Cycles(1.0)),
            0x41 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::C)), Cycles(1.0)),
            0x42 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::D)), Cycles(1.0)),
            0x43 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::E)), Cycles(1.0)),
            0x44 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::H)), Cycles(1.0)),
            0x45 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::L)), Cycles(1.0)),
            0x46 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::HL)), Cycles(2.0)),
            0x48 => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::B)), Cycles(1.0)),
            0x49 => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::C)), Cycles(1.0)),
            0x4A => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::D)), Cycles(1.0)),
            0x4B => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::E)), Cycles(1.0)),
            0x4C => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::H)), Cycles(1.0)),
            0x4D => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::L)), Cycles(1.0)),
            0x4E => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::HL)), Cycles(2.0)),
            0x50 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::B)), Cycles(1.0)),
            0x51 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::C)), Cycles(1.0)),
            0x52 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::D)), Cycles(1.0)),
            0x53 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::E)), Cycles(1.0)),
            0x54 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::H)), Cycles(1.0)),
            0x55 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::L)), Cycles(1.0)),
            0x56 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::HL)), Cycles(2.0)),
            0x58 => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::B)), Cycles(1.0)),
            0x59 => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::C)), Cycles(1.0)),
            0x5A => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::D)), Cycles(1.0)),
            0x5B => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::E)), Cycles(1.0)),
            0x5C => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::H)), Cycles(1.0)),
            0x5D => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::L)), Cycles(1.0)),
            0x5E => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::HL)), Cycles(2.0)),
            0x60 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::B)), Cycles(1.0)),
            0x61 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::C)), Cycles(1.0)),
            0x62 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::D)), Cycles(1.0)),
            0x63 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::E)), Cycles(1.0)),
            0x64 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::H)), Cycles(1.0)),
            0x65 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::L)), Cycles(1.0)),
            0x66 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::HL)), Cycles(2.0)),
            0x68 => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::B)), Cycles(1.0)),
            0x69 => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::C)), Cycles(1.0)),
            0x6A => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::D)), Cycles(1.0)),
            0x6B => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::E)), Cycles(1.0)),
            0x6C => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::H)), Cycles(1.0)),
            0x6D => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::L)), Cycles(1.0)),
            0x6E => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::HL)), Cycles(2.0)),
            0x70 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::B)), Cycles(2.0)),
            0x71 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::C)), Cycles(2.0)),
            0x72 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::D)), Cycles(2.0)),
            0x73 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::E)), Cycles(2.0)),
            0x74 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::H)), Cycles(2.0)),
            0x75 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::L)), Cycles(2.0)),
            0x47 => (Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::A)), Cycles(1.0)),
            0x4F => (Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::A)), Cycles(1.0)),
            0x57 => (Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::A)), Cycles(1.0)),
            0x5F => (Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::A)), Cycles(1.0)),
            0x67 => (Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::A)), Cycles(1.0)),
            0x6F => (Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::A)), Cycles(1.0)),
            0x02 => (Opcode::LD(OpcodeParameter::Register_Register(Register::BC, Register::A)), Cycles(2.0)),
            0x12 => (Opcode::LD(OpcodeParameter::Register_Register(Register::DE, Register::A)), Cycles(2.0)),
            0x77 => (Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::A)), Cycles(2.0)),
            0x36 => (Opcode::LD(OpcodeParameter::Register_U8(Register::HL, self.1)), Cycles(3.0)),
            0x0A => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::BC)), Cycles(2.0)),
            0x1A => (Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::DE)), Cycles(2.0)),
            0xFA => (Opcode::LD(OpcodeParameter::Register_U16(Register::A, two_byte_param)), Cycles(4.0)),
            0x3E => (Opcode::LD(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0xEA => (Opcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::A)), Cycles(4.0)),
            0xF2 => (Opcode::LD(OpcodeParameter::Register_FF00plusRegister(Register::A, Register::C)), Cycles(2.0)),
            0xE2 => (Opcode::LD(OpcodeParameter::FF00plusRegister_Register(Register::C, Register::A)), Cycles(2.0)),
            0x3A => (Opcode::LDD(OpcodeParameter::Register_RegisterDecrement(Register::A, Register::HL)), Cycles(2.0)),
            0x32 => (Opcode::LDD(OpcodeParameter::RegisterDecrement_Register(Register::HL, Register::A)), Cycles(2.0)),
            0x2A => (Opcode::LDI(OpcodeParameter::Register_RegisterIncrement(Register::A, Register::HL)), Cycles(2.0)),
            0x22 => (Opcode::LDI(OpcodeParameter::RegisterIncrement_Register(Register::HL, Register::A)), Cycles(2.0)),
            0xE0 => (Opcode::LD(OpcodeParameter::FF00plusU8_Register(self.1, Register::A)), Cycles(3.0)),
            0xF0 => (Opcode::LD(OpcodeParameter::Register_FF00plusU8(Register::A, self.1)), Cycles(3.0)),
            0x01 => (Opcode::LD(OpcodeParameter::Register_U16(Register::BC, two_byte_param)), Cycles(3.0)),
            0x11 => (Opcode::LD(OpcodeParameter::Register_U16(Register::DE, two_byte_param)), Cycles(3.0)),
            0x21 => (Opcode::LD(OpcodeParameter::Register_U16(Register::HL, two_byte_param)), Cycles(3.0)),
            0x31 => (Opcode::LD(OpcodeParameter::Register_U16(Register::SP, two_byte_param)), Cycles(3.0)),
            0xF9 => (Opcode::LD(OpcodeParameter::Register_Register(Register::SP, Register::HL)), Cycles(2.0)),
            0xF8 => (Opcode::LD(OpcodeParameter::Register_RegisterPlusI8(Register::HL, Register::SP, self.1 as i8)), Cycles(3.0)),
            0x08 => (Opcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::SP)), Cycles(5.0)),
            0xC5 => (Opcode::PUSH(Register::BC), Cycles(4.0)),
            0xD5 => (Opcode::PUSH(Register::DE), Cycles(4.0)),
            0xE5 => (Opcode::PUSH(Register::HL), Cycles(4.0)),
            0xF5 => (Opcode::PUSH(Register::AF), Cycles(4.0)),
            0xC1 => (Opcode::POP(Register::BC), Cycles(3.0)),
            0xD1 => (Opcode::POP(Register::DE), Cycles(3.0)),
            0xE1 => (Opcode::POP(Register::HL), Cycles(3.0)),
            0xF1 => (Opcode::POP(Register::AF), Cycles(3.0)),
            0x87 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0x80 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0x81 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0x82 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0x83 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0x84 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0x85 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0x86 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xC6 => (Opcode::ADD(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0x09 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::BC)), Cycles(2.0)),
            0x19 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::DE)), Cycles(2.0)),
            0x29 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::HL)), Cycles(2.0)),
            0x39 => (Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::SP)), Cycles(2.0)),
            0xE8 => (Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, self.1 as i8)), Cycles(4.0)),
            0x8F => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0x88 => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0x89 => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0x8A => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0x8B => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0x8C => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0x8D => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0x8E => (Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xCE => (Opcode::ADC(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0x97 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0x90 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0x91 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0x92 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0x93 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0x94 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0x95 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0x96 => (Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xD6 => (Opcode::SUB(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0x9F => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0x98 => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0x99 => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0x9A => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0x9B => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0x9C => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0x9D => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0x9E => (Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xDE => (Opcode::SBC(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0xA7 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0xA0 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0xA1 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0xA2 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0xA3 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0xA4 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0xA5 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0xA6 => (Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xE6 => (Opcode::AND(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0xB7 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0xB0 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0xB1 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0xB2 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0xB3 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0xB4 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0xB5 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0xB6 => (Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xF6 => (Opcode::OR(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0xAF => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0xA8 => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0xA9 => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0xAA => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0xAB => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0xAC => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0xAD => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0xAE => (Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xEE => (Opcode::XOR(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0xBF => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::A)), Cycles(1.0)),
            0xB8 => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::B)), Cycles(1.0)),
            0xB9 => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::C)), Cycles(1.0)),
            0xBA => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::D)), Cycles(1.0)),
            0xBB => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::E)), Cycles(1.0)),
            0xBC => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::H)), Cycles(1.0)),
            0xBD => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::L)), Cycles(1.0)),
            0xBE => (Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::HL)), Cycles(2.0)),
            0xFE => (Opcode::CP(OpcodeParameter::Register_U8(Register::A, self.1)), Cycles(2.0)),
            0x3C => (Opcode::INC(true, false, Register::A), Cycles(1.0)),
            0x04 => (Opcode::INC(true, false, Register::B), Cycles(1.0)),
            0x0C => (Opcode::INC(true, false, Register::C), Cycles(1.0)),
            0x14 => (Opcode::INC(true, false, Register::D), Cycles(1.0)),
            0x1C => (Opcode::INC(true, false, Register::E), Cycles(1.0)),
            0x24 => (Opcode::INC(true, false, Register::H), Cycles(1.0)),
            0x2C => (Opcode::INC(true, false, Register::L), Cycles(1.0)),
            0x34 => (Opcode::INC(true, true, Register::HL), Cycles(3.0)),
            0x03 => (Opcode::INC(false, false, Register::BC), Cycles(2.0)),
            0x13 => (Opcode::INC(false, false, Register::DE), Cycles(2.0)),
            0x23 => (Opcode::INC(false, false, Register::HL), Cycles(2.0)),
            0x33 => (Opcode::INC(false, false, Register::SP), Cycles(2.0)),
            0x3D => (Opcode::DEC(true, false, Register::A), Cycles(1.0)),
            0x05 => (Opcode::DEC(true, false, Register::B), Cycles(1.0)),
            0x0D => (Opcode::DEC(true, false, Register::C), Cycles(1.0)),
            0x15 => (Opcode::DEC(true, false, Register::D), Cycles(1.0)),
            0x1D => (Opcode::DEC(true, false, Register::E), Cycles(1.0)),
            0x25 => (Opcode::DEC(true, false, Register::H), Cycles(1.0)),
            0x2D => (Opcode::DEC(true, false, Register::L), Cycles(1.0)),
            0x35 => (Opcode::DEC(true, true, Register::HL), Cycles(3.0)),
            0x0B => (Opcode::DEC(false, false, Register::BC), Cycles(2.0)),
            0x1B => (Opcode::DEC(false, false, Register::DE), Cycles(2.0)),
            0x2B => (Opcode::DEC(false, false, Register::HL), Cycles(2.0)),
            0x3B => (Opcode::DEC(false, false, Register::SP), Cycles(2.0)),
            0x27 => (Opcode::DAA, Cycles(1.0)),
            0x2F => (Opcode::CPL, Cycles(1.0)),
            0x3F => (Opcode::CCF, Cycles(1.0)),
            0x37 => (Opcode::SCF, Cycles(1.0)),
            0x17 => (Opcode::RLA, Cycles(1.0)),
            0x07 => (Opcode::RLCA, Cycles(1.0)),
            0x0F => (Opcode::RRCA, Cycles(1.0)),
            0x1F => (Opcode::RRA, Cycles(1.0)),
            0xCB => match self.1 {
                0x00 => (Opcode::PrefixCB(CBOpcode::RLC(Register::B)), Cycles(2.0)),
                0x01 => (Opcode::PrefixCB(CBOpcode::RLC(Register::C)), Cycles(2.0)),
                0x02 => (Opcode::PrefixCB(CBOpcode::RLC(Register::D)), Cycles(2.0)),
                0x03 => (Opcode::PrefixCB(CBOpcode::RLC(Register::E)), Cycles(2.0)),
                0x04 => (Opcode::PrefixCB(CBOpcode::RLC(Register::H)), Cycles(2.0)),
                0x05 => (Opcode::PrefixCB(CBOpcode::RLC(Register::L)), Cycles(2.0)),
                0x06 => (Opcode::PrefixCB(CBOpcode::RLC(Register::HL)), Cycles(4.0)),
                0x07 => (Opcode::PrefixCB(CBOpcode::RLC(Register::A)), Cycles(2.0)),

                0x08 => (Opcode::PrefixCB(CBOpcode::RRC(Register::B)), Cycles(2.0)),
                0x09 => (Opcode::PrefixCB(CBOpcode::RRC(Register::C)), Cycles(2.0)),
                0x0A => (Opcode::PrefixCB(CBOpcode::RRC(Register::D)), Cycles(2.0)),
                0x0B => (Opcode::PrefixCB(CBOpcode::RRC(Register::E)), Cycles(2.0)),
                0x0C => (Opcode::PrefixCB(CBOpcode::RRC(Register::H)), Cycles(2.0)),
                0x0D => (Opcode::PrefixCB(CBOpcode::RRC(Register::L)), Cycles(2.0)),
                0x0E => (Opcode::PrefixCB(CBOpcode::RRC(Register::HL)), Cycles(4.0)),
                0x0F => (Opcode::PrefixCB(CBOpcode::RRC(Register::A)), Cycles(2.0)),

                0x10 => (Opcode::PrefixCB(CBOpcode::RL(Register::B)), Cycles(2.0)),
                0x11 => (Opcode::PrefixCB(CBOpcode::RL(Register::C)), Cycles(2.0)),
                0x12 => (Opcode::PrefixCB(CBOpcode::RL(Register::D)), Cycles(2.0)),
                0x13 => (Opcode::PrefixCB(CBOpcode::RL(Register::E)), Cycles(2.0)),
                0x14 => (Opcode::PrefixCB(CBOpcode::RL(Register::H)), Cycles(2.0)),
                0x15 => (Opcode::PrefixCB(CBOpcode::RL(Register::L)), Cycles(2.0)),
                0x16 => (Opcode::PrefixCB(CBOpcode::RL(Register::HL)), Cycles(4.0)),
                0x17 => (Opcode::PrefixCB(CBOpcode::RL(Register::A)), Cycles(2.0)),

                0x18 => (Opcode::PrefixCB(CBOpcode::RR(Register::B)), Cycles(2.0)),
                0x19 => (Opcode::PrefixCB(CBOpcode::RR(Register::C)), Cycles(2.0)),
                0x1A => (Opcode::PrefixCB(CBOpcode::RR(Register::D)), Cycles(2.0)),
                0x1B => (Opcode::PrefixCB(CBOpcode::RR(Register::E)), Cycles(2.0)),
                0x1C => (Opcode::PrefixCB(CBOpcode::RR(Register::H)), Cycles(2.0)),
                0x1D => (Opcode::PrefixCB(CBOpcode::RR(Register::L)), Cycles(2.0)),
                0x1E => (Opcode::PrefixCB(CBOpcode::RR(Register::HL)), Cycles(4.0)),
                0x1F => (Opcode::PrefixCB(CBOpcode::RR(Register::A)), Cycles(2.0)),

                0x20 => (Opcode::PrefixCB(CBOpcode::SLA(Register::B)), Cycles(2.0)),
                0x21 => (Opcode::PrefixCB(CBOpcode::SLA(Register::C)), Cycles(2.0)),
                0x22 => (Opcode::PrefixCB(CBOpcode::SLA(Register::D)), Cycles(2.0)),
                0x23 => (Opcode::PrefixCB(CBOpcode::SLA(Register::E)), Cycles(2.0)),
                0x24 => (Opcode::PrefixCB(CBOpcode::SLA(Register::H)), Cycles(2.0)),
                0x25 => (Opcode::PrefixCB(CBOpcode::SLA(Register::L)), Cycles(2.0)),
                0x26 => (Opcode::PrefixCB(CBOpcode::SLA(Register::HL)), Cycles(4.0)),
                0x27 => (Opcode::PrefixCB(CBOpcode::SLA(Register::A)), Cycles(2.0)),

                0x28 => (Opcode::PrefixCB(CBOpcode::SRA(Register::B)), Cycles(2.0)),
                0x29 => (Opcode::PrefixCB(CBOpcode::SRA(Register::C)), Cycles(2.0)),
                0x2A => (Opcode::PrefixCB(CBOpcode::SRA(Register::D)), Cycles(2.0)),
                0x2B => (Opcode::PrefixCB(CBOpcode::SRA(Register::E)), Cycles(2.0)),
                0x2C => (Opcode::PrefixCB(CBOpcode::SRA(Register::H)), Cycles(2.0)),
                0x2D => (Opcode::PrefixCB(CBOpcode::SRA(Register::L)), Cycles(2.0)),
                0x2E => (Opcode::PrefixCB(CBOpcode::SRA(Register::HL)), Cycles(4.0)),
                0x2F => (Opcode::PrefixCB(CBOpcode::SRA(Register::A)), Cycles(2.0)),

                0x30 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::B)), Cycles(2.0)),
                0x31 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::C)), Cycles(2.0)),
                0x32 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::D)), Cycles(2.0)),
                0x33 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::E)), Cycles(2.0)),
                0x34 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::H)), Cycles(2.0)),
                0x35 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::L)), Cycles(2.0)),
                0x36 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::HL)), Cycles(4.0)),
                0x37 => (Opcode::PrefixCB(CBOpcode::SWAP(Register::A)), Cycles(2.0)),

                0x38 => (Opcode::PrefixCB(CBOpcode::SRL(Register::B)), Cycles(2.0)),
                0x39 => (Opcode::PrefixCB(CBOpcode::SRL(Register::C)), Cycles(2.0)),
                0x3A => (Opcode::PrefixCB(CBOpcode::SRL(Register::D)), Cycles(2.0)),
                0x3B => (Opcode::PrefixCB(CBOpcode::SRL(Register::E)), Cycles(2.0)),
                0x3C => (Opcode::PrefixCB(CBOpcode::SRL(Register::H)), Cycles(2.0)),
                0x3D => (Opcode::PrefixCB(CBOpcode::SRL(Register::L)), Cycles(2.0)),
                0x3E => (Opcode::PrefixCB(CBOpcode::SRL(Register::HL)), Cycles(4.0)),
                0x3F => (Opcode::PrefixCB(CBOpcode::SRL(Register::A)), Cycles(2.0)),

                0x40 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::B)), Cycles(2.0)),
                0x41 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::C)), Cycles(2.0)),
                0x42 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::D)), Cycles(2.0)),
                0x43 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::E)), Cycles(2.0)),
                0x44 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::H)), Cycles(2.0)),
                0x45 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::L)), Cycles(2.0)),
                0x46 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::HL)), Cycles(3.0)),
                0x47 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0, Register::A)), Cycles(2.0)),
                0x48 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::B)), Cycles(2.0)),
                0x49 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::C)), Cycles(2.0)),
                0x4A => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::D)), Cycles(2.0)),
                0x4B => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::E)), Cycles(2.0)),
                0x4C => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::H)), Cycles(2.0)),
                0x4D => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::L)), Cycles(2.0)),
                0x4E => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::HL)), Cycles(3.0)),
                0x4F => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I1, Register::A)), Cycles(2.0)),
                0x50 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::B)), Cycles(2.0)),
                0x51 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::C)), Cycles(2.0)),
                0x52 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::D)), Cycles(2.0)),
                0x53 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::E)), Cycles(2.0)),
                0x54 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::H)), Cycles(2.0)),
                0x55 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::L)), Cycles(2.0)),
                0x56 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::HL)), Cycles(3.0)),
                0x57 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I2, Register::A)), Cycles(2.0)),
                0x58 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::B)), Cycles(2.0)),
                0x59 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::C)), Cycles(2.0)),
                0x5A => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::D)), Cycles(2.0)),
                0x5B => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::E)), Cycles(2.0)),
                0x5C => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::H)), Cycles(2.0)),
                0x5D => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::L)), Cycles(2.0)),
                0x5E => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::HL)), Cycles(3.0)),
                0x5F => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::A)), Cycles(2.0)),
                0x60 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::B)), Cycles(2.0)),
                0x61 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::C)), Cycles(2.0)),
                0x62 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::D)), Cycles(2.0)),
                0x63 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::E)), Cycles(2.0)),
                0x64 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::H)), Cycles(2.0)),
                0x65 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::L)), Cycles(2.0)),
                0x66 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::HL)), Cycles(3.0)),
                0x67 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::A)), Cycles(2.0)),
                0x68 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::B)), Cycles(2.0)),
                0x69 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::C)), Cycles(2.0)),
                0x6A => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::D)), Cycles(2.0)),
                0x6B => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::E)), Cycles(2.0)),
                0x6C => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::H)), Cycles(2.0)),
                0x6D => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::L)), Cycles(2.0)),
                0x6E => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::HL)), Cycles(3.0)),
                0x6F => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I5, Register::A)), Cycles(2.0)),
                0x70 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::B)), Cycles(2.0)),
                0x71 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::C)), Cycles(2.0)),
                0x72 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::D)), Cycles(2.0)),
                0x73 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::E)), Cycles(2.0)),
                0x74 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::H)), Cycles(2.0)),
                0x75 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::L)), Cycles(2.0)),
                0x76 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::HL)), Cycles(3.0)),
                0x77 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I6, Register::A)), Cycles(2.0)),
                0x78 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::B)), Cycles(2.0)),
                0x79 => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::C)), Cycles(2.0)),
                0x7A => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::D)), Cycles(2.0)),
                0x7B => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::E)), Cycles(2.0)),
                0x7C => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::H)), Cycles(2.0)),
                0x7D => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::L)), Cycles(2.0)),
                0x7E => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::HL)), Cycles(3.0)),
                0x7F => (Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I7, Register::A)), Cycles(2.0)),

                0x80 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::B)), Cycles(2.0)),
                0x81 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::C)), Cycles(2.0)),
                0x82 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::D)), Cycles(2.0)),
                0x83 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::E)), Cycles(2.0)),
                0x84 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::H)), Cycles(2.0)),
                0x85 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::L)), Cycles(2.0)),
                0x86 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::HL)), Cycles(4.0)),
                0x87 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0, Register::A)), Cycles(2.0)),
                0x88 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::B)), Cycles(2.0)),
                0x89 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::C)), Cycles(2.0)),
                0x8A => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::D)), Cycles(2.0)),
                0x8B => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::E)), Cycles(2.0)),
                0x8C => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::H)), Cycles(2.0)),
                0x8D => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::L)), Cycles(2.0)),
                0x8E => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::HL)), Cycles(4.0)),
                0x8F => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I1, Register::A)), Cycles(2.0)),
                0x90 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::B)), Cycles(2.0)),
                0x91 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::C)), Cycles(2.0)),
                0x92 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::D)), Cycles(2.0)),
                0x93 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::E)), Cycles(2.0)),
                0x94 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::H)), Cycles(2.0)),
                0x95 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::L)), Cycles(2.0)),
                0x96 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::HL)), Cycles(4.0)),
                0x97 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::A)), Cycles(2.0)),
                0x98 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::B)), Cycles(2.0)),
                0x99 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::C)), Cycles(2.0)),
                0x9A => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::D)), Cycles(2.0)),
                0x9B => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::E)), Cycles(2.0)),
                0x9C => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::H)), Cycles(2.0)),
                0x9D => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::L)), Cycles(2.0)),
                0x9E => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::HL)), Cycles(4.0)),
                0x9F => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I3, Register::A)), Cycles(2.0)),
                0xA0 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::B)), Cycles(2.0)),
                0xA1 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::C)), Cycles(2.0)),
                0xA2 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::D)), Cycles(2.0)),
                0xA3 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::E)), Cycles(2.0)),
                0xA4 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::H)), Cycles(2.0)),
                0xA5 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::L)), Cycles(2.0)),
                0xA6 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::HL)), Cycles(4.0)),
                0xA7 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I4, Register::A)), Cycles(2.0)),
                0xA8 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::B)), Cycles(2.0)),
                0xA9 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::C)), Cycles(2.0)),
                0xAA => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::D)), Cycles(2.0)),
                0xAB => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::E)), Cycles(2.0)),
                0xAC => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::H)), Cycles(2.0)),
                0xAD => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::L)), Cycles(2.0)),
                0xAE => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::HL)), Cycles(4.0)),
                0xAF => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I5, Register::A)), Cycles(2.0)),
                0xB0 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::B)), Cycles(2.0)),
                0xB1 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::C)), Cycles(2.0)),
                0xB2 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::D)), Cycles(2.0)),
                0xB3 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::E)), Cycles(2.0)),
                0xB4 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::H)), Cycles(2.0)),
                0xB5 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::L)), Cycles(2.0)),
                0xB6 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::HL)), Cycles(4.0)),
                0xB7 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I6, Register::A)), Cycles(2.0)),
                0xB8 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::B)), Cycles(2.0)),
                0xB9 => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::C)), Cycles(2.0)),
                0xBA => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::D)), Cycles(2.0)),
                0xBB => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::E)), Cycles(2.0)),
                0xBC => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::H)), Cycles(2.0)),
                0xBD => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::L)), Cycles(2.0)),
                0xBE => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::HL)), Cycles(4.0)),
                0xBF => (Opcode::PrefixCB(CBOpcode::RES(BitIndex::I7, Register::A)), Cycles(2.0)),

                0xC0 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::B)), Cycles(2.0)),
                0xC1 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::C)), Cycles(2.0)),
                0xC2 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::D)), Cycles(2.0)),
                0xC3 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::E)), Cycles(2.0)),
                0xC4 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::H)), Cycles(2.0)),
                0xC5 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::L)), Cycles(2.0)),
                0xC6 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::HL)), Cycles(4.0)),
                0xC7 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I0, Register::A)), Cycles(2.0)),
                0xC8 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::B)), Cycles(2.0)),
                0xC9 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::C)), Cycles(2.0)),
                0xCA => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::D)), Cycles(2.0)),
                0xCB => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::E)), Cycles(2.0)),
                0xCC => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::H)), Cycles(2.0)),
                0xCD => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::L)), Cycles(2.0)),
                0xCE => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::HL)), Cycles(4.0)),
                0xCF => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::A)), Cycles(2.0)),
                0xD0 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::B)), Cycles(2.0)),
                0xD1 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::C)), Cycles(2.0)),
                0xD2 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::D)), Cycles(2.0)),
                0xD3 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::E)), Cycles(2.0)),
                0xD4 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::H)), Cycles(2.0)),
                0xD5 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::L)), Cycles(2.0)),
                0xD6 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::HL)), Cycles(4.0)),
                0xD7 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I2, Register::A)), Cycles(2.0)),
                0xD8 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::B)), Cycles(2.0)),
                0xD9 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::C)), Cycles(2.0)),
                0xDA => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::D)), Cycles(2.0)),
                0xDB => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::E)), Cycles(2.0)),
                0xDC => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::H)), Cycles(2.0)),
                0xDD => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::L)), Cycles(2.0)),
                0xDE => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::HL)), Cycles(4.0)),
                0xDF => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3, Register::A)), Cycles(2.0)),
                0xE0 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::B)), Cycles(2.0)),
                0xE1 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::C)), Cycles(2.0)),
                0xE2 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::D)), Cycles(2.0)),
                0xE3 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::E)), Cycles(2.0)),
                0xE4 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::H)), Cycles(2.0)),
                0xE5 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::L)), Cycles(2.0)),
                0xE6 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::HL)), Cycles(4.0)),
                0xE7 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I4, Register::A)), Cycles(2.0)),
                0xE8 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::B)), Cycles(2.0)),
                0xE9 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::C)), Cycles(2.0)),
                0xEA => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::D)), Cycles(2.0)),
                0xEB => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::E)), Cycles(2.0)),
                0xEC => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::H)), Cycles(2.0)),
                0xED => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::L)), Cycles(2.0)),
                0xEE => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::HL)), Cycles(4.0)),
                0xEF => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I5, Register::A)), Cycles(2.0)),
                0xF0 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::B)), Cycles(2.0)),
                0xF1 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::C)), Cycles(2.0)),
                0xF2 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::D)), Cycles(2.0)),
                0xF3 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::E)), Cycles(2.0)),
                0xF4 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::H)), Cycles(2.0)),
                0xF5 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::L)), Cycles(2.0)),
                0xF6 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::HL)), Cycles(4.0)),
                0xF7 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I6, Register::A)), Cycles(2.0)),
                0xF8 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::B)), Cycles(2.0)),
                0xF9 => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::C)), Cycles(2.0)),
                0xFA => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::D)), Cycles(2.0)),
                0xFB => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::E)), Cycles(2.0)),
                0xFC => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::H)), Cycles(2.0)),
                0xFD => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::L)), Cycles(2.0)),
                0xFE => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::HL)), Cycles(4.0)),
                0xFF => (Opcode::PrefixCB(CBOpcode::SET(BitIndex::I7, Register::A)), Cycles(2.0)),
            },
            0xC3 => (Opcode::JP(OpcodeParameter::U16(two_byte_param)), Cycles(4.0)),
            0xC2 => (Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, two_byte_param)), Cycles(3.0)),
            0xCA => (Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, two_byte_param)), Cycles(3.0)),
            0xD2 => (Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry, two_byte_param)), Cycles(3.0)),
            0xDA => (Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry, two_byte_param)), Cycles(3.0)),
            0xE9 => (Opcode::JP(OpcodeParameter::Register(Register::HL)), Cycles(1.0)),
            0x18 => (Opcode::JR(OpcodeParameter::I8(self.1 as i8)), Cycles(2.0)),
            0x20 => (Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Zero, self.1 as i8)), Cycles(2.0)),
            0x28 => (Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Zero, self.1 as i8)), Cycles(2.0)),
            0x30 => (Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Carry, self.1 as i8)), Cycles(2.0)),
            0x38 => (Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Carry, self.1 as i8)), Cycles(2.0)),
            0xCD => (Opcode::CALL(OpcodeParameter::U16(two_byte_param)), Cycles(6.0)),
            0xC4 => (Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, two_byte_param)), Cycles(3.0)),
            0xCC => (Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, two_byte_param)), Cycles(3.0)),
            0xD4 => (Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry, two_byte_param)), Cycles(3.0)),
            0xDC => (Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry, two_byte_param)), Cycles(3.0)),
            0xC7 => (Opcode::RST(0x00), Cycles(4.0)),
            0xCF => (Opcode::RST(0x08), Cycles(4.0)),
            0xD7 => (Opcode::RST(0x10), Cycles(4.0)),
            0xDF => (Opcode::RST(0x18), Cycles(4.0)),
            0xE7 => (Opcode::RST(0x20), Cycles(4.0)),
            0xEF => (Opcode::RST(0x28), Cycles(4.0)),
            0xF7 => (Opcode::RST(0x30), Cycles(4.0)),
            0xFF => (Opcode::RST(0x38), Cycles(4.0)),
            0xC9 => (Opcode::RET(OpcodeParameter::NoParam), Cycles(4.0)),
            0xC0 => (Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Zero)), Cycles(2.0)),
            0xC8 => (Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Zero)), Cycles(2.0)),
            0xD0 => (Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Carry)), Cycles(2.0)),
            0xD8 => (Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Carry)), Cycles(2.0)),
            0xD9 => (Opcode::RETI, Cycles(4.0)),
            0xF3 => (Opcode::DI, Cycles(1.0)),
            0xFB => (Opcode::EI, Cycles(1.0)),
            0x76 => (Opcode::HALT, Cycles(1.0)),
            0x10 => (Opcode::STOP, Cycles(1.0)),
            0x00 => (Opcode::NOP, Cycles(1.0)),
            _ => (Opcode::IllegalInstruction, Cycles(1.0)),
        }
    }
}

#[derive(Debug)]
pub enum Opcode {
    LD(OpcodeParameter),
    LDD(OpcodeParameter),
    LDI(OpcodeParameter),
    LDHL(OpcodeParameter),
    PUSH(Register),
    POP(Register),
    ADD(OpcodeParameter),
    ADC(OpcodeParameter),
    SUB(OpcodeParameter),
    SBC(OpcodeParameter),
    AND(OpcodeParameter),
    OR(OpcodeParameter),
    XOR(OpcodeParameter),
    CP(OpcodeParameter),
    INC(bool, bool, Register),
    DEC(bool, bool, Register),
    DAA,
    CPL,
    CCF,
    SCF,
    NOP,
    HALT,
    STOP,
    DI,
    EI,
    RLCA,
    RLA,
    RRCA,
    RRA,
    JP(OpcodeParameter),
    JR(OpcodeParameter),
    CALL(OpcodeParameter),
    RST(u8),
    RET(OpcodeParameter),
    RETI,
    PrefixCB(CBOpcode),
    IllegalInstruction,
}

#[derive(Debug)]
pub enum CBOpcode {
    SWAP(Register),
    RLC(Register),
    RL(Register),
    RRC(Register),
    RR(Register),
    SLA(Register),
    SRA(Register),
    SRL(Register),
    BIT(BitIndex, Register),
    SET(BitIndex, Register),
    RES(BitIndex, Register),
}

// Store cycles in M
#[derive(Debug, Copy, Clone)]
pub struct Cycles(pub f32); 

impl Cycles {
    pub fn to_t(&self) -> Self {
        Self(self.0 * 4.0)
    }
}

pub struct CPU {
    registers: Registers,
    cycles: Cycles,
    last_op_cycles: Cycles,
    exec_calls_count: usize,
    is_halted: bool,
    ime: bool, // Interrupt Master Enable
    ei_delay: bool,
    enable_logs: bool,
    is_cgb: bool,
    double_speed_mode: bool,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            cycles: Cycles(0.0),
            last_op_cycles: Cycles(0.0),
            exec_calls_count: 0,
            is_halted: false,
            ei_delay: false,
            ime: true,
            enable_logs: !env::var("CPU_LOG").is_err() || !env::var("CPU_LOGS").is_err(),
            is_cgb: false,
            double_speed_mode: false,
        }
    }

    pub fn new_cgb() -> Self {
        Self {
            registers: Registers::new_cgb(),
            cycles: Cycles(0.0),
            last_op_cycles: Cycles(0.0),
            exec_calls_count: 0,
            is_halted: false,
            ei_delay: false,
            ime: true,
            enable_logs: !env::var("CPU_LOG").is_err() || !env::var("CPU_LOGS").is_err(),
            is_cgb: true,
            double_speed_mode: false,
        }
    }

    pub fn get_exec_calls_count(&self) -> usize {
        self.exec_calls_count
    }

    fn increment_exec_calls_count(&mut self) {
        self.exec_calls_count += 1;
    }

    fn increment_cycles(&mut self, cycles: Cycles) {
        if self.is_cgb && self.double_speed_mode {
            self.cycles.0 += cycles.0 / 2.0;
        } else {
            self.cycles.0 += cycles.0;
        }
    }

    pub fn reset_cycles(&mut self) {
        self.cycles = Cycles(0.0);
    }

    pub fn get_cycles(&mut self) -> Cycles {
        self.cycles
    }

    pub fn set_last_op_cycles(&mut self, cycles_start: Cycles, cycles_end: Cycles) {
        self.last_op_cycles = Cycles(cycles_end.0 - cycles_start.0);
    }

    pub fn get_last_op_cycles(&self) -> Cycles {
        self.last_op_cycles
    }

    fn log(&self, parameter_bytes: OpcodeParameterBytes) {
        println!("A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X} {:02X} {:02X} {:02X})",
            self.registers.get(Register::A),
            self.registers.get(Register::F),
            self.registers.get(Register::B),
            self.registers.get(Register::C),
            self.registers.get(Register::D),
            self.registers.get(Register::E),
            self.registers.get(Register::H),
            self.registers.get(Register::L),
            self.registers.get(Register::SP),
            self.registers.get(Register::PC),
            parameter_bytes.0,
            parameter_bytes.1,
            parameter_bytes.2,
            parameter_bytes.3,
        );
    }

    pub fn handle_interrupt(&mut self, bus: &mut Bus, interrupt: Interrupt) {
        bus.interrupts.set(interrupt, false);
        self.ime = false;
        self.registers.decrement(Register::PC, 3);
        self.exec(Opcode::CALL(OpcodeParameter::U16(interrupt.get_vector())), bus);
    }

    pub fn check_interrupts(&mut self, bus: &mut Bus) -> Option<Interrupt> {
        let interrupts = (bus.read(INTERRUPT_ENABLE_ADDRESS) & 0b00011111) & (bus.read(INTERRUPT_FLAG_ADDRESS) & 0b00011111);
        if interrupts != 0 {
            self.is_halted = false;
        }
        if !self.ime || interrupts == 0 {
            return None;
        }

        if Interrupt::VBlank.get(interrupts) {
            return Some(Interrupt::VBlank);
        } else if Interrupt::LCDSTAT.get(interrupts) {
            return Some(Interrupt::LCDSTAT);
        } else if Interrupt::Timer.get(interrupts) {
            return Some(Interrupt::Timer);
        } else if Interrupt::Serial.get(interrupts) {
            return Some(Interrupt::Serial);
        } else if Interrupt::Joypad.get(interrupts) {
            return Some(Interrupt::Joypad);
        }
        None
    }

    pub fn ei_delay(&mut self, bus: &mut Bus) {
        if self.ei_delay && !self.ime {
            self.ei_delay = false;
            self.run(bus);
            self.ime = true;
        }
    }

    pub fn run(&mut self, bus: &mut Bus) {
        let cycles_start = self.get_cycles();
        if let Some(interrupt) = self.check_interrupts(bus) {
            self.handle_interrupt(bus, interrupt);
            self.increment_cycles(Cycles(5.0));
        } else if !self.is_halted {
            let program_counter = self.registers.get(Register::PC);
            let parameter_bytes = OpcodeParameterBytes::from_address(program_counter, bus);
            let (opcode, cycles) = parameter_bytes.parse_opcode();
            if self.enable_logs {
                self.log(parameter_bytes);
                self.increment_exec_calls_count();
            }
            self.increment_cycles(cycles);
            self.exec(opcode, bus);
            self.ei_delay(bus);
        } else if self.is_halted {
            self.increment_cycles(Cycles(1.0));
        }
        let cycles_end = self.get_cycles();
        self.set_last_op_cycles(cycles_start, cycles_end);
    }

    pub fn exec(&mut self, opcode: Opcode, bus: &mut Bus) {
        match opcode {
            // Load
            Opcode::LD(params) => match params {
                OpcodeParameter::Register_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    if reg1.is_16bit() && reg2.is_8bit() {
                        let val = self.registers.get_8bit(reg2);
                        let addr = self.registers.get(reg1);
                        bus.write(addr, val);
                    } else if reg1.is_8bit() && reg2.is_16bit() {
                        let val = bus.read(self.registers.get(reg2)) as u16;
                        self.registers.set(reg1, val);
                    } else {
                        self.registers.set(reg1, self.registers.get(reg2));
                    }
                },
                OpcodeParameter::Register_U16(register, val) => {
                    self.registers.increment(Register::PC, 3);
                    match register.is_8bit() {
                        true => self.registers.set(register, bus.read(val) as u16),
                        false => self.registers.set(register, val),
                    };
                },
                OpcodeParameter::Register_U8(register, val) => {
                    self.registers.increment(Register::PC, 2);
                    match register.is_8bit() {
                        true => self.registers.set(register, val as u16),
                        false => bus.write(self.registers.get(register), val),
                    }
                },
                OpcodeParameter::U16_Register(address, register) => {
                    self.registers.increment(Register::PC, 3);
                    let value = self.registers.get(register);
                    let bytes = value.to_be_bytes();
                    match register.is_8bit() {
                        true => bus.write(address, bytes[1]),
                        false => bus.write_16bit(address, value),
                    };
                },
                OpcodeParameter::Register_FF00plusU8(register, val) => {
                    self.registers.increment(Register::PC, 2);
                    self.registers.set(register, bus.read(0xFF00 + (val as u16)) as u16);
                },
                OpcodeParameter::FF00plusU8_Register(val, register) => {
                    self.registers.increment(Register::PC, 2);
                    match register.is_8bit() {
                        true => bus.write(0xFF00 + (val as u16), self.registers.get_8bit(register)),
                        false => bus.write_16bit(0xFF00 + (val as u16), self.registers.get(register)),
                    }
                },
                OpcodeParameter::Register_FF00plusRegister(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let addr = 0xFF00 + self.registers.get(reg2);
                    self.registers.set(reg1, bus.read(addr) as u16);
                },
                OpcodeParameter::FF00plusRegister_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let addr = 0xFF00 + self.registers.get(reg1);
                    bus.write(addr, self.registers.get_8bit(reg2));
                },
                OpcodeParameter::Register_RegisterPlusI8(reg1, reg2, value) => {
                    self.registers.increment(Register::PC, 2);
                    let reg2_value = self.registers.get(reg2);
                    self.registers.set_flag(FlagRegister::Zero, false);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::Carry, (reg2_value & 0x00FF) + ((value as u16) & 0x00FF) > 0xFF);
                    self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(reg2_value.to_be_bytes()[1], value as u8));
                    let res = (self.registers.get(reg2) as i16).wrapping_add(value as i16);
                    self.registers.set(reg1, res as u16);
                },
                _ => unreachable!(),
            },
            // Increment or decrement program counter by signed N
            Opcode::JR(params) => {
                self.registers.increment(Register::PC, 2);
                let (condition_met, value) = match params {
                    OpcodeParameter::I8(val) => (true, val as i8),
                    OpcodeParameter::FlagRegisterReset_I8(flag, val) => (!self.registers.get_flag(flag), val as i8),
                    OpcodeParameter::FlagRegisterSet_I8(flag, val) => (self.registers.get_flag(flag), val as i8),
                    _ => unreachable!(),
                };
                if condition_met {
                    self.increment_cycles(Cycles(1.0));
                    let pc = (self.registers.get(Register::PC) as i16) + (value as i16);
                    self.registers.set(Register::PC, pc as u16);
                }
            },
            // Load and increment
            Opcode::LDI(params) => match params {
                OpcodeParameter::Register_RegisterIncrement(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let val = bus.read(self.registers.get(reg2));
                    self.registers.set(reg1, val as u16);
                    self.registers.increment(reg2, 1);
                },
                OpcodeParameter::RegisterIncrement_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let val = self.registers.get_8bit(reg2);
                    bus.write(self.registers.get(reg1), val);
                    self.registers.increment(reg1, 1);
                },
                _ => unreachable!(),
            },
            // Load and decrement
            Opcode::LDD(params) => match params {
                OpcodeParameter::Register_RegisterDecrement(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let val = bus.read(self.registers.get(reg2));
                    self.registers.set(reg1, val as u16);
                    self.registers.decrement(reg2, 1);
                },
                OpcodeParameter::RegisterDecrement_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    let val = self.registers.get_8bit(reg2);
                    bus.write(self.registers.get(reg1), val);
                    self.registers.decrement(reg1, 1);
                },
                _ => unreachable!(),
            },
            Opcode::AND(params) => {
                match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        self.registers.increment(Register::PC, 1);
                        if reg2.is_8bit() {
                            self.registers.set(reg1, self.registers.get(reg1) & self.registers.get(reg2));
                        } else {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) & val);
                        }
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    },
                    OpcodeParameter::Register_U8(reg, val) => {
                        self.registers.increment(Register::PC, 2);
                        self.registers.set(reg, self.registers.get(reg) & (val as u16));
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    },
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, true);
                self.registers.set_flag(FlagRegister::Carry, false);
            },
            Opcode::OR(params) => {
                match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        self.registers.increment(Register::PC, 1);
                        if reg2.is_8bit() {
                            self.registers.set(reg1, self.registers.get(reg1) | self.registers.get(reg2));
                        } else {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) | val);
                        }
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    },
                    OpcodeParameter::Register_U8(reg, val) => {
                        self.registers.increment(Register::PC, 2);
                        self.registers.set(reg, self.registers.get(reg) | (val as u16));
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    },
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
                self.registers.set_flag(FlagRegister::Carry, false);
            },
            Opcode::XOR(params) => {
                match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        self.registers.increment(Register::PC, 1);
                        if reg2.is_8bit() {
                            self.registers.set(reg1, self.registers.get(reg1) ^ self.registers.get(reg2));
                        } else {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) ^ val);
                        }
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    },
                    OpcodeParameter::Register_U8(reg, val) => {
                        self.registers.increment(Register::PC, 2);
                        self.registers.set(reg, self.registers.get(reg) ^ (val as u16));
                        self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    },
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
                self.registers.set_flag(FlagRegister::Carry, false);
            },
            // Substract without storing the value
            Opcode::CP(params) => {
                let (val1, val2) = match params {
                    OpcodeParameter::Register_U8(register, val) => {
                        self.registers.increment(Register::PC, 2);
                        (self.registers.get(register) as i16, val as i16)
                    },
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        self.registers.increment(Register::PC, 1);
                        let val1 = self.registers.get(reg1) as i16;
                        let val2 = match reg2.is_8bit() {
                            true => self.registers.get(reg2) as i16,
                            false => bus.read(self.registers.get(reg2)) as i16,
                        };
                        (val1, val2)
                    }
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Zero, (val1 - val2) == 0);
                self.registers.set_flag(FlagRegister::Substract, true);
                self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(val1.to_be_bytes()[1], val2.to_be_bytes()[1]));
                self.registers.set_flag(FlagRegister::Carry, val2 > val1);
            },
            Opcode::ADD(params) => {
                self.registers.increment(Register::PC, 1);
                self.registers.set_flag(FlagRegister::Substract, false);
                match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        if reg1.is_8bit() && reg2.is_8bit() {
                            let res = (self.registers.get(reg1) as u16) + (self.registers.get(reg2) as u16);
                            self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(self.registers.get_8bit(reg1), self.registers.get_8bit(reg2)));
                            self.registers.increment(reg1, self.registers.get(reg2));
                            self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                            self.registers.set_flag(FlagRegister::Carry, res > 0xFF);
                        } else if reg1.is_16bit() && reg2.is_16bit() {
                            let val1 = self.registers.get(reg1);
                            let val2 = self.registers.get(reg2);
                            self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry_16bit(val1, val2));
                            let res = (val1 as usize) + (val2 as usize);
                            self.registers.increment(reg1, self.registers.get(reg2));
                            let carry = res > 0xFFFF;
                            self.registers.set_flag(FlagRegister::Carry, carry);
                        } else if reg1.is_8bit() && reg2.is_16bit() {
                            let val1 = self.registers.get(reg1);
                            let val2 = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.increment(reg1, val2);
                            self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(val1.to_be_bytes()[1], val2.to_be_bytes()[1]));
                            self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                            self.registers.set_flag(FlagRegister::Carry, (val1 as usize + val2 as usize) > 0xFF);
                        }
                    },
                    OpcodeParameter::Register_U8(reg1, val) => {
                        self.registers.increment(Register::PC, 1);
                        match reg1.is_8bit() {
                            true => {
                                let val1 = self.registers.get(reg1);
                                let val2 = val as u16;
                                self.registers.increment(reg1, val2);
                                self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(val1.to_be_bytes()[1], val2.to_be_bytes()[1]));
                                self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                                self.registers.set_flag(FlagRegister::Carry, (val1 as u16) + (val2 as u16) > 0xFF);
                            },
                            false => {
                                let addr = self.registers.get(reg1);
                                let val1 = bus.read(addr);
                                let val2 = val;
                                let res = val1.wrapping_add(val2);
                                bus.write(addr, res);
                                self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(val1, val2));
                                self.registers.set_flag(FlagRegister::Zero, res == 0);
                                self.registers.set_flag(FlagRegister::Carry, (val1 as u16) + (val2 as u16) > 0xFF);
                            },
                        };
                    },
                    OpcodeParameter::Register_I8(reg, value) => {
                        self.registers.increment(Register::PC, 1);
                        self.registers.set_flag(FlagRegister::Zero, false);
                        self.registers.set_flag(FlagRegister::Carry, (self.registers.get(reg) & 0x00FF) + ((value as u16) & 0x00FF) > 0xFF);
                        self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(self.registers.get_8bit(reg), value as u8));
                        let res = (self.registers.get(reg) as i16).wrapping_add(value as i16);
                        self.registers.set(reg, res as u16);
                    },
                    _ => unreachable!(),
                };
            },
            Opcode::ADC(params) => {
                let (carry_prev, half_carry_prev) = match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::ADD(OpcodeParameter::Register_Register(reg1, reg2)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::ADD(OpcodeParameter::Register_U8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    OpcodeParameter::Register_U8(reg1, val) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::ADD(OpcodeParameter::Register_U8(reg1, val)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::ADD(OpcodeParameter::Register_U8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    OpcodeParameter::Register_I8(reg1, val) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::ADD(OpcodeParameter::Register_I8(reg1, val)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::ADD(OpcodeParameter::Register_I8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Carry, carry_prev || self.registers.get_flag(FlagRegister::Carry));
                self.registers.set_flag(FlagRegister::HalfCarry, half_carry_prev || self.registers.get_flag(FlagRegister::HalfCarry));
            },
            Opcode::SUB(params) => {
                self.registers.increment(Register::PC, 1);
                let (register, val1, val2) = match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        let val1 = self.registers.get(reg1);
                        let val2 = match reg1.is_8bit() && reg2.is_8bit() {
                            true => self.registers.get(reg2),
                            false => bus.read(self.registers.get(reg2)) as u16,
                        };
                        (reg1, val1, val2)
                    },
                    OpcodeParameter::Register_U8(reg1, val) => {
                        self.registers.increment(Register::PC, 1);
                        (reg1, self.registers.get(reg1), val as u16)
                    },
                    _ => unreachable!(),
                };
                let carry = val2 > val1;
                let val1 = match carry {
                    true => val1 | 0x100,
                    false => val1,
                };
                let result = val1.wrapping_sub(val2);
                self.registers.set(register, result);
                self.registers.set_flag(FlagRegister::Zero, self.registers.get(register) == 0);
                self.registers.set_flag(FlagRegister::Substract, true);
                self.registers.set_flag(FlagRegister::Carry, carry);
                self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(val1.to_be_bytes()[1], val2.to_be_bytes()[1]));
            },
            Opcode::SBC(params) => {
                let (carry_prev, half_carry_prev) = match params {
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::SUB(OpcodeParameter::Register_Register(reg1, reg2)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::SUB(OpcodeParameter::Register_U8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    OpcodeParameter::Register_U8(reg1, val) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::SUB(OpcodeParameter::Register_U8(reg1, val)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::SUB(OpcodeParameter::Register_U8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    OpcodeParameter::Register_I8(reg1, val) => {
                        let carry = self.registers.get_flag(FlagRegister::Carry);
                        self.exec(Opcode::ADD(OpcodeParameter::Register_I8(reg1, val)), bus);
                        let carry_prev = self.registers.get_flag(FlagRegister::Carry);
                        let half_carry_prev = self.registers.get_flag(FlagRegister::HalfCarry);
                        if carry {
                            self.registers.decrement(Register::PC, 2);
                            self.exec(Opcode::SUB(OpcodeParameter::Register_I8(reg1, 1)), bus);
                        }
                        (carry_prev, half_carry_prev)
                    },
                    _ => unreachable!(),
                };
                self.registers.set_flag(FlagRegister::Carry, carry_prev || self.registers.get_flag(FlagRegister::Carry));
                self.registers.set_flag(FlagRegister::HalfCarry, half_carry_prev || self.registers.get_flag(FlagRegister::HalfCarry));
            },
            // Increment by 1
            Opcode::INC(affect_flags, on_address, register) => {
                self.registers.increment(Register::PC, 1);
                if on_address {
                    let addr = self.registers.get(register);
                    let prev_value = bus.read(addr);
                    bus.write(addr, prev_value.wrapping_add(1));
                    if affect_flags {
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(prev_value, 1));
                        self.registers.set_flag(FlagRegister::Zero, bus.read(addr) == 0);
                    }
                } else {
                    let prev_value = self.registers.get(register);
                    self.registers.increment(register, 1);
                    if affect_flags {
                        let byte_compare = match register.is_8bit() {
                            true => prev_value.to_be_bytes()[1],
                            false => prev_value.to_be_bytes()[0],
                        };
                        let result = self.registers.get(register);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(byte_compare, 1));
                        self.registers.set_flag(FlagRegister::Zero, result == 0);
                    }
                }
            },
            // Decrement by 1
            Opcode::DEC(affect_flags, on_address, register) => {
                self.registers.increment(Register::PC, 1);
                if on_address {
                    let addr = self.registers.get(register);
                    let prev_value = bus.read(addr);
                    bus.write(addr, prev_value.wrapping_sub(1));
                    if affect_flags {
                        self.registers.set_flag(FlagRegister::Substract, true);
                        self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(prev_value, 1));
                        self.registers.set_flag(FlagRegister::Zero, bus.read(addr) == 0);
                    }
                } else {
                    let prev_value = self.registers.get(register);
                    self.registers.decrement(register, 1);
                    if affect_flags {
                        let byte_compare = match register.is_8bit() {
                            true => prev_value.to_be_bytes()[1],
                            false => prev_value.to_be_bytes()[0],
                        };
                        let result = self.registers.get(register);
                        self.registers.set_flag(FlagRegister::Substract, true);
                        self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(byte_compare, 1));
                        self.registers.set_flag(FlagRegister::Zero, result == 0);
                    }
                }
            },
            // BCD correction
            Opcode::DAA => {
                self.registers.increment(Register::PC, 1);
                let mut val = self.registers.get_8bit(Register::A);
                if !self.registers.get_flag(FlagRegister::Substract) {
                    if self.registers.get_flag(FlagRegister::Carry) || val > 0x99 {
                        val = val.wrapping_add(0x60);
                        self.registers.set_flag(FlagRegister::Carry, true);
                    }
                    if self.registers.get_flag(FlagRegister::HalfCarry) || ((val & 0x0F) > 0x09) {
                        val = val.wrapping_add(0x6);
                    }
                } else {
                    if self.registers.get_flag(FlagRegister::Carry) {
                        val = val.wrapping_sub(0x60);
                    }
                    if self.registers.get_flag(FlagRegister::HalfCarry) {
                        val = val.wrapping_sub(0x6);
                    }
                }
                self.registers.set(Register::A, val as u16);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
                self.registers.set_flag(FlagRegister::Zero, val == 0);
            },
            // Jump to address
            Opcode::JP(params) => match params {
                OpcodeParameter::U16(address) => self.registers.set(Register::PC, address),
                OpcodeParameter::Register(register) => self.registers.set(Register::PC, self.registers.get(register)),
                OpcodeParameter::FlagRegisterReset_U16(flag, addr) => {
                    self.registers.increment(Register::PC, 3);
                    if !self.registers.get_flag(flag) {
                        self.registers.set(Register::PC, addr);
                        self.increment_cycles(Cycles(1.0));
                    }
                },
                OpcodeParameter::FlagRegisterSet_U16(flag, addr) => {
                    self.registers.increment(Register::PC, 3);
                    if self.registers.get_flag(flag) {
                        self.registers.set(Register::PC, addr);
                        self.increment_cycles(Cycles(1.0));
                    }
                },
                _ => unreachable!(),
            },
            // CALL
            Opcode::CALL(params) => {
                self.registers.increment(Register::PC, 3);
                let (condition_met, address) = match params {
                    OpcodeParameter::U16(address) => {
                        (true, address)
                    },
                    OpcodeParameter::FlagRegisterReset_U16(flag, address) => {
                        let condition_met = !self.registers.get_flag(flag);
                        if condition_met {self.increment_cycles(Cycles(3.0))};
                        (condition_met, address)
                    },
                    OpcodeParameter::FlagRegisterSet_U16(flag, address) => {
                        let condition_met = self.registers.get_flag(flag);
                        if condition_met {self.increment_cycles(Cycles(3.0))};
                        (condition_met, address)
                    },
                    _ => unreachable!(),
                };
                if condition_met {
                    let pc = self.registers.get(Register::PC);
                    self.registers.decrement(Register::SP, 2);
                    let sp = self.registers.get(Register::SP);
                    bus.write_16bit(sp, pc);
                    self.registers.set(Register::PC, address);
                }
            },
            // RST, same as Call
            Opcode::RST(address) => {
                self.registers.decrement(Register::PC, 2);
                self.exec(Opcode::CALL(OpcodeParameter::U16(address as u16)), bus);
            },
            // PUSH onto the stack
            Opcode::PUSH(register) => {
                self.registers.increment(Register::PC, 1);
                let val = self.registers.get(register).to_be_bytes();
                self.registers.decrement(Register::SP, 1);
                bus.write(self.registers.get(Register::SP), val[0]);
                self.registers.decrement(Register::SP, 1);
                bus.write(self.registers.get(Register::SP), val[1]);
            },
            // POP onto the stack
            Opcode::POP(register) => {
                self.registers.increment(Register::PC, 1);
                let sp = self.registers.get(Register::SP);
                let val = bus.read_16bit(sp);
                match register {
                    Register::AF => self.registers.set(register, val & 0xFFF0),
                    _ => self.registers.set(register, val),
                };
                self.registers.increment(Register::SP, 2);
            },
            // RET, same as POP PC when no parameter is specified
            Opcode::RET(params) => {
                self.registers.increment(Register::PC, 1);
                match params {
                    OpcodeParameter::NoParam => self.exec(Opcode::POP(Register::PC), bus),
                    OpcodeParameter::FlagRegisterReset(flag) => {
                        if !self.registers.get_flag(flag) {
                            self.exec(Opcode::POP(Register::PC), bus);
                            self.increment_cycles(Cycles(3.0));
                        }
                    },
                    OpcodeParameter::FlagRegisterSet(flag) => {
                        if self.registers.get_flag(flag) {
                            self.exec(Opcode::POP(Register::PC), bus);
                            self.increment_cycles(Cycles(3.0));
                        }
                    },
                    _ => unreachable!(),
                };
            },
            // Rotate A Left
            Opcode::RLCA => {
                self.registers.increment(Register::PC, 1);
                let val = self.registers.get_8bit(Register::A);
                self.registers.set_flag(FlagRegister::Carry, get_bit(val, BitIndex::I7));
                let result = val.rotate_left(1);
                self.registers.set(Register::A, result as u16);
                self.registers.set_flag(FlagRegister::Zero, false);
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            // Rotate A Right 
            Opcode::RRCA => {
                self.registers.increment(Register::PC, 1);
                let val = self.registers.get_8bit(Register::A);
                self.registers.set_flag(FlagRegister::Carry, get_bit(val, BitIndex::I0));
                let result = val.rotate_right(1);
                self.registers.set(Register::A, result as u16);
                self.registers.set_flag(FlagRegister::Zero, false);
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            Opcode::RLA => {
                self.registers.increment(Register::PC, 1);
                let val = self.registers.get_8bit(Register::A);
                let old_carry = self.registers.get_flag(FlagRegister::Carry);
                let new_carry = get_bit(val, BitIndex::I7);
                let val = val << 1 | (old_carry as u8);
                self.registers.set(Register::A, val as u16);
                self.registers.set_flag(FlagRegister::Carry, new_carry);
                self.registers.set_flag(FlagRegister::Zero, false);
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            Opcode::RRA => {
                self.registers.increment(Register::PC, 1);
                let val = self.registers.get_8bit(Register::A);
                let old_carry = self.registers.get_flag(FlagRegister::Carry);
                let new_carry = get_bit(val, BitIndex::I0);
                let val = val >> 1 | ((old_carry as u8) << 7);
                self.registers.set(Register::A, val as u16);
                self.registers.set_flag(FlagRegister::Carry, new_carry);
                self.registers.set_flag(FlagRegister::Zero, false);
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            Opcode::PrefixCB(opcode) => {
                self.registers.increment(Register::PC, 2);
                match opcode {
                    CBOpcode::RLC(register) => {
                        let (val, result) = match register.is_8bit() {
                            true => {
                                let val = self.registers.get_8bit(register);
                                let result = val.rotate_left(1);
                                self.registers.set(register, result as u16);
                                (val, result)
                            },
                            false => {
                                let addr = self.registers.get(register);
                                let val = bus.read(addr);
                                let result = val.rotate_left(1);
                                bus.write(addr, result);
                                (val, result)
                            }
                        };
                        self.registers.set_flag(FlagRegister::Zero, result == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                        self.registers.set_flag(FlagRegister::Carry, get_bit(val, BitIndex::I7));
                    },
                    CBOpcode::RRC(register) => {
                        let (val, result) = match register.is_8bit() {
                            true => {
                                let val = self.registers.get_8bit(register);
                                let result = val.rotate_right(1);
                                self.registers.set(register, result as u16);
                                (val, result)
                            },
                            false => {
                                let addr = self.registers.get(register);
                                let val = bus.read(addr);
                                let result = val.rotate_right(1);
                                bus.write(addr, result);
                                (val, result)
                            },
                        };
                        self.registers.set_flag(FlagRegister::Zero, result == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                        self.registers.set_flag(FlagRegister::Carry, get_bit(val, BitIndex::I0));
                    },
                    CBOpcode::RL(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let old_carry = self.registers.get_flag(FlagRegister::Carry);
                        let new_carry = get_bit(val, BitIndex::I7);
                        let val = val << 1 | (old_carry as u8);
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                        self.registers.set_flag(FlagRegister::Carry, new_carry);
                        self.registers.set_flag(FlagRegister::Zero, val == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::RR(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let old_carry = self.registers.get_flag(FlagRegister::Carry);
                        let new_carry = get_bit(val, BitIndex::I0);
                        let val = val >> 1 | ((old_carry as u8) << 7);
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                        self.registers.set_flag(FlagRegister::Carry, new_carry);
                        self.registers.set_flag(FlagRegister::Zero, val == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::SLA(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register) as i8,
                            false => bus.read(self.registers.get(register)) as i8,
                        };
                        let res = val << 1;
                        match register.is_8bit() {
                            true => self.registers.set(register, res as u16),
                            false => bus.write(self.registers.get(register), res as u8),
                        };
                        self.registers.set_flag(FlagRegister::Carry, get_bit(val as u8, BitIndex::I7));
                        self.registers.set_flag(FlagRegister::Zero, res == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::SRA(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register) as i8,
                            false => bus.read(self.registers.get(register)) as i8,
                        };
                        let res = val >> 1;
                        match register.is_8bit() {
                            true => self.registers.set(register, res as u16),
                            false => bus.write(self.registers.get(register), res as u8),
                        };
                        self.registers.set_flag(FlagRegister::Carry, get_bit(val as u8, BitIndex::I0));
                        self.registers.set_flag(FlagRegister::Zero, res == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::SRL(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let carry = get_bit(val, BitIndex::I0);
                        let val = val >> 1;
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                        self.registers.set_flag(FlagRegister::Carry, carry);
                        self.registers.set_flag(FlagRegister::Zero, val == 0);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::SWAP(register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let val = (val << 4) | (val >> 4);
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                        self.registers.set_flag(FlagRegister::Zero, val == 0);
                        self.registers.set_flag(FlagRegister::Carry, false);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, false);
                    },
                    CBOpcode::BIT(index, register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let res = get_bit(val, index);
                        self.registers.set_flag(FlagRegister::Zero, !res);
                        self.registers.set_flag(FlagRegister::Substract, false);
                        self.registers.set_flag(FlagRegister::HalfCarry, true);
                    },
                    CBOpcode::RES(index, register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let val = set_bit(val, false, index);
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                    },
                    CBOpcode::SET(index, register) => {
                        let val = match register.is_8bit() {
                            true => self.registers.get_8bit(register),
                            false => bus.read(self.registers.get(register)),
                        };
                        let val = set_bit(val, true, index);
                        match register.is_8bit() {
                            true => self.registers.set(register, val as u16),
                            false => bus.write(self.registers.get(register), val),
                        };
                    },
                };
            },
            Opcode::CPL => {
                self.registers.increment(Register::PC, 1);
                self.registers.set(Register::A, !self.registers.get(Register::A));
                self.registers.set_flag(FlagRegister::Substract, true);
                self.registers.set_flag(FlagRegister::HalfCarry, true);
            },
            // Invert the carry flag
            Opcode::CCF => {
                self.registers.increment(Register::PC, 1);
                self.registers.set_flag(FlagRegister::Carry, !self.registers.get_flag(FlagRegister::Carry));
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            // Set the carry flag
            Opcode::SCF => {
                self.registers.increment(Register::PC, 1);
                self.registers.set_flag(FlagRegister::Carry, true);
                self.registers.set_flag(FlagRegister::Substract, false);
                self.registers.set_flag(FlagRegister::HalfCarry, false);
            },
            // Enable interrupts
            Opcode::EI => {
                self.registers.increment(Register::PC, 1);
                self.ei_delay = true;
            },
            // Disable interrupts
            Opcode::DI => {
                self.registers.increment(Register::PC, 1);
                self.ime = false;
            },
            // Same as enabling interrupts and then executing RET
            Opcode::RETI => {
                self.exec(Opcode::RET(OpcodeParameter::NoParam), bus);
                self.ime = true;
            },
            // Don't execute instructions until an interrupt is requested
            Opcode::HALT => {
                self.registers.increment(Register::PC, 1);
                self.is_halted = true;
            },
            Opcode::STOP => {
                self.registers.increment(Register::PC, 2);
                if self.is_cgb && bus.prepare_double_speed_mode() {
                    bus.set_double_speed_mode(!self.double_speed_mode);
                    let speed_switch_register = bus.read(PREPARE_SPEED_SWITCH_ADDRESS);
                    // Disable prepare speed switch
                    bus.write(PREPARE_SPEED_SWITCH_ADDRESS, speed_switch_register & 0xFE);
                    println!("Switching speed mode to: {}", !self.double_speed_mode);
                    self.double_speed_mode = !self.double_speed_mode;
                }
            },
            Opcode::NOP => self.registers.increment(Register::PC, 1),
            Opcode::IllegalInstruction => {
                println!("Illegal instruction!");
                self.registers.increment(Register::PC, 1);
            },
            _ => unreachable!(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registers_setters_getters() {
        // Test 8 bit setters and getters
        let mut registers = Registers::new();
        registers.set(Register::A, 0b01010101);
        assert_eq!(registers.get(Register::A), 0b01010101);
        registers.set(Register::F, 0b01010101);
        assert_eq!(registers.get(Register::F), 0b01010101);
        registers.set(Register::B, 0b01010101);
        assert_eq!(registers.get(Register::B), 0b01010101);
        registers.set(Register::C, 0b01010101);
        assert_eq!(registers.get(Register::C), 0b01010101);
        registers.set(Register::D, 0b01010101);
        assert_eq!(registers.get(Register::D), 0b01010101);
        registers.set(Register::E, 0b01010101);
        assert_eq!(registers.get(Register::E), 0b01010101);
        registers.set(Register::H, 0b01010101);
        assert_eq!(registers.get(Register::H), 0b01010101);
        registers.set(Register::L, 0b01010101);
        assert_eq!(registers.get(Register::L), 0b01010101);

        // Test 16 bit setters and getters
        let mut registers = Registers::new();
        registers.set(Register::A, 0b01010101);
        registers.set(Register::F, 0b11111111);
        assert_eq!(registers.get(Register::AF), 0b0101010111111111);
        registers.set(Register::AF, 0b1111111101010101);
        assert_eq!(registers.get(Register::AF), 0b1111111101010101);

        registers.set(Register::B, 0b01010101);
        registers.set(Register::C, 0b11111111);
        assert_eq!(registers.get(Register::BC), 0b0101010111111111);
        registers.set(Register::BC, 0b1111111101010101);
        assert_eq!(registers.get(Register::BC), 0b1111111101010101);

        registers.set(Register::D, 0b01010101);
        registers.set(Register::E, 0b11111111);
        assert_eq!(registers.get(Register::DE), 0b0101010111111111);
        registers.set(Register::DE, 0b1111111101010101);
        assert_eq!(registers.get(Register::DE), 0b1111111101010101);

        registers.set(Register::H, 0b01010101);
        registers.set(Register::L, 0b11111111);
        assert_eq!(registers.get(Register::HL), 0b0101010111111111);
        registers.set(Register::HL, 0b1111111101010101);
        assert_eq!(registers.get(Register::HL), 0b1111111101010101);

        registers.set(Register::SP, 0b0101010111111111);
        assert_eq!(registers.get(Register::SP), 0b0101010111111111);

        registers.set(Register::PC, 0b0101010111111111);
        assert_eq!(registers.get(Register::PC), 0b0101010111111111);
    }

    #[test]
    fn test_ld_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0xFF);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::A)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0xFF);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::LD(OpcodeParameter::Register_U16(Register::SP, 0xF1F1)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0xF1F1);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        bus.write(addr, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_U16(Register::A, addr)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::LD(OpcodeParameter::Register_U8(Register::B, 0xF1)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set(Register::SP, 0x1234);
        cpu.exec(Opcode::LD(OpcodeParameter::U16_Register(0xF0F0, Register::SP)), &mut bus);
        assert_eq!(bus.read_16bit(0xF0F0), 0x1234);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x12);
        cpu.exec(Opcode::LD(OpcodeParameter::U16_Register(addr, Register::A)), &mut bus);
        assert_eq!(bus.read(addr), 0x12);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0xFF);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x00);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::A)), &mut bus);
        assert_eq!(bus.read(addr), 0xFF);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0xFF);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0xFF);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xFF00;
        cpu.registers.set(Register::A, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::FF00plusU8_Register(0x42, Register::A)), &mut bus);
        assert_eq!(bus.read(addr + 0x42), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xFF00;
        cpu.registers.set(Register::A, 0x00);
        bus.write(addr + 0x42, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_FF00plusU8(Register::A, 0x42)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let val = 100;
        cpu.registers.set(Register::SP, val);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_RegisterPlusI8(Register::HL, Register::SP, -5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::HL), val - 5);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::HL, 0x0000);
        cpu.registers.set(Register::SP, 0x000F);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_RegisterPlusI8(Register::HL, Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::HL), 0x0010);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_U8(Register::HL, 0xF1)), &mut bus);
        assert_eq!(bus.read(addr), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::C, 1);
        bus.write(0xFF01, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_FF00plusRegister(Register::A, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::C, 1);
        cpu.registers.set(Register::A, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::FF00plusRegister_Register(Register::C, Register::A)), &mut bus);
        assert_eq!(bus.read(0xFF01), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_ldi_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0xF1);
        cpu.exec(Opcode::LDI(OpcodeParameter::Register_RegisterIncrement(Register::A, Register::HL)), &mut bus);
        assert_eq!(bus.read(addr), 0xF1);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::HL), addr + 1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x1F);
        cpu.registers.set(Register::HL, addr);
        cpu.exec(Opcode::LDI(OpcodeParameter::RegisterIncrement_Register(Register::HL, Register::A)), &mut bus);
        assert_eq!(bus.read(addr), 0x1F);
        assert_eq!(cpu.registers.get(Register::HL), addr + 1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_ldd_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0xF1);
        cpu.exec(Opcode::LDD(OpcodeParameter::Register_RegisterDecrement(Register::A, Register::HL)), &mut bus);
        assert_eq!(bus.read(addr), 0xF1);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::HL), addr - 1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::A, 0x1F);
        cpu.registers.set(Register::HL, addr);
        cpu.exec(Opcode::LDD(OpcodeParameter::RegisterDecrement_Register(Register::HL, Register::A)), &mut bus);
        assert_eq!(bus.read(addr), 0x1F);
        assert_eq!(cpu.registers.get(Register::HL), addr - 1);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_jp_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::JP(OpcodeParameter::U16(0x1F1F)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1F1F);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        cpu.exec(Opcode::JP(OpcodeParameter::Register(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), addr);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, addr)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), addr);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.exec(Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, addr)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), addr);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.exec(Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, addr)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, addr)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);
    }

    #[test]
    fn test_jr_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set(Register::PC, 100);
        cpu.exec(Opcode::JR(OpcodeParameter::I8(-5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 95 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.exec(Opcode::JR(OpcodeParameter::I8(5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 105 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Zero, -5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 95 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.exec(Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Zero, -5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 95 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.exec(Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Zero, -5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 100 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Zero, -5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 100 + 2);

        cpu.registers.set(Register::PC, 100);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Zero, 5)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 105 + 2);
    }

    #[test]
    fn test_di_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::DI, &mut bus);
        assert_eq!(cpu.ime, false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_ei_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::EI, &mut bus);
        assert_eq!(cpu.ime, true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_rlca_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b00000010);
        cpu.exec(Opcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00000100);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b10000000);
        cpu.exec(Opcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00000001);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0x01);
        cpu.exec(Opcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0x02);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_rrca_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b01000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00100000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b00000001);
        cpu.exec(Opcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b10000000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_call_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::CALL(OpcodeParameter::U16(0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0xF0F0);

        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, 0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0xF0F0);

        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.exec(Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, 0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);

        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, 0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0xF0F0);

        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.exec(Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, 0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp);
        assert_eq!(cpu.registers.get(Register::PC), 0x103);
    }

    #[test]
    fn test_rst_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::RST(0xF0), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 1);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0x00F0);
    }

    #[test]
    fn test_push_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xD000;
        cpu.registers.set(Register::SP, addr);
        cpu.registers.set(Register::AF, 0x1234);
        cpu.exec(Opcode::PUSH(Register::AF), &mut bus);
        assert_eq!(bus.read_16bit(addr - 2), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), addr - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_pop_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xD000;
        cpu.registers.set(Register::SP, addr);
        bus.write_16bit(addr, 0x1234);
        cpu.exec(Opcode::POP(Register::PC), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), addr + 2);
    }

    #[test]
    fn test_ret_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::NoParam), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), sp + 2);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Zero)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), sp + 2);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Zero)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get(Register::SP), sp);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Zero)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), sp + 2);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Zero)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get(Register::SP), sp);
    }

    #[test]
    fn test_and_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0xF1);
        cpu.registers.set(Register::C, 0x1F);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0xF1 & 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::C, 0x00);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x1F);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x1F);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F & 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x00);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x1F);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_U8(Register::B, 0x1A)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F & 0x1A);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.exec(Opcode::AND(OpcodeParameter::Register_U8(Register::B, 0x00)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_or_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0xF1);
        cpu.registers.set(Register::C, 0x1F);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0xF1 | 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::C, 0x00);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x1F);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x1F);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F | 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x00);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x1F);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_U8(Register::B, 0x1F)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F | 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.exec(Opcode::OR(OpcodeParameter::Register_U8(Register::B, 0x00)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_xor_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0xF1);
        cpu.registers.set(Register::C, 0x1F);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0xF1 ^ 0x1F);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::C, 0x00);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x1F);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0xF1);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F ^ 0xF1);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0x00);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0x00);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x1F);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_U8(Register::B, 0xF1)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F ^ 0xF1);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x00);
        cpu.exec(Opcode::XOR(OpcodeParameter::Register_U8(Register::B, 0x00)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_cp_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0xF1);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_U8(Register::B, 0xF1)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b00110000);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_U8(Register::B, 0b00000100)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b01000000);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_U8(Register::B, 0b10000000)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0xF1);
        cpu.registers.set(Register::C, 0xF1);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b00110000);
        cpu.registers.set(Register::C, 0b00000100);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b01000000);
        cpu.registers.set(Register::C, 0b10000000);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0xF1);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0xF1);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0b00110000);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b00000100);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 0b01000000);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b10000000);
        cpu.exec(Opcode::CP(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_add_instructions() {
        // let mut bus = Bus::new();
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b00001000);
        cpu.registers.set(Register::C, 0b00001000);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0b00010000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b10000000);
        cpu.registers.set(Register::C, 0b10000000);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0x00);
        cpu.registers.set(Register::B, 0x80);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::B)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0x80);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 40);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 40);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 80);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 40);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_U8(Register::B, 40)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 80);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 40);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_U8(Register::HL, 40)), &mut bus);
        assert_eq!(bus.read(addr), 80);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 40);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::B, -40 as i8)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x0000);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x0001);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::SP, 0x0100);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::SP), 0x0101);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x00FF);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x0100);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x7FFF);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x8000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x000F);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x0010);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0xFFFF);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x0000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x0000);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, (0xFF as u8) as i8)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0xFFFF);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::SP, 0x0001);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_I8(Register::SP, (0xFF as u8) as i8)), &mut bus);
        assert_eq!(cpu.registers.get(Register::SP), 0x0000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::BC, 0b0000100000000000);
        cpu.registers.set(Register::HL, 0b0000100000000000);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::BC, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::BC), 0b0001000000000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::HL, 0x0000);
        cpu.registers.set(Register::SP, 0x8000);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::SP)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::HL), 0x8000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::HL, 0x0001);
        cpu.registers.set(Register::SP, 0x00FF);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::SP)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::HL), 0x0100);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.registers.set_flag(FlagRegister::Substract, true);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::HL, 0x2608);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::HL), 0x4C10);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::SP, 0x000F);
        cpu.registers.set(Register::HL, 0x0001);
        cpu.exec(Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::SP)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::HL), 0x0010);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_adc_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.registers.set(Register::A, 0xFE);
        cpu.exec(Opcode::ADC(OpcodeParameter::Register_U8(Register::A, 0x01)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0x00);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.registers.set(Register::A, 0x00);
        cpu.exec(Opcode::ADC(OpcodeParameter::Register_U8(Register::A, 0x0F)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0x10);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.registers.set(Register::A, 0x01);
        cpu.exec(Opcode::ADC(OpcodeParameter::Register_U8(Register::A, 0x0F)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0x11);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_sbc_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.registers.set(Register::A, 0x00);
        cpu.exec(Opcode::SBC(OpcodeParameter::Register_U8(Register::A, 0x00)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0xFF);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_sub_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b00001000);
        cpu.registers.set(Register::C, 0b00001000);
        cpu.exec(Opcode::SUB(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0b00010000);
        cpu.registers.set(Register::C, 0b00011000);
        cpu.exec(Opcode::SUB(OpcodeParameter::Register_Register(Register::B, Register::C)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 248);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xC000;
        cpu.registers.set(Register::B, 40);
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 40);
        cpu.exec(Opcode::SUB(OpcodeParameter::Register_Register(Register::B, Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get(Register::B), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 40);
        cpu.exec(Opcode::SUB(OpcodeParameter::Register_U8(Register::B, 40)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get(Register::B), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_inc_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0);
        cpu.exec(Opcode::INC(true, false, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00001111);
        cpu.exec(Opcode::INC(true, false, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b00001111);
        cpu.exec(Opcode::INC(true, true, Register::HL), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(bus.read(addr), 0b00010000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::BC, 0b0000111111111111);
        cpu.exec(Opcode::INC(true, false, Register::BC), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::BC), 0b0001000000000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_dec_instructions() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 1);
        cpu.exec(Opcode::DEC(true, false, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get(Register::A), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00010000);
        cpu.exec(Opcode::DEC(true, false, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00001111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b00010000);
        cpu.exec(Opcode::DEC(true, true, Register::HL), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(bus.read(addr), 0b00001111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::BC, 0b0001000000000000);
        cpu.exec(Opcode::DEC(true, false, Register::BC), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::BC), 0b0000111111111111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_rla_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::RLA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000011);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b10000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::RLA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_rra_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::RRA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b10000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b10000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::RRA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b01000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_prefix_cb_rlc_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RLC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000010);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b10000000);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RLC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00000001);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RLC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b00000001);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RLC(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(bus.read(addr), 0b00000010);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_rrc_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RRC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b10000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000010);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RRC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000001);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RRC(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b00000001);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RRC(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(bus.read(addr), 0b10000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_rl_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RL(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000011);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b10000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RL(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000010);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RL(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000100);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        bus.write(addr, 0b00000010);
        cpu.registers.set(Register::HL, addr);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RL(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(bus.read(addr), 0b00000100);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_rr_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RR(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b10000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b01000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RR(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00100000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RR(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        bus.write(addr, 0b01000000);
        cpu.registers.set(Register::HL, addr);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RR(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(bus.read(addr), 0b00100000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_sla_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x01);
        cpu.registers.set_flag(FlagRegister::Zero, true);
        cpu.registers.set_flag(FlagRegister::Substract, true);
        cpu.registers.set_flag(FlagRegister::HalfCarry, true);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SLA(Register::B)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::B), 0x02);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_sra_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::B, 0x01);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, false);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SRA(Register::B)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::B), 0x00);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_srl_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000010);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SRL(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000001);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000001);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SRL(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        bus.write(addr, 0b00000001);
        cpu.registers.set(Register::HL, addr);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SRL(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(bus.read(addr), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_swap_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SWAP(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b01011111);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00000000);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SWAP(Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0b00000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SWAP(Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(bus.read(addr), 0b01011111);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_bit_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I3, Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I4, Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::BIT(BitIndex::I0 ,Register::HL)), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_res_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RES(BitIndex::I2, Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_8bit(Register::A), 0b11110001);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::RES(BitIndex::I0 ,Register::HL)), &mut bus);
        assert_eq!(bus.read(addr), 0b11110100);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_prefix_cb_set_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SET(BitIndex::I1, Register::A)), &mut bus);
        assert_eq!(cpu.registers.get_8bit(Register::A), 0b11110111);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let addr = 0xC000;
        cpu.registers.set(Register::HL, addr);
        bus.write(addr, 0b11110101);
        cpu.exec(Opcode::PrefixCB(CBOpcode::SET(BitIndex::I3 ,Register::HL)), &mut bus);
        assert_eq!(bus.read(addr), 0b11111101);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);
    }

    #[test]
    fn test_daa_instruction() {
        let mut bus = Bus::new();
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0x0A);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::DAA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::A), 0x10);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0x9A);
        cpu.exec(Opcode::DAA, &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::A), 0x00);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0x00);
        cpu.registers.set_flag(FlagRegister::Zero, false);
        cpu.registers.set_flag(FlagRegister::Substract, false);
        cpu.registers.set_flag(FlagRegister::HalfCarry, true);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::DAA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0x06);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }

    #[test]
    fn test_cpl_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set(Register::A, 0b11110000);
        cpu.exec(Opcode::CPL, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00001111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
    }

    #[test]
    fn test_ccf_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Substract, true);
        cpu.registers.set_flag(FlagRegister::HalfCarry, true);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::CCF, &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Substract, true);
        cpu.registers.set_flag(FlagRegister::HalfCarry, true);
        cpu.registers.set_flag(FlagRegister::Carry, true);
        cpu.exec(Opcode::CCF, &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
    }

    #[test]
    fn test_scf_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Substract, true);
        cpu.registers.set_flag(FlagRegister::HalfCarry, true);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::SCF, &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
    }

    #[test]
    fn test_nop_instructions() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::NOP, &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }
}
