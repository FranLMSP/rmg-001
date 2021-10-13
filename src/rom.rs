use std::fs::File;
use std::io::Read;

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

    pub fn print_content(&self) {
        println!("{:02X?}", self.bytes);
    }
}
