use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
    join_bytes,
    add_half_carry,
    sub_half_carry
};
use crate::bus::Bus;

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

pub enum InterruptFlag {
    VBlank,
    LCDSTAT,
    Timer,
    Serial,
    Joypad,
}

impl InterruptFlag {
    pub fn get_bit_index(interrupt: InterruptFlag) -> BitIndex {
        match interrupt {
            InterruptFlag::VBlank  => BitIndex::I0,
            InterruptFlag::LCDSTAT => BitIndex::I1,
            InterruptFlag::Timer   => BitIndex::I2,
            InterruptFlag::Serial  => BitIndex::I3,
            InterruptFlag::Joypad  => BitIndex::I4,
        }
    }
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

    pub fn increment(&mut self, register: Register, times: u8) {
        self.set(register, self.get(register) + (times as u16));
    }

    pub fn decrement(&mut self, register: Register, times: u8) {
        self.set(register, self.get(register) - (times as u16));
    }
}

#[derive(Debug)]
pub enum OpcodeParameter {
    Register(Register),
    Register_U8(Register, u8),
    Register_U16(Register, u16),
    Register_I8(Register, u8),
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

    Register_RegisterPlusI8(Register, Register, u8),

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
    INC(bool, Register),
    DEC(bool, Register),
    SWAP,
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
    RLC,
    RL,
    RRC,
    RR,
    SLA,
    SRA,
    SRL,
    BIT,
    SET,
    RES,
    JP(OpcodeParameter),
    JR(OpcodeParameter),
    CALL(OpcodeParameter),
    RST(u8),
    RET(OpcodeParameter),
    RETI,
    PrefixCB,
    IllegalInstruction,
}


pub struct CPU {
    registers: Registers,
    exec_calls_count: usize,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            exec_calls_count: 0,
        }
    }

    pub fn get_exec_calls_count(&self) -> usize {
        self.exec_calls_count
    }

    fn increment_exec_calls_count(&mut self) {
        self.exec_calls_count += 1;
    }

    pub fn run(&mut self, bus: &mut Bus) {
        let program_counter = self.registers.get(Register::PC);
        let parameter_bytes = CPU::read_parameter_bytes(program_counter, bus);
        let opcode = CPU::parse_opcode(parameter_bytes);
        // println!("Opcode: {:02X?} | PC: {:04X?} | Params: {:02X?}", opcode, self.registers.get(Register::PC), &parameter_bytes);
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
        self.exec(opcode, bus);
        self.increment_exec_calls_count();
    }

    pub fn exec(&mut self, opcode: Opcode, bus: &mut Bus) {
        match opcode {
            // Load
            Opcode::LD(params) => match params {
                OpcodeParameter::Register_Register(reg1, reg2) => {
                    if reg1.is_16bit() && reg2.is_8bit() {
                        let val = self.registers.get(reg2).to_be_bytes()[1];
                        let addr = self.registers.get(reg1);
                        bus.write(addr, val);
                    } else if reg1.is_8bit() && reg2.is_16bit() {
                        let val = bus.read(self.registers.get(reg2)) as u16;
                        self.registers.set(reg1, val);
                    } else {
                        self.registers.set(reg1, self.registers.get(reg2));
                    }
                    self.registers.increment(Register::PC, 1);
                },
                OpcodeParameter::Register_U16(register, val) => {
                    self.registers.set(register, val);
                    self.registers.increment(Register::PC, 3);
                },
                OpcodeParameter::Register_U8(register, val) => {
                    self.registers.set(register, val as u16);
                    self.registers.increment(Register::PC, 2);
                },
                OpcodeParameter::U16_Register(address, register) => {
                    let value = self.registers.get(register);
                    let bytes = value.to_be_bytes();
                    match register.is_8bit() {
                        true => bus.write(address, bytes[1]),
                        false => bus.write_16bit(address, value),
                    }
                    self.registers.increment(Register::PC, 3);
                },
                OpcodeParameter::Register_FF00plusU8(register, val) => {
                    self.registers.set(register, bus.read(0xFF00 + (val as u16)) as u16);
                    self.registers.increment(Register::PC, 2);
                },
                OpcodeParameter::FF00plusU8_Register(val, register) => {
                    match register.is_8bit() {
                        true => bus.write(0xFF00 + (val as u16), self.registers.get(register).to_be_bytes()[1]),
                        false => bus.write_16bit(0xFF00 + (val as u16), self.registers.get(register)),
                    }
                    self.registers.increment(Register::PC, 2);
                },
                _ => {},
            },
            // Increment or decrement program counter by signed N
            Opcode::JR(params) => {
                self.registers.increment(Register::PC, 2);
                let mut condition_met = false;
                let mut value = 0 as i16;
                match params {
                    OpcodeParameter::I8(val) => {
                        condition_met = true;
                        value = val as i16;
                    },
                    OpcodeParameter::FlagRegisterReset_I8(flag, val) => {
                        condition_met = !self.registers.get_flag(flag);
                        value = val as i16;
                    },
                    OpcodeParameter::FlagRegisterSet_I8(flag, val) => {
                        condition_met = self.registers.get_flag(flag);
                        value = val as i16;
                    },
                    _ => {},
                };
                if condition_met {
                    let pc = (self.registers.get(Register::PC) as i16) + (value as i16);
                    self.registers.set(Register::PC, pc as u16);
                }
            },
            // Load and increment
            Opcode::LDI(params) => match params {
                OpcodeParameter::Register_RegisterIncrement(reg1, reg2) => {
                    let val = bus.read(self.registers.get(reg2));
                    self.registers.set(reg1, val as u16);
                    self.registers.increment(reg2, 1);
                    self.registers.increment(Register::PC, 1);
                },
                _ => {},
            },
            Opcode::AND(params) => match params {
                OpcodeParameter::Register_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    match reg2.is_8bit() {
                        true => self.registers.set(reg1, self.registers.get(reg1) & self.registers.get(reg2)),
                        false => {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) & val);
                        },
                    };
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, true);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                OpcodeParameter::Register_U8(reg, val) => {
                    self.registers.increment(Register::PC, 2);
                    self.registers.set(reg, self.registers.get(reg) | (val as u16));
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, true);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                _ => {},
            },
            Opcode::OR(params) => match params {
                OpcodeParameter::Register_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    match reg2.is_8bit() {
                        true => self.registers.set(reg1, self.registers.get(reg1) | self.registers.get(reg2)),
                        false => {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) | val);
                        },
                    };
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, false);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                OpcodeParameter::Register_U8(reg, val) => {
                    self.registers.increment(Register::PC, 2);
                    self.registers.set(reg, self.registers.get(reg) | (val as u16));
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, false);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                _ => {},
            },
            Opcode::XOR(params) => match params {
                OpcodeParameter::Register_Register(reg1, reg2) => {
                    self.registers.increment(Register::PC, 1);
                    match reg2.is_8bit() {
                        true => self.registers.set(reg1, self.registers.get(reg1) ^ self.registers.get(reg2)),
                        false => {
                            let val = bus.read(self.registers.get(reg2)) as u16;
                            self.registers.set(reg1, self.registers.get(reg1) ^ val);
                        },
                    };
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg1) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, false);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                OpcodeParameter::Register_U8(reg, val) => {
                    self.registers.increment(Register::PC, 2);
                    self.registers.set(reg, self.registers.get(reg) ^ (val as u16));
                    self.registers.set_flag(FlagRegister::Zero, self.registers.get(reg) == 0);
                    self.registers.set_flag(FlagRegister::Substract, false);
                    self.registers.set_flag(FlagRegister::HalfCarry, false);
                    self.registers.set_flag(FlagRegister::Carry, false);
                },
                _ => {},
            }
            // Substract without storing the value
            Opcode::CP(params) => {
                let mut val1: i16 = 0;
                let mut val2: i16 = 0;
                match params {
                    OpcodeParameter::Register_U8(register, val) => {
                        self.registers.increment(Register::PC, 2);
                        val1 = self.registers.get(register) as i16;
                        val2 = val as i16;
                    },
                    OpcodeParameter::Register_Register(reg1, reg2) => {
                        self.registers.increment(Register::PC, 1);
                        val1 = self.registers.get(reg1) as i16;
                        match reg2.is_8bit() {
                            true => val2 = self.registers.get(reg2) as i16,
                            false => val2 = bus.read(self.registers.get(reg2)) as i16,
                        };
                    }
                    _ => {},
                };
                self.registers.set_flag(FlagRegister::Zero, (val1 - val2) == 0);
                self.registers.set_flag(FlagRegister::Substract, true);
                self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(val1.to_be_bytes()[1], val2.to_be_bytes()[1]));
                self.registers.set_flag(FlagRegister::Carry, val2 > val1);
            },
            // Increment by 1
            Opcode::INC(affect_flags, register) => {
                let prev_value = self.registers.get(register);
                self.registers.increment(register, 1);
                if affect_flags {
                    self.registers.set_flag(FlagRegister::Substract, false);
                    let mut byte_compare = 0;
                    match register.is_8bit() {
                        true => byte_compare = prev_value.to_be_bytes()[1],
                        false => byte_compare = prev_value.to_be_bytes()[0],
                    }
                    self.registers.set_flag(FlagRegister::HalfCarry, add_half_carry(byte_compare, 1));
                    let result = self.registers.get(register);
                    self.registers.set_flag(FlagRegister::Zero, result == 0);
                }
                self.registers.increment(Register::PC, 1);
            },
            // Decrement by 1
            Opcode::DEC(affect_flags, register) => {
                let prev_value = self.registers.get(register);
                self.registers.decrement(register, 1);
                if affect_flags {
                    self.registers.set_flag(FlagRegister::Substract, true);
                    let mut byte_compare = 0;
                    match register.is_8bit() {
                        true => byte_compare = prev_value.to_be_bytes()[1],
                        false => byte_compare = prev_value.to_be_bytes()[0],
                    }
                    self.registers.set_flag(FlagRegister::HalfCarry, sub_half_carry(byte_compare, 1));
                    let result = self.registers.get(register);
                    self.registers.set_flag(FlagRegister::Zero, result == 0);
                }
                self.registers.increment(Register::PC, 1);
            },
            // Jump to address
            Opcode::JP(params) => match params {
                OpcodeParameter::U16(address) => self.registers.set(Register::PC, address),
                _ => {},
            },
            // CALL
            Opcode::CALL(params) => match params {
                OpcodeParameter::U16(address) => {
                    let pc = self.registers.get(Register::PC);
                    self.registers.decrement(Register::SP, 2);
                    let sp = self.registers.get(Register::SP);
                    bus.write_16bit(sp, pc + 3);
                    self.registers.set(Register::PC, address);
                },
                _ => {},
            },
            // RST, same as Call
            Opcode::RST(address) => self.exec(Opcode::CALL(OpcodeParameter::U16(address as u16)), bus),
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
                self.registers.set(register, val);
                self.registers.increment(Register::SP, 2);
            },
            // RET, same as POP PC when no parameter is specified
            Opcode::RET(params) => match params {
                OpcodeParameter::NoParam => self.exec(Opcode::POP(Register::PC), bus),
                _ => {},
            }
            // Rotate A Left
            Opcode::RLCA => {
                let val = self.registers.get(Register::A).to_be_bytes()[1];
                let result = val.rotate_left(7);
                if get_bit(result, BitIndex::I7) {
                    self.registers.set_flag(FlagRegister::Carry, true);
                }
                self.registers.increment(Register::PC, 1);
            },
            // Rotate A Right 
            Opcode::RRCA => {
                let val = self.registers.get(Register::A).to_be_bytes()[1];
                let result = val.rotate_right(7);
                if get_bit(result, BitIndex::I0) {
                    self.registers.set_flag(FlagRegister::Carry, true);
                }
                self.registers.increment(Register::PC, 1);
            },
            // Disable interrupts
            Opcode::DI => {
                bus.write(0xFFFF, 0x00); // Disable all interrupts
                self.registers.increment(Register::PC, 1);
            },
            Opcode::NOP => self.registers.increment(Register::PC, 1),
            _ => println!("Illegal instruction"),
        };
    }

    fn read_parameter_bytes(address: u16, bus: &Bus) -> (u8, u8, u8, u8) {
        (
            bus.read(address),
            bus.read(address + 1),
            bus.read(address + 2),
            bus.read(address + 3),
        )
    }

    pub fn parse_opcode(params: (u8, u8, u8, u8)) -> Opcode {
        let opcode = params.0;
        let two_byte_param = join_bytes(params.2, params.1);
        match opcode {
            0x06 => Opcode::LD(OpcodeParameter::Register_U8(Register::B, params.1)),
            0x0E => Opcode::LD(OpcodeParameter::Register_U8(Register::C, params.1)),
            0x16 => Opcode::LD(OpcodeParameter::Register_U8(Register::D, params.1)),
            0x1E => Opcode::LD(OpcodeParameter::Register_U8(Register::E, params.1)),
            0x26 => Opcode::LD(OpcodeParameter::Register_U8(Register::H, params.1)),
            0x2E => Opcode::LD(OpcodeParameter::Register_U8(Register::L, params.1)),
            0x7F => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0x78 => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0x79 => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0x7A => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0x7B => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0x7C => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0x7D => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0x7E => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0x40 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::B)),
            0x41 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::C)),
            0x42 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::D)),
            0x43 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::E)),
            0x44 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::H)),
            0x45 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::L)),
            0x46 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::HL)),
            0x48 => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::B)),
            0x49 => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::C)),
            0x4A => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::D)),
            0x4B => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::E)),
            0x4C => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::H)),
            0x4D => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::L)),
            0x4E => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::HL)),
            0x50 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::B)),
            0x51 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::C)),
            0x52 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::D)),
            0x53 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::E)),
            0x54 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::H)),
            0x55 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::L)),
            0x56 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::HL)),
            0x58 => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::B)),
            0x59 => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::C)),
            0x5A => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::D)),
            0x5B => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::E)),
            0x5C => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::H)),
            0x5D => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::L)),
            0x5E => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::HL)),
            0x60 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::B)),
            0x61 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::C)),
            0x62 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::D)),
            0x63 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::E)),
            0x64 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::H)),
            0x65 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::L)),
            0x66 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::HL)),
            0x68 => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::B)),
            0x69 => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::C)),
            0x6A => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::D)),
            0x6B => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::E)),
            0x6C => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::H)),
            0x6D => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::L)),
            0x6E => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::HL)),
            0x70 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::B)),
            0x71 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::C)),
            0x72 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::D)),
            0x73 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::E)),
            0x74 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::H)),
            0x75 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::L)),
            0x47 => Opcode::LD(OpcodeParameter::Register_Register(Register::B, Register::A)),
            0x4F => Opcode::LD(OpcodeParameter::Register_Register(Register::C, Register::A)),
            0x57 => Opcode::LD(OpcodeParameter::Register_Register(Register::D, Register::A)),
            0x5F => Opcode::LD(OpcodeParameter::Register_Register(Register::E, Register::A)),
            0x67 => Opcode::LD(OpcodeParameter::Register_Register(Register::H, Register::A)),
            0x6F => Opcode::LD(OpcodeParameter::Register_Register(Register::L, Register::A)),
            0x02 => Opcode::LD(OpcodeParameter::Register_Register(Register::BC, Register::A)),
            0x12 => Opcode::LD(OpcodeParameter::Register_Register(Register::DE, Register::A)),
            0x77 => Opcode::LD(OpcodeParameter::Register_Register(Register::HL, Register::A)),
            0x36 => Opcode::LD(OpcodeParameter::Register_U8(Register::HL, params.1)),
            0x0A => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::BC)),
            0x1A => Opcode::LD(OpcodeParameter::Register_Register(Register::A, Register::DE)),
            0xFA => Opcode::LD(OpcodeParameter::Register_U16(Register::A, two_byte_param)), // Receives 16 bit value, but lower bit is ignored
            0x3E => Opcode::LD(OpcodeParameter::Register_U8(Register::A, params.1)),
            0xEA => Opcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::A)),
            0xF2 => Opcode::LD(OpcodeParameter::Register_FF00plusRegister(Register::A, Register::C)),
            0xE2 => Opcode::LD(OpcodeParameter::FF00plusRegister_Register(Register::A, Register::C)),
            0x3A => Opcode::LDD(OpcodeParameter::Register_RegisterDecrement(Register::A, Register::HL)),
            0x32 => Opcode::LDD(OpcodeParameter::RegisterDecrement_Register(Register::HL, Register::A)),
            0x2A => Opcode::LDI(OpcodeParameter::Register_RegisterIncrement(Register::A, Register::HL)),
            0x22 => Opcode::LDI(OpcodeParameter::RegisterIncrement_Register(Register::HL, Register::A)),
            0xE0 => Opcode::LD(OpcodeParameter::FF00plusU8_Register(params.1, Register::A)),
            0xF0 => Opcode::LD(OpcodeParameter::Register_FF00plusU8(Register::A, params.1)),
            0x01 => Opcode::LD(OpcodeParameter::Register_U16(Register::BC, two_byte_param)),
            0x11 => Opcode::LD(OpcodeParameter::Register_U16(Register::DE, two_byte_param)),
            0x21 => Opcode::LD(OpcodeParameter::Register_U16(Register::HL, two_byte_param)),
            0x31 => Opcode::LD(OpcodeParameter::Register_U16(Register::SP, two_byte_param)),
            0xF9 => Opcode::LD(OpcodeParameter::Register_Register(Register::SP, Register::HL)),
            0xF8 => Opcode::LD(OpcodeParameter::Register_RegisterPlusI8(Register::HL, Register::SP, params.1)),
            0x08 => Opcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::SP)),
            0xC5 => Opcode::PUSH(Register::BC),
            0xD5 => Opcode::PUSH(Register::DE),
            0xE5 => Opcode::PUSH(Register::HL),
            0xF5 => Opcode::PUSH(Register::AF),
            0xC1 => Opcode::POP(Register::BC),
            0xD1 => Opcode::POP(Register::DE),
            0xE1 => Opcode::POP(Register::HL),
            0xF1 => Opcode::POP(Register::AF),
            0x87 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0x80 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0x81 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0x82 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0x83 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0x84 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0x85 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0x86 => Opcode::ADD(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xC6 => Opcode::ADD(OpcodeParameter::Register_U8(Register::A, params.1)),
            0x8F => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0x88 => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0x89 => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0x8A => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0x8B => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0x8C => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0x8D => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0x8E => Opcode::ADC(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xCE => Opcode::ADC(OpcodeParameter::Register_U8(Register::A, params.1)),
            0x97 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0x90 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0x91 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0x92 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0x93 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0x94 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0x95 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0x96 => Opcode::SUB(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xD6 => Opcode::SUB(OpcodeParameter::Register_U8(Register::A, params.1)),
            0x9F => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0x98 => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0x99 => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0x9A => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0x9B => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0x9C => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0x9D => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0x9E => Opcode::SBC(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xDE => Opcode::SBC(OpcodeParameter::Register_U8(Register::A, params.1)),
            0xA7 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0xA0 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0xA1 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0xA2 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0xA3 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0xA4 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0xA5 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0xA6 => Opcode::AND(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xE6 => Opcode::AND(OpcodeParameter::Register_U8(Register::A, params.1)),
            0xB7 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0xB0 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0xB1 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0xB2 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0xB3 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0xB4 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0xB5 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0xB6 => Opcode::OR(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xF6 => Opcode::OR(OpcodeParameter::Register_U8(Register::A, params.1)),
            0xAF => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0xA8 => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0xA9 => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0xAA => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0xAB => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0xAC => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0xAD => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0xAE => Opcode::XOR(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xEE => Opcode::XOR(OpcodeParameter::Register_U8(Register::A, params.1)),
            0xBF => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::A)),
            0xB8 => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::B)),
            0xB9 => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::C)),
            0xBA => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::D)),
            0xBB => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::E)),
            0xBC => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::H)),
            0xBD => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::L)),
            0xBE => Opcode::CP(OpcodeParameter::Register_Register(Register::A, Register::HL)),
            0xFE => Opcode::CP(OpcodeParameter::Register_U8(Register::A, params.1)),
            0x3C => Opcode::INC(true, Register::A),
            0x04 => Opcode::INC(true, Register::B),
            0x0C => Opcode::INC(true, Register::C),
            0x14 => Opcode::INC(true, Register::D),
            0x1C => Opcode::INC(true, Register::E),
            0x24 => Opcode::INC(true, Register::H),
            0x2C => Opcode::INC(true, Register::L),
            0x34 => Opcode::INC(true, Register::HL),
            0x03 => Opcode::INC(false, Register::BC),
            0x13 => Opcode::INC(false, Register::DE),
            0x23 => Opcode::INC(false, Register::HL),
            0x33 => Opcode::INC(false, Register::SP),
            0x3D => Opcode::DEC(true, Register::A),
            0x05 => Opcode::DEC(true, Register::B),
            0x0D => Opcode::DEC(true, Register::C),
            0x15 => Opcode::DEC(true, Register::D),
            0x1D => Opcode::DEC(true, Register::E),
            0x25 => Opcode::DEC(true, Register::H),
            0x2D => Opcode::DEC(true, Register::L),
            0x35 => Opcode::DEC(true, Register::HL),
            0x0B => Opcode::DEC(false, Register::BC),
            0x1B => Opcode::DEC(false, Register::DE),
            0x2B => Opcode::DEC(false, Register::HL),
            0x3B => Opcode::DEC(false, Register::SP),
            0x09 => Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::BC)),
            0x19 => Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::DE)),
            0x29 => Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::HL)),
            0x39 => Opcode::ADD(OpcodeParameter::Register_Register(Register::HL, Register::SP)),
            0xE8 => Opcode::ADD(OpcodeParameter::Register_I8(Register::HL, params.1)),
            0x27 => Opcode::DAA,
            0x2F => Opcode::CPL,
            0x3F => Opcode::CCF,
            0x37 => Opcode::SCF,
            0x17 => Opcode::RLA,
            0x07 => Opcode::RLCA,
            0x0F => Opcode::RRCA,
            0x1F => Opcode::RRA,
            0xCB => Opcode::PrefixCB,
            //0xCB => Opcode::SWAP,
            //0xCB => Opcode::RLC,
            //0xCB => Opcode::RL,
            //0xCB => Opcode::RRC,
            //0xCB => Opcode::RR,
            //0xCB => Opcode::SLA,
            //0xCB => Opcode::SRA,
            //0xCB => Opcode::SRL,
            //0xCB => Opcode::BIT,
            //0xCB => Opcode::SET,
            //0xCB => Opcode::RES,
            0xC3 => Opcode::JP(OpcodeParameter::U16(two_byte_param)),
            0xC2 => Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, two_byte_param)),
            0xCA => Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, two_byte_param)),
            0xD2 => Opcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry, two_byte_param)),
            0xDA => Opcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry, two_byte_param)),
            0xE9 => Opcode::JP(OpcodeParameter::Register(Register::HL)),
            0x18 => Opcode::JR(OpcodeParameter::I8(params.1 as i8)),
            0x20 => Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Zero, params.1 as i8)),
            0x28 => Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Zero, params.1 as i8)),
            0x30 => Opcode::JR(OpcodeParameter::FlagRegisterReset_I8(FlagRegister::Carry, params.1 as i8)),
            0x38 => Opcode::JR(OpcodeParameter::FlagRegisterSet_I8(FlagRegister::Carry, params.1 as i8)),
            0xCD => Opcode::CALL(OpcodeParameter::U16(two_byte_param)),
            0xC4 => Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero, two_byte_param)),
            0xCC => Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero, two_byte_param)),
            0xD4 => Opcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry, two_byte_param)),
            0xDC => Opcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry, two_byte_param)),
            0xC7 => Opcode::RST(0x00),
            0xCF => Opcode::RST(0x08),
            0xD7 => Opcode::RST(0x10),
            0xDF => Opcode::RST(0x18),
            0xE7 => Opcode::RST(0x20),
            0xEF => Opcode::RST(0x28),
            0xF7 => Opcode::RST(0x30),
            0xFF => Opcode::RST(0x38),
            0xC9 => Opcode::RET(OpcodeParameter::NoParam),
            0xC0 => Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Zero)),
            0xC8 => Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Zero)),
            0xD0 => Opcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Carry)),
            0xD8 => Opcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Carry)),
            0xD9 => Opcode::RETI,
            0xF3 => Opcode::DI,
            0xFB => Opcode::EI,
            0x76 => Opcode::HALT,
            0x10 => Opcode::STOP,
            0x00 => Opcode::NOP,
            _ => Opcode::IllegalInstruction,
        }
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
    fn test_cpu_instructions() {
        // LD
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
        cpu.exec(Opcode::LD(OpcodeParameter::FF00plusU8_Register(4, Register::A)), &mut bus);
        assert_eq!(bus.read(addr + 4), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xFF00;
        cpu.registers.set(Register::A, 0x00);
        bus.write(addr + 4, 0xF1);
        cpu.exec(Opcode::LD(OpcodeParameter::Register_FF00plusU8(Register::A, 4)), &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0xF1);
        assert_eq!(cpu.registers.get(Register::PC), 0x102);

        // LDI
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

        // JP
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::JP(OpcodeParameter::U16(0x1F1F)), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1F1F);

        // JR
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

        // DI
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::DI, &mut bus);
        assert_eq!(bus.read(0xFFFF), 0x00);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // RLCA
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b00000010);
        cpu.exec(Opcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00000010);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b00000001);
        cpu.exec(Opcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b00000001);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // RRCA
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b01000000);
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.exec(Opcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b01000000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set_flag(FlagRegister::Carry, false);
        cpu.registers.set(Register::A, 0b10000000);
        cpu.exec(Opcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(Register::A), 0b10000000);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Carry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // CALL
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::CALL(OpcodeParameter::U16(0xF0F0)), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0xF0F0);

        // RST
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(Register::SP, sp);
        cpu.registers.set(Register::PC, 0x1234);
        cpu.exec(Opcode::RST(0xF0), &mut bus);
        assert_eq!(bus.read_16bit(sp - 2), 0x1234 + 3);
        assert_eq!(cpu.registers.get(Register::SP), sp - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0x00F0);

        // PUSH
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xD000;
        cpu.registers.set(Register::SP, addr);
        cpu.registers.set(Register::AF, 0x1234);
        cpu.exec(Opcode::PUSH(Register::AF), &mut bus);
        assert_eq!(bus.read_16bit(addr - 2), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), addr - 2);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // POP
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let addr = 0xD000;
        cpu.registers.set(Register::SP, addr);
        bus.write_16bit(addr, 0x1234);
        cpu.exec(Opcode::POP(Register::PC), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), addr + 2);

        // RET
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        let sp = 0xD000;
        cpu.registers.set(Register::SP, sp);
        bus.write_16bit(sp, 0x1234);
        cpu.exec(Opcode::RET(OpcodeParameter::NoParam), &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x1234);
        assert_eq!(cpu.registers.get(Register::SP), sp + 2);

        // AND
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
        cpu.exec(Opcode::AND(OpcodeParameter::Register_U8(Register::B, 0x1F)), &mut bus);
        assert_eq!(cpu.registers.get(Register::B), 0x1F & 0x1F);
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

        // OR
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

        // XOR
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

        // CP
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

        // INC
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0);
        cpu.exec(Opcode::INC(true, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00001111);
        cpu.exec(Opcode::INC(true, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::HL, 0b0000111111111111);
        cpu.exec(Opcode::INC(true, Register::HL), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::HL), 0b0001000000000000);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // DEC
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 1);
        cpu.exec(Opcode::DEC(true, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), false);
        assert_eq!(cpu.registers.get(Register::A), 0);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::A, 0b00010000);
        cpu.exec(Opcode::DEC(true, Register::A), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::A), 0b00001111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(Register::HL, 0b0001000000000000);
        cpu.exec(Opcode::DEC(true, Register::HL), &mut bus);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Zero), false);
        assert_eq!(cpu.registers.get_flag(FlagRegister::Substract), true);
        assert_eq!(cpu.registers.get_flag(FlagRegister::HalfCarry), true);
        assert_eq!(cpu.registers.get(Register::HL), 0b0000111111111111);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);

        // NOP
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(Opcode::NOP, &mut bus);
        assert_eq!(cpu.registers.get(Register::PC), 0x101);
    }
}
