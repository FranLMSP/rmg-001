pub enum BitIndex {
    I0,
    I1,
    I2,
    I3,
    I4,
    I5,
    I6,
    I7,
}

pub fn get_bit_index(index: BitIndex) -> u8 {
    match index {
        BitIndex::I0 => 0,
        BitIndex::I1 => 1,
        BitIndex::I2 => 2,
        BitIndex::I3 => 3,
        BitIndex::I4 => 4,
        BitIndex::I5 => 5,
        BitIndex::I6 => 6,
        BitIndex::I7 => 7,
    }
}

pub fn get_bit(byte: u8, index: BitIndex) -> bool {
    ((byte >> get_bit_index(index)) & 0b00000001) == 1
}

pub fn set_bit(byte: u8, value: bool, index: BitIndex) -> u8 {
    match value {
        true => 0b00000001 << get_bit_index(index) | byte,
        false => ((0b0000001 << get_bit_index(index)) ^ 0b11111111) & byte,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_bit() {
        assert_eq!(set_bit(0b00000000, true, BitIndex::I0), 0b00000001);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I1), 0b00000010);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I2), 0b00000100);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I3), 0b00001000);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I4), 0b00010000);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I5), 0b00100000);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I6), 0b01000000);
        assert_eq!(set_bit(0b00000000, true, BitIndex::I7), 0b10000000);

        assert_eq!(set_bit(0b11111111, false, BitIndex::I0), 0b11111110);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I1), 0b11111101);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I2), 0b11111011);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I3), 0b11110111);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I4), 0b11101111);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I5), 0b11011111);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I6), 0b10111111);
        assert_eq!(set_bit(0b11111111, false, BitIndex::I7), 0b01111111);

        // Just a couple of random test
        assert_eq!(set_bit(0b00000001, true, BitIndex::I0), 0b00000001);
        assert_eq!(set_bit(0b11101111, true, BitIndex::I4), 0b11111111);
        assert_eq!(set_bit(0b11111110, false, BitIndex::I0), 0b11111110);
        assert_eq!(set_bit(0b00010000, false, BitIndex::I4), 0b00000000);
    }

    #[test]
    fn test_get_bit() {
        assert_eq!(get_bit(0b00000001, BitIndex::I0), true);
        assert_eq!(get_bit(0b00000010, BitIndex::I1), true);
        assert_eq!(get_bit(0b00000100, BitIndex::I2), true);
        assert_eq!(get_bit(0b00001000, BitIndex::I3), true);
        assert_eq!(get_bit(0b00010000, BitIndex::I4), true);
        assert_eq!(get_bit(0b00100000, BitIndex::I5), true);
        assert_eq!(get_bit(0b01000000, BitIndex::I6), true);
        assert_eq!(get_bit(0b10000000, BitIndex::I7), true);

        assert_eq!(get_bit(0b11111110, BitIndex::I0), false);
        assert_eq!(get_bit(0b11111101, BitIndex::I1), false);
        assert_eq!(get_bit(0b11111011, BitIndex::I2), false);
        assert_eq!(get_bit(0b11110111, BitIndex::I3), false);
        assert_eq!(get_bit(0b11101111, BitIndex::I4), false);
        assert_eq!(get_bit(0b11011111, BitIndex::I5), false);
        assert_eq!(get_bit(0b10111111, BitIndex::I6), false);
        assert_eq!(get_bit(0b01111111, BitIndex::I7), false);
    }
}
