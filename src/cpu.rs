use crate::utils::{BitIndex, get_bit, set_bit, join_bytes};
use crate::bus::Bus;

#[derive(Debug)]
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
    AF(u16),
    BC(u16),
    DE(u16),
    HL(u16),

    SP(u16), // Stack pointer
    PC(u16), // Program counter
}

#[derive(Debug)]
pub enum FlagRegister {
    Zero(bool), // Set when the result of a math operation is zero or if two values matches using the CP instruction
    Substract(bool), // Set if a substraction was performed in the last math instruction
    HalfCarry(bool), // Set if a carry ocurred from the lower nibble in the last math operation
    Carry(bool), // Set if a carry was ocurrend from the last math operation or if register A is the smaller value when executing the CP instruction
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
            a: 0,
            f: 0b00000000, // The first 4 lower bits are always set to 0
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0x100, // On power up, the Gamebou executes the instruction at hex 100
        }
    }

    pub fn get(&self, register: &Register) -> u16 {
        match register {
            Register::A(_) => self.a as u16,
            Register::B(_) => self.b as u16,
            Register::C(_) => self.c as u16,
            Register::D(_) => self.d as u16,
            Register::E(_) => self.e as u16,
            Register::F(_) => self.f as u16,
            Register::H(_) => self.h as u16,
            Register::L(_) => self.l as u16,
            Register::AF(_) => self.get_af(),
            Register::BC(_) => self.get_bc(),
            Register::DE(_) => self.get_de(),
            Register::HL(_) => self.get_hl(),
            Register::SP(_) => self.sp,
            Register::PC(_) => self.pc,
        }
    }

    pub fn set(&mut self, register: &Register) {
        match register {
            Register::A(val)  => self.a = *val,
            Register::B(val)  => self.b = *val,
            Register::C(val)  => self.c = *val,
            Register::D(val)  => self.d = *val,
            Register::E(val)  => self.e = *val,
            Register::F(val)  => self.f = *val,
            Register::H(val)  => self.h = *val,
            Register::L(val)  => self.l = *val,
            Register::AF(val) => self.set_af(*val),
            Register::BC(val) => self.set_bc(*val),
            Register::DE(val) => self.set_de(*val),
            Register::HL(val) => self.set_hl(*val),
            Register::SP(val) => self.sp = *val,
            Register::PC(val) => self.pc = *val,
        }
    }

    pub fn get_flag(&self, flag: &FlagRegister) -> bool {
        match flag {
            FlagRegister::Zero(_)      => get_bit(self.f, BitIndex::I7),
            FlagRegister::Substract(_) => get_bit(self.f, BitIndex::I6),
            FlagRegister::HalfCarry(_) => get_bit(self.f, BitIndex::I5),
            FlagRegister::Carry(_)     => get_bit(self.f, BitIndex::I4),
        }
    }

    pub fn set_flag(&mut self, flag: &FlagRegister) {
        match flag {
            FlagRegister::Zero(val)      => self.f = set_bit(self.f, *val, BitIndex::I7),
            FlagRegister::Substract(val) => self.f = set_bit(self.f, *val, BitIndex::I6),
            FlagRegister::HalfCarry(val) => self.f = set_bit(self.f, *val, BitIndex::I5),
            FlagRegister::Carry(val)     => self.f = set_bit(self.f, *val, BitIndex::I4),
        }
    }

    pub fn increment_pc(&mut self, times: u8) {
        self.pc += times as u16;
    }

    pub fn decrement_pc(&mut self, times: u8) {
        self.pc -= times as u16;
    }

    pub fn increment_sp(&mut self, times: u8) {
        self.sp += times as u16;
    }

    pub fn decrement_sp(&mut self, times: u8) {
        self.sp -= times as u16;
    }

    fn get_af(&self) -> u16 {
        join_bytes(self.a, self.f)
    }

    fn get_bc(&self) -> u16 {
        join_bytes(self.b, self.c)
    }

    fn get_de(&self) -> u16 {
        join_bytes(self.d, self.e)
    }

    fn get_hl(&self) -> u16 {
        join_bytes(self.h, self.l)
    }

    fn set_af(&mut self, val: u16) {
        let bytes = val.to_be_bytes();
        self.a = bytes[0];
        self.f = bytes[1];
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

#[derive(Debug)]
pub enum OpcodeParameter {
    Register(Register),
    Register_U8(Register),
    Register_U16(Register),
    Register_I8(Register),
    Register_I16(Register),
    U8_Register(Register),
    U16_Register(u16, Register),
    I8_Register(Register),
    I16_Register(Register),
    Register_16BitAddress(Register),
    Register_Register(Register, Register),

    Register_RegisterDecrement(Register, Register),
    RegisterDecrement_Register(Register, Register),

    Register_RegisterIncrement(Register, Register),
    RegisterIncrement_Register(Register, Register),

    Register_FF00plusRegister(Register, Register),
    FF00plusRegister_Register(Register, Register),
    Register_FF00plusU8(Register),
    FF00plusU8_Register(Register),

    Register_RegisterPlusI8(Register, Register),

    U8(u16),
    I8(u16),
    U16(u16),
    I16(u16),
    FlagRegisterReset(FlagRegister),
    FlagRegisterSet(FlagRegister),
    FlagRegisterReset_U16(FlagRegister, u16),
    FlagRegisterSet_U16(FlagRegister, u16),
    FlagRegisterReset_I16(FlagRegister, u16),
    FlagRegisterSet_I16(FlagRegister, u16),

    NoParam,
}

#[derive(Debug)]
pub enum CpuOpcode {
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
    INC(Register),
    DEC(Register),
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
}

impl CPU {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
        }
    }

    // Get the program counter
    pub fn get_register(&self, register: Register) -> u16 {
        self.registers.get(&register)
    }

    pub fn run(&mut self, bus: &mut Bus) {
        let program_counter = self.registers.get(&Register::PC(0));
        let parameter_bytes = CPU::read_parameter_bytes(program_counter, bus);
        println!("{:02X?}", parameter_bytes);
        let opcode = CPU::parse_opcode(&parameter_bytes);
        self.exec(opcode, bus);
    }

    pub fn exec(&mut self, opcode: CpuOpcode, bus: &mut Bus) {
        println!("Executing {:?}", opcode);
        println!("Current PC: {:02X?}", self.registers.get(&Register::PC(0)));
        match opcode {
            // Load
            CpuOpcode::LD(params) => match params {
                OpcodeParameter::Register_U16(register) => {
                    self.registers.set(&register);
                    self.registers.increment_pc(3);
                },
                OpcodeParameter::U16_Register(address, register) => {
                    type Rg = Register;
                    let value = self.registers.get(&register);
                    let bytes = value.to_be_bytes();
                    match register {
                        Rg::A(_) | Rg::F(_) | Rg::B(_) | Rg::C(_) | Rg::D(_) | Rg::E(_) | Rg::H(_) | Rg::L(_) => {
                            bus.write(address, bytes[1]);
                        },
                        Rg::AF(_) | Rg::BC(_) | Rg::DE(_) | Rg::HL(_) | Rg::SP(_) | Rg::PC(_) => {
                            bus.write(address, bytes[1]);
                            bus.write(address + 1, bytes[0]);
                        }
                    }
                    self.registers.increment_pc(3);
                },
                _ => {},
            }
            // Jump to address
            CpuOpcode::JP(params) => match params {
                OpcodeParameter::U16(address) => self.registers.set(&Register::PC(address)),
                _ => {},
            },
            // CALL
            CpuOpcode::CALL(params) => match params {
                OpcodeParameter::U16(address) => {
                    let pc_bytes = self.registers.get(&Register::PC(0)).to_be_bytes();
                    self.registers.decrement_sp(2);
                    let sp = self.registers.get(&Register::SP(0));
                    bus.write(sp, pc_bytes[1]);
                    bus.write(sp + 1, pc_bytes[0]);
                    self.registers.set(&Register::PC(address));
                },
                _ => {},
            },
            // RST, same as Call
            CpuOpcode::RST(address) => self.exec(CpuOpcode::CALL(OpcodeParameter::U16(address as u16)), bus),
            // Rotate A Left
            CpuOpcode::RLCA => {
                let val = self.registers.get(&Register::A(0)).to_be_bytes()[1];
                let result = val.rotate_left(7);
                if get_bit(result, BitIndex::I7) {
                    self.registers.set_flag(&FlagRegister::Carry(true));
                }
                self.registers.increment_pc(1);
            },
            // Rotate A Right 
            CpuOpcode::RRCA => {
                let val = self.registers.get(&Register::A(0)).to_be_bytes()[1];
                let result = val.rotate_right(7);
                if get_bit(result, BitIndex::I0) {
                    self.registers.set_flag(&FlagRegister::Carry(true));
                }
                self.registers.increment_pc(1);
            },
            // Disable interrupts
            CpuOpcode::DI => {
                bus.write(0xFFFF, 0x00); // Disable all interrupts
                self.registers.increment_pc(1);
            },
            CpuOpcode::NOP => self.registers.increment_pc(1),
            _ => println!("Illegal instruction"),
        };
    }

    fn read_parameter_bytes(address: u16, bus: &Bus) -> [u8; 3] {
        [
            bus.read(address),
            bus.read(address + 1),
            bus.read(address + 2),
        ]
    }

    pub fn parse_opcode(params: &[u8; 3]) -> CpuOpcode {
        let opcode = params[0];
        let two_byte_param = join_bytes(params[2], params[1]);
        match opcode {
            0x06 => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::B(0))),
            0x0E => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::C(0))),
            0x16 => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::D(0))),
            0x1E => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::E(0))),
            0x26 => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::H(0))),
            0x2E => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::L(0))),
            0x7F => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0x78 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0x79 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0x7A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0x7B => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0x7C => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0x7D => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0x7E => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0x40 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::B(0))),
            0x41 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::C(0))),
            0x42 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::D(0))),
            0x43 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::E(0))),
            0x44 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::H(0))),
            0x45 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::L(0))),
            0x46 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::HL(0))),
            0x48 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::B(0))),
            0x49 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::C(0))),
            0x4A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::D(0))),
            0x4B => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::E(0))),
            0x4C => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::H(0))),
            0x4D => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::L(0))),
            0x4E => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::HL(0))),
            0x50 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::B(0))),
            0x51 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::C(0))),
            0x52 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::D(0))),
            0x53 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::E(0))),
            0x54 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::H(0))),
            0x55 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::L(0))),
            0x56 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::HL(0))),
            0x58 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::B(0))),
            0x59 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::C(0))),
            0x5A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::D(0))),
            0x5B => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::E(0))),
            0x5C => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::H(0))),
            0x5D => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::L(0))),
            0x5E => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::HL(0))),
            0x60 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::B(0))),
            0x61 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::C(0))),
            0x62 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::D(0))),
            0x63 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::E(0))),
            0x64 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::H(0))),
            0x65 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::L(0))),
            0x66 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::HL(0))),
            0x68 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::B(0))),
            0x69 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::C(0))),
            0x6A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::D(0))),
            0x6B => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::E(0))),
            0x6C => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::H(0))),
            0x6D => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::L(0))),
            0x6E => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::HL(0))),
            0x70 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::B(0))),
            0x71 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::C(0))),
            0x72 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::D(0))),
            0x73 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::E(0))),
            0x74 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::H(0))),
            0x75 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::L(0))),
            0x36 => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::HL(0))),
            0x0A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::BC(0))),
            0x1A => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::A(0), Register::DE(0))),
            0xFA => CpuOpcode::LD(OpcodeParameter::Register_U8(Register::A(0))), // Receives 16 bit value, but lower bit is ignored
            0x3E => CpuOpcode::LD(OpcodeParameter::Register_U16(Register::A(0))),
            0x47 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::B(0), Register::A(0))),
            0x4F => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::C(0), Register::A(0))),
            0x57 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::D(0), Register::A(0))),
            0x5F => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::E(0), Register::A(0))),
            0x67 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::H(0), Register::A(0))),
            0x6F => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::L(0), Register::A(0))),
            0x02 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::BC(0), Register::A(0))),
            0x12 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::DE(0), Register::A(0))),
            0x77 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::HL(0), Register::A(0))),
            0xEA => CpuOpcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::A(0))),
            0xF2 => CpuOpcode::LD(OpcodeParameter::Register_FF00plusRegister(Register::A(0), Register::C(0))),
            0xE2 => CpuOpcode::LD(OpcodeParameter::FF00plusRegister_Register(Register::A(0), Register::C(0))),
            0x3A => CpuOpcode::LDD(OpcodeParameter::Register_RegisterDecrement(Register::A(0), Register::HL(0))),
            0x32 => CpuOpcode::LDD(OpcodeParameter::RegisterDecrement_Register(Register::HL(0), Register::A(0))),
            0x2A => CpuOpcode::LDI(OpcodeParameter::Register_RegisterIncrement(Register::A(0), Register::HL(0))),
            0x22 => CpuOpcode::LDI(OpcodeParameter::RegisterIncrement_Register(Register::HL(0), Register::A(0))),
            0xE0 => CpuOpcode::LD(OpcodeParameter::FF00plusU8_Register(Register::A(0))),
            0xF0 => CpuOpcode::LD(OpcodeParameter::Register_FF00plusU8(Register::A(0))),
            0x01 => CpuOpcode::LD(OpcodeParameter::Register_U16(Register::BC(0))),
            0x11 => CpuOpcode::LD(OpcodeParameter::Register_U16(Register::DE(0))),
            0x21 => CpuOpcode::LD(OpcodeParameter::Register_U16(Register::HL(0))),
            0x31 => CpuOpcode::LD(OpcodeParameter::Register_U16(Register::SP(two_byte_param))),
            0xF9 => CpuOpcode::LD(OpcodeParameter::Register_Register(Register::SP(0), Register::HL(0))),
            0xF8 => CpuOpcode::LD(OpcodeParameter::Register_RegisterPlusI8(Register::HL(0), Register::SP(0))),
            0x08 => CpuOpcode::LD(OpcodeParameter::U16_Register(two_byte_param, Register::SP(0))),
            0xC5 => CpuOpcode::PUSH(Register::BC(0)),
            0xD5 => CpuOpcode::PUSH(Register::DE(0)),
            0xE5 => CpuOpcode::PUSH(Register::HL(0)),
            0xF5 => CpuOpcode::PUSH(Register::AF(0)),
            0xC1 => CpuOpcode::POP(Register::BC(0)),
            0xD1 => CpuOpcode::POP(Register::DE(0)),
            0xE1 => CpuOpcode::POP(Register::HL(0)),
            0xF1 => CpuOpcode::POP(Register::AF(0)),
            0x87 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0x80 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0x81 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0x82 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0x83 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0x84 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0x85 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0x86 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xC6 => CpuOpcode::ADD(OpcodeParameter::Register_U8(Register::A(0))),
            0x8F => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0x88 => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0x89 => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0x8A => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0x8B => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0x8C => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0x8D => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0x8E => CpuOpcode::ADC(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xCE => CpuOpcode::ADC(OpcodeParameter::Register_U8(Register::A(0))),
            0x97 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0x90 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0x91 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0x92 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0x93 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0x94 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0x95 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0x96 => CpuOpcode::SUB(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xD6 => CpuOpcode::SUB(OpcodeParameter::Register_U8(Register::A(0))),
            0x9F => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0x98 => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0x99 => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0x9A => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0x9B => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0x9C => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0x9D => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0x9E => CpuOpcode::SBC(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xDE => CpuOpcode::SBC(OpcodeParameter::Register_U8(Register::A(0))),
            0xA7 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0xA0 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0xA1 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0xA2 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0xA3 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0xA4 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0xA5 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0xA6 => CpuOpcode::AND(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xE6 => CpuOpcode::AND(OpcodeParameter::Register_U8(Register::A(0))),
            0xB7 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0xB0 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0xB1 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0xB2 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0xB3 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0xB4 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0xB5 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0xB6 => CpuOpcode::OR(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xF6 => CpuOpcode::OR(OpcodeParameter::Register_U8(Register::A(0))),
            0xAF => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0xA8 => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0xA9 => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0xAA => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0xAB => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0xAC => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0xAD => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0xAE => CpuOpcode::XOR(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xEE => CpuOpcode::XOR(OpcodeParameter::Register_U8(Register::A(0))),
            0xBF => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::A(0))),
            0xB8 => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::B(0))),
            0xB9 => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::C(0))),
            0xBA => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::D(0))),
            0xBB => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::E(0))),
            0xBC => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::H(0))),
            0xBD => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::L(0))),
            0xBE => CpuOpcode::CP(OpcodeParameter::Register_Register(Register::A(0), Register::HL(0))),
            0xFE => CpuOpcode::CP(OpcodeParameter::Register_U8(Register::A(0))),
            0x3C => CpuOpcode::INC(Register::A(0)),
            0x04 => CpuOpcode::INC(Register::B(0)),
            0x0C => CpuOpcode::INC(Register::C(0)),
            0x14 => CpuOpcode::INC(Register::D(0)),
            0x1C => CpuOpcode::INC(Register::E(0)),
            0x24 => CpuOpcode::INC(Register::H(0)),
            0x2C => CpuOpcode::INC(Register::L(0)),
            0x34 => CpuOpcode::INC(Register::HL(0)),
            0x3D => CpuOpcode::DEC(Register::A(0)),
            0x05 => CpuOpcode::DEC(Register::B(0)),
            0x0D => CpuOpcode::DEC(Register::C(0)),
            0x15 => CpuOpcode::DEC(Register::D(0)),
            0x1D => CpuOpcode::DEC(Register::E(0)),
            0x25 => CpuOpcode::DEC(Register::H(0)),
            0x2D => CpuOpcode::DEC(Register::L(0)),
            0x35 => CpuOpcode::DEC(Register::HL(0)),
            0x09 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::HL(0), Register::BC(0))),
            0x19 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::HL(0), Register::DE(0))),
            0x29 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::HL(0), Register::HL(0))),
            0x39 => CpuOpcode::ADD(OpcodeParameter::Register_Register(Register::HL(0), Register::SP(0))),
            0xE8 => CpuOpcode::ADD(OpcodeParameter::Register_I8(Register::HL(0))),
            0x03 => CpuOpcode::INC(Register::BC(0)),
            0x13 => CpuOpcode::INC(Register::DE(0)),
            0x23 => CpuOpcode::INC(Register::HL(0)),
            0x33 => CpuOpcode::INC(Register::SP(0)),
            0x0B => CpuOpcode::DEC(Register::BC(0)),
            0x1B => CpuOpcode::DEC(Register::DE(0)),
            0x2B => CpuOpcode::DEC(Register::HL(0)),
            0x3B => CpuOpcode::DEC(Register::SP(0)),
            0x27 => CpuOpcode::DAA,
            0x2F => CpuOpcode::CPL,
            0x3F => CpuOpcode::CCF,
            0x37 => CpuOpcode::SCF,
            0x17 => CpuOpcode::RLA,
            0x07 => CpuOpcode::RLCA,
            0x0F => CpuOpcode::RRCA,
            0x1F => CpuOpcode::RRA,
            0xCB => CpuOpcode::PrefixCB,
            //0xCB => CpuOpcode::SWAP,
            //0xCB => CpuOpcode::RLC,
            //0xCB => CpuOpcode::RL,
            //0xCB => CpuOpcode::RRC,
            //0xCB => CpuOpcode::RR,
            //0xCB => CpuOpcode::SLA,
            //0xCB => CpuOpcode::SRA,
            //0xCB => CpuOpcode::SRL,
            //0xCB => CpuOpcode::BIT,
            //0xCB => CpuOpcode::SET,
            //0xCB => CpuOpcode::RES,
            0xC3 => CpuOpcode::JP(OpcodeParameter::U16(two_byte_param)),
            0xC2 => CpuOpcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero(true), 0)),
            0xCA => CpuOpcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero(true), 0)),
            0xD2 => CpuOpcode::JP(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry(true), 0)),
            0xDA => CpuOpcode::JP(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry(true), 0)),
            0xE9 => CpuOpcode::JP(OpcodeParameter::Register(Register::HL(0))),
            0x18 => CpuOpcode::JR(OpcodeParameter::I8(0)),
            0x20 => CpuOpcode::JR(OpcodeParameter::FlagRegisterReset_I16(FlagRegister::Zero(true), 0)),
            0x28 => CpuOpcode::JR(OpcodeParameter::FlagRegisterSet_I16(FlagRegister::Zero(true), 0)),
            0x30 => CpuOpcode::JR(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry(true), 0)),
            0x38 => CpuOpcode::JR(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry(true), 0)),
            0xCD => CpuOpcode::CALL(OpcodeParameter::U16(0)),
            0xC4 => CpuOpcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Zero(true), 0)),
            0xCC => CpuOpcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Zero(true), 0)),
            0xD4 => CpuOpcode::CALL(OpcodeParameter::FlagRegisterReset_U16(FlagRegister::Carry(true), 0)),
            0xDC => CpuOpcode::CALL(OpcodeParameter::FlagRegisterSet_U16(FlagRegister::Carry(true), 0)),
            0xC7 => CpuOpcode::RST(0x00),
            0xCF => CpuOpcode::RST(0x08),
            0xD7 => CpuOpcode::RST(0x10),
            0xDF => CpuOpcode::RST(0x18),
            0xE7 => CpuOpcode::RST(0x20),
            0xEF => CpuOpcode::RST(0x28),
            0xF7 => CpuOpcode::RST(0x30),
            0xFF => CpuOpcode::RST(0x38),
            0xC9 => CpuOpcode::RET(OpcodeParameter::NoParam),
            0xC0 => CpuOpcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Zero(true))),
            0xC8 => CpuOpcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Zero(true))),
            0xD0 => CpuOpcode::RET(OpcodeParameter::FlagRegisterReset(FlagRegister::Carry(true))),
            0xD8 => CpuOpcode::RET(OpcodeParameter::FlagRegisterSet(FlagRegister::Carry(true))),
            0xD9 => CpuOpcode::RETI,
            0xF3 => CpuOpcode::DI,
            0xFB => CpuOpcode::EI,
            0x76 => CpuOpcode::HALT,
            0x10 => CpuOpcode::STOP,
            0x00 => CpuOpcode::NOP,
            _ => CpuOpcode::IllegalInstruction,
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
        registers.set(&Register::A(0b01010101));
        assert_eq!(registers.get(&Register::A(0)), 0b01010101);
        registers.set(&Register::F(0b01010101));
        assert_eq!(registers.get(&Register::F(0)), 0b01010101);
        registers.set(&Register::B(0b01010101));
        assert_eq!(registers.get(&Register::B(0)), 0b01010101);
        registers.set(&Register::C(0b01010101));
        assert_eq!(registers.get(&Register::C(0)), 0b01010101);
        registers.set(&Register::D(0b01010101));
        assert_eq!(registers.get(&Register::D(0)), 0b01010101);
        registers.set(&Register::E(0b01010101));
        assert_eq!(registers.get(&Register::E(0)), 0b01010101);
        registers.set(&Register::H(0b01010101));
        assert_eq!(registers.get(&Register::H(0)), 0b01010101);
        registers.set(&Register::L(0b01010101));
        assert_eq!(registers.get(&Register::L(0)), 0b01010101);

        // Test 16 bit setters and getters
        let mut registers = Registers::new();
        registers.set(&Register::A(0b01010101));
        registers.set(&Register::F(0b11111111));
        assert_eq!(registers.get(&Register::AF(0)), 0b0101010111111111);
        registers.set(&Register::AF(0b1111111101010101));
        assert_eq!(registers.get(&Register::AF(0)), 0b1111111101010101);

        registers.set(&Register::B(0b01010101));
        registers.set(&Register::C(0b11111111));
        assert_eq!(registers.get(&Register::BC(0)), 0b0101010111111111);
        registers.set(&Register::BC(0b1111111101010101));
        assert_eq!(registers.get(&Register::BC(0)), 0b1111111101010101);

        registers.set(&Register::D(0b01010101));
        registers.set(&Register::E(0b11111111));
        assert_eq!(registers.get(&Register::DE(0)), 0b0101010111111111);
        registers.set(&Register::DE(0b1111111101010101));
        assert_eq!(registers.get(&Register::DE(0)), 0b1111111101010101);

        registers.set(&Register::H(0b01010101));
        registers.set(&Register::L(0b11111111));
        assert_eq!(registers.get(&Register::HL(0)), 0b0101010111111111);
        registers.set(&Register::HL(0b1111111101010101));
        assert_eq!(registers.get(&Register::HL(0)), 0b1111111101010101);

        registers.set(&Register::SP(0b0101010111111111));
        assert_eq!(registers.get(&Register::SP(0)), 0b0101010111111111);

        registers.set(&Register::PC(0b0101010111111111));
        assert_eq!(registers.get(&Register::PC(0)), 0b0101010111111111);
    }

    #[test]
    fn test_cpu_instructions() {
        // LD
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(CpuOpcode::LD(OpcodeParameter::Register_U16(Register::SP(0xF1F1))), &mut bus);
        assert_eq!(cpu.registers.get(&Register::SP(0xF1F1)), 0xF1F1);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x103);
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set(&Register::SP(0x1234));
        cpu.exec(CpuOpcode::LD(OpcodeParameter::U16_Register(0xF0F0, Register::SP(0))), &mut bus);
        assert_eq!(bus.read(0xF0F0), 0x34);
        assert_eq!(bus.read(0xF0F1), 0x12);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x103);

        // JP
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(CpuOpcode::JP(OpcodeParameter::U16(0x1F1F)), &mut bus);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x1F1F);

        // DI
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(CpuOpcode::DI, &mut bus);
        assert_eq!(bus.read(0xFFFF), 0x00);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);

        // RLCA
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.registers.set(&Register::A(0b00000010));
        cpu.exec(CpuOpcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(&Register::A(0)), 0b00000010);
        assert_eq!(cpu.registers.get_flag(&FlagRegister::Carry(false)), false);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(&Register::A(0b00000001));
        cpu.exec(CpuOpcode::RLCA, &mut bus);
        assert_eq!(cpu.registers.get(&Register::A(0)), 0b00000001);
        assert_eq!(cpu.registers.get_flag(&FlagRegister::Carry(true)), true);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);

        // RRCA
        let mut cpu = CPU::new();
        cpu.registers.set(&Register::A(0b01000000));
        cpu.exec(CpuOpcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(&Register::A(0)), 0b01000000);
        assert_eq!(cpu.registers.get_flag(&FlagRegister::Carry(false)), false);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);
        let mut cpu = CPU::new();
        cpu.registers.set(&Register::A(0b10000000));
        cpu.exec(CpuOpcode::RRCA, &mut bus);
        assert_eq!(cpu.registers.get(&Register::A(0)), 0b10000000);
        assert_eq!(cpu.registers.get_flag(&FlagRegister::Carry(true)), true);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);

        // CALL
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(&Register::SP(sp));
        cpu.registers.set(&Register::PC(0x1234));
        cpu.exec(CpuOpcode::CALL(OpcodeParameter::U16(0xF0F0)), &mut bus);
        assert_eq!(bus.read(sp - 2), 0x34);
        assert_eq!(bus.read(sp - 1), 0x12);
        assert_eq!(cpu.registers.get(&Register::SP(0)), sp - 2);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0xF0F0);

        // RST
        let mut cpu = CPU::new();
        let sp = 0xFFDF;
        cpu.registers.set(&Register::SP(sp));
        cpu.registers.set(&Register::PC(0x1234));
        cpu.exec(CpuOpcode::RST(0xF0), &mut bus);
        assert_eq!(bus.read(sp - 2), 0x34);
        assert_eq!(bus.read(sp - 1), 0x12);
        assert_eq!(cpu.registers.get(&Register::SP(0)), sp - 2);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x00F0);

        // NOP
        let mut cpu = CPU::new();
        let mut bus = Bus::new();
        cpu.exec(CpuOpcode::NOP, &mut bus);
        assert_eq!(cpu.registers.get(&Register::PC(0)), 0x101);
    }
}
