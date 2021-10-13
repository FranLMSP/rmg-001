pub enum MemoryMap {
    BankZero,
    BankSwitchable,
    VideoRAM,
    ExternalRAM,
    WorkRAM1,
    WorkRAM2,
    EchoRAM,
    SpriteAttributeTable,
    NotUsable,
    IORegisters,
    HighRAM,
    InterruptEnableRegister,
}

impl MemoryMap {
    pub fn get_map(address: u16) -> Self {
        if address <= 0x3FFF {return Self::BankZero;}
        if address >= 0x4000 && address <= 0x7FFF {return Self::BankSwitchable;}
        if address >= 0x8000 && address <= 0x9FFF {return Self::VideoRAM;}
        if address >= 0xA000 && address <= 0xBFFF {return Self::ExternalRAM;}
        if address >= 0xC000 && address <= 0xCFFF {return Self::WorkRAM1;}
        if address >= 0xD000 && address <= 0xDFFF {return Self::WorkRAM2;}
        if address >= 0xE000 && address <= 0xFDFF {return Self::EchoRAM;} // Mirror of C000~DDFF
        if address >= 0xFE00 && address <= 0xFE9F {return Self::SpriteAttributeTable;}
        if address >= 0xFEA0 && address <= 0xFEFF {return Self::NotUsable;}
        if address >= 0xFF00 && address <= 0xFF7F {return Self::IORegisters;}
        if address >= 0xFF80 && address <= 0xFFFE {return Self::HighRAM;}
        if address == 0xFFFF {return Self::InterruptEnableRegister;}
        Self::BankZero
    }
}
