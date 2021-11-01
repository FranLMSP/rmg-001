use crate::bus::{Bus};
use crate::utils::{BitIndex, get_bit};

pub const JOYPAD_ADDRESS: u16 = 0xFF00;

#[derive(Debug, Copy, Clone)]
pub enum Button {
    A,
    B,
    Up,
    Down,
    Left,
    Right,
    Start,
    Select
}

pub struct Joypad {
    a: bool,
    b: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    start: bool,
    select: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            a: false,
            b: false,
            up: false,
            down: false,
            left: false,
            right: false,
            start: false,
            select: false,
        }
    }

    pub fn press(&mut self, button: Button) {
        println!("{:?} pressed", button);
        match button {
            Button::A      => self.a = true,
            Button::B      => self.b = true,
            Button::Up     => self.up = true,
            Button::Down   => self.down = true,
            Button::Left   => self.left = true,
            Button::Right  => self.right = true,
            Button::Start  => self.start = true,
            Button::Select => self.select = true,
        };
    }

    pub fn release(&mut self, button: Button) {
        println!("{:?} released", button);
        match button {
            Button::A      => self.a = false,
            Button::B      => self.b = false,
            Button::Up     => self.up = false,
            Button::Down   => self.down = false,
            Button::Left   => self.left = false,
            Button::Right  => self.right = false,
            Button::Start  => self.start = false,
            Button::Select => self.select = false,
        };
    }

    pub fn update(&self, bus: &mut Bus) {
        let byte = bus.read(JOYPAD_ADDRESS);
        let direction = !get_bit(byte, BitIndex::I4);
        let action = !get_bit(byte, BitIndex::I5);

        let data = 0b11000000 |
        (byte & 0b00110000) |
        (
            (!((direction && self.down) || (action && self.start)) as u8) << 3
        ) | (
            (!((direction && self.up) || (action && self.select)) as u8) << 2
        ) | (
            (!((direction && self.left) || (action && self.b)) as u8) << 1
        ) | (
            (!((direction && self.right) || (action && self.a)) as u8)
        );
        println!("New joypad write: {:08b}", data);
        bus.force_write(JOYPAD_ADDRESS, data);
    }
}