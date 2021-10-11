use crate::utils::{BitIndex, get_bit, set_bit};

pub enum Register {
    A(u8), // Accumulator
    F(u8), // Flags
    B(u8), 
    C(u8),
    D(u8),
    E(u8),
    H(u8),
    L(u8),
    // This registers are just the same as above but combined to get a 16 bits register
    BC(u16),
    DE(u16),
    HL(u16),

    SP(u16),
    PC(u16),
}

pub enum FlagRegister {
    Zero(bool), // Set when the result of a math operation is zero or if two values matches using the CP instruction
    Substract(bool), // Set if a substraction was performed in the last math instruction
    HalfCarry(bool), // Set if a carry ocurred from the lower nibble in the last math operation
    Carry(bool), // Set if a carry was ocurrend from the last math operation or if register A is the smaller value when executing the CP instruction
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
            a: 0,
            f: 0b11110000, // The first 4 lower bits are always set to 0
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    pub fn get(&self, register: Register) -> u16 {
        match register {
            Register::A(_) => self.a as u16,
            Register::B(_) => self.b as u16,
            Register::C(_) => self.c as u16,
            Register::D(_) => self.d as u16,
            Register::E(_) => self.e as u16,
            Register::F(_) => self.f as u16,
            Register::H(_) => self.h as u16,
            Register::L(_) => self.l as u16,
            Register::BC(_) => self.get_bc(),
            Register::DE(_) => self.get_de(),
            Register::HL(_) => self.get_hl(),
            Register::SP(_) => self.sp,
            Register::PC(_) => self.pc,
        }
    }

    pub fn set(&mut self, register: Register) {
        match register {
            Register::A(val)  => self.a = val,
            Register::B(val)  => self.b = val,
            Register::C(val)  => self.c = val,
            Register::D(val)  => self.d = val,
            Register::E(val)  => self.e = val,
            Register::F(val)  => self.f = val,
            Register::H(val)  => self.h = val,
            Register::L(val)  => self.l = val,
            Register::BC(val) => self.set_bc(val),
            Register::DE(val) => self.set_de(val),
            Register::HL(val) => self.set_hl(val),
            Register::SP(val) => self.sp = val,
            Register::PC(val) => self.pc = val,
        }
    }

    pub fn get_flag(&self, flag: FlagRegister) -> bool {
        match flag {
            FlagRegister::Zero(_)      => get_bit(self.f, BitIndex::I7),
            FlagRegister::Substract(_) => get_bit(self.f, BitIndex::I6),
            FlagRegister::HalfCarry(_) => get_bit(self.f, BitIndex::I5),
            FlagRegister::Carry(_)     => get_bit(self.f, BitIndex::I4),
        }
    }

    pub fn set_flag(&mut self, flag: FlagRegister) {
        match flag {
            FlagRegister::Zero(val)      => self.f = set_bit(self.f, val, BitIndex::I7),
            FlagRegister::Substract(val) => self.f = set_bit(self.f, val, BitIndex::I6),
            FlagRegister::HalfCarry(val) => self.f = set_bit(self.f, val, BitIndex::I5),
            FlagRegister::Carry(val)     => self.f = set_bit(self.f, val, BitIndex::I4),
        }
    }

    fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    fn set_bc(&mut self, val: u16) {
        let bytes = val.to_be_bytes();
        self.b = bytes[0];
        self.c = bytes[1];
    }

    fn set_de(&mut self, val: u16) {
        let bytes = val.to_be_bytes();
        self.d = bytes[0];
        self.e = bytes[1];
    }

    fn set_hl(&mut self, val: u16) {
        let bytes = val.to_be_bytes();
        self.h = bytes[0];
        self.l = bytes[1];
    }
}

pub struct CPU {
    registers: Registers,
}
