use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
};
use crate::bus::Bus;

struct ColorPalette(u8, u8, u8, u8);

struct Tile {

}

struct Sprite {
}


pub enum LCDControl {
    DisplayEnable,
    WindowTileMapAddress,
    WindowEnable,
    BackgroundWindowTileAddress,
    BackgroundTileMapAddress,
    ObjectSize,
    ObjectEnable,
    BackgroundPriority,
}

pub enum LCDStatusModeFlag {
    HBlank,
    VBlank,
    SearchingOAM,
    TransferringToLCD,
}

pub enum LCDStatus {
    LYCInterrupt,
    Mode2OAMInterrupt,
    Mode1VBlankInterrupt,
    Mode0HBlankInterrupt,
    LYCFlag,
    ModeFlag(LCDStatusModeFlag),
}

pub struct Window {
    x: u8,
    y: u8,
}

impl Window {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
        }
    }
}

pub struct PPU {
    scroll_y: u8,
    scroll_x: u8,
    window: Window,
}

const LCD_CONTROL_ADDRESS: u16 = 0xFF40;
const LCD_STATUS_ADDRESS: u16 = 0xFF41;

impl PPU {
    pub fn new() -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            window: Window::new(),
        }
    }

    fn get_lcd_control(bus: &Bus, control: LCDControl) -> bool {
        let byte = bus.read(LCD_CONTROL_ADDRESS);
        match control {
            LCDControl::DisplayEnable => get_bit(byte, BitIndex::I7),
            LCDControl::WindowTileMapAddress => get_bit(byte, BitIndex::I6),
            LCDControl::WindowEnable =>  get_bit(byte, BitIndex::I5),
            LCDControl::BackgroundWindowTileAddress =>  get_bit(byte, BitIndex::I4),
            LCDControl::BackgroundTileMapAddress =>  get_bit(byte, BitIndex::I3),
            LCDControl::ObjectSize =>  get_bit(byte, BitIndex::I2),
            LCDControl::ObjectEnable =>  get_bit(byte, BitIndex::I1),
            LCDControl::BackgroundPriority =>  get_bit(byte, BitIndex::I0),
        }
    }

    fn set_lcd_control(bus: &mut Bus, control: LCDControl, val: bool) {
        let mut byte = bus.read(LCD_CONTROL_ADDRESS);
        match control {
            LCDControl::DisplayEnable => byte = set_bit(byte, val, BitIndex::I7),
            LCDControl::WindowTileMapAddress => byte = set_bit(byte, val, BitIndex::I6),
            LCDControl::WindowEnable => byte = set_bit(byte, val, BitIndex::I5),
            LCDControl::BackgroundWindowTileAddress => byte = set_bit(byte, val, BitIndex::I4),
            LCDControl::BackgroundTileMapAddress => byte = set_bit(byte, val, BitIndex::I3),
            LCDControl::ObjectSize => byte = set_bit(byte, val, BitIndex::I2),
            LCDControl::ObjectEnable => byte = set_bit(byte, val, BitIndex::I1),
            LCDControl::BackgroundPriority => byte = set_bit(byte, val, BitIndex::I0),
        };
        bus.write(LCD_CONTROL_ADDRESS, byte);
    }

    fn get_lcd_status(bus: &Bus, status: LCDStatus) -> bool {
        let byte = bus.read(LCD_STATUS_ADDRESS);
        match status {
            LCDStatus::LYCInterrupt => get_bit(byte, BitIndex::I6),
            LCDStatus::Mode2OAMInterrupt => get_bit(byte, BitIndex::I5),
            LCDStatus::Mode1VBlankInterrupt => get_bit(byte, BitIndex::I4),
            LCDStatus::Mode0HBlankInterrupt => get_bit(byte, BitIndex::I3),
            LCDStatus::LYCFlag => get_bit(byte, BitIndex::I2),
            LCDStatus::ModeFlag(mode) => match mode {
                LCDStatusModeFlag::HBlank => (byte & 0b00000011) == 0,
                LCDStatusModeFlag::VBlank => (byte & 0b00000011) == 1,
                LCDStatusModeFlag::SearchingOAM => (byte & 0b00000011) == 2,
                LCDStatusModeFlag::TransferringToLCD => (byte & 0b00000011) == 3,
            },
        }
    }
}
