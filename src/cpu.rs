pub enum Register {
    A(u8),
    B(u8),
    C(u8),
    D(u8),
    E(u8),
    F(u8),
    H(u8),
    L(u8),
    SP(u16),
    PC(u16),
}

pub struct Registers {
    a: Register,
    b: Register,
    c: Register,
    d: Register,
    e: Register,
    f: Register,
    h: Register,
    sp: Register,
    pc: Register,
}

pub struct CPU {
    registers: Registers,
}
