use std::fs::File;
use std::io::Read;

pub struct ROMInfo {
}

impl ROMInfo {
}

pub struct ROM {
    bytes: Vec<u8>,
}

impl ROM {
    pub fn load_file(filename: String) -> std::io::Result<Self> {
        let mut file = File::open(filename)?;
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        Ok(Self {
            bytes,
        })
    }

    pub fn read(&self, address: u16) -> u8 {
        match self.bytes.get(address as usize) {
            Some(val) => *val,
            None => 0xFF,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    pub fn print_content(&self, address: Option<u16>) {
        match address {
            Some(address) => println!("{:02X?}", self.read(address)),
            None => println!("{:02X?}", self.bytes),
        };
    }
}
