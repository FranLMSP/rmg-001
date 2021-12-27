pub const WRAM_BANK_SELECT_ADDRESS: u16 = 0xFF70;

pub trait RAM {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub struct DMGRAM {
    data: [u8; 4096 * 2],
}

impl DMGRAM {
    pub fn new() -> Self {
        Self {
            data: [0; 4096 * 2],
        }
    }
}

impl RAM for DMGRAM {
    fn read(&self, address: u16) -> u8 {
        if address == WRAM_BANK_SELECT_ADDRESS {
            return 0xFF;
        }
        self.data[(address - 0xC000) as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        if address == WRAM_BANK_SELECT_ADDRESS {
            return;
        }
        self.data[(address - 0xC000) as usize] = value;
    }
}


pub struct CGBRAM {
    data: [u8; 4096 * 8],
    bank: u8,

}

impl CGBRAM {
    pub fn new() -> Self {
        Self {
            data: [0; 4096 * 8],
            bank: 1,
        }
    }

    fn switch_bank(&mut self, bank: u8) {
        self.bank = bank;
        if self.bank > 7 {
            self.bank = 7;
        } else if bank == 0 {
            self.bank = 1;
        }
    }
}

impl RAM for CGBRAM {
    fn read(&self, address: u16) -> u8 {
        if address == WRAM_BANK_SELECT_ADDRESS {
            return self.bank;
        }
        if address <= 0xCFFF {
            return self.data[(address - 0xC000) as usize];
        }
        self.data[((address - 0xD000) as usize) + (4096 * (self.bank as usize))]
    }

    fn write(&mut self, address: u16, value: u8) {
        if address == WRAM_BANK_SELECT_ADDRESS {
            return self.switch_bank(value);
        } else if address <= 0xCFFF {
            return self.data[(address - 0xC000) as usize] = value;
        }
        self.data[((address - 0xD000) as usize) + (4096 * (self.bank as usize))] = value;
    }
}
