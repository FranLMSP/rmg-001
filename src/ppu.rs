use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
};
use crate::bus::{Bus, BANK_ZERO};

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

const LCD_CONTROL_ADDRESS: u16 = 0xFF40;
const LCD_STATUS_ADDRESS: u16 = 0xFF41;

const SCROLL_X_ADDRESS: u16 = 0xFF42;
const SCROLL_Y_ADDRESS: u16 = 0xFF43;
const LCD_Y_ADDRESS: u16 = 0xFF44;
const LCD_Y_COMPARE_ADDRESS: u16 = 0xFF45;
const DMA_ADDRESS: u16 = 0xFF46;
const BACKGROUND_PALETTE_ADDRESS: u16 = 0xFF47;
const OBJECT_PALETTE_0_ADDRESS: u16 = 0xFF48;
const OBJECT_PALETTE_1_ADDRESS: u16 = 0xFF49;
const WINDOW_X_ADDRESS: u16 = 0xFF4A;
const WINDOW_Y_ADDRESS: u16 = 0xFF4B;

pub struct Window {}

impl Window {
    pub fn new() -> Self {
        Self {}
    }

    fn get_x(bus: &Bus) -> u8 {
        bus.read(WINDOW_X_ADDRESS)
    }

    fn set_x(bus: &mut Bus, val: u8) {
        bus.write(WINDOW_X_ADDRESS, val);
    }

    fn get_y(bus: &Bus) -> u8 {
        bus.read(WINDOW_Y_ADDRESS)
    }

    fn set_y(bus: &mut Bus, val: u8) {
        bus.write(WINDOW_Y_ADDRESS, val);
    }
}

pub struct PPU {
    window: Window,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            window: Window::new(),
        }
    }

    fn get_sprite(address: u16) {

    }

    fn get_scroll_x(bus: &Bus) -> u8 {
        bus.read(SCROLL_X_ADDRESS)
    }

    fn set_scroll_x(bus: &mut Bus, val: u8) {
        bus.write(SCROLL_X_ADDRESS, val);
    }

    fn get_scroll_y(bus: &Bus) -> u8 {
        bus.read(SCROLL_Y_ADDRESS)
    }

    fn set_scroll_y(bus: &mut Bus, val: u8) {
        bus.write(SCROLL_Y_ADDRESS, val);
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
        byte = match control {
            LCDControl::DisplayEnable => set_bit(byte, val, BitIndex::I7),
            LCDControl::WindowTileMapAddress => set_bit(byte, val, BitIndex::I6),
            LCDControl::WindowEnable => set_bit(byte, val, BitIndex::I5),
            LCDControl::BackgroundWindowTileAddress => set_bit(byte, val, BitIndex::I4),
            LCDControl::BackgroundTileMapAddress => set_bit(byte, val, BitIndex::I3),
            LCDControl::ObjectSize => set_bit(byte, val, BitIndex::I2),
            LCDControl::ObjectEnable => set_bit(byte, val, BitIndex::I1),
            LCDControl::BackgroundPriority => set_bit(byte, val, BitIndex::I0),
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

    fn set_lcd_status(bus: &mut Bus, status: LCDStatus, val: bool) {
        let mut byte = bus.read(LCD_STATUS_ADDRESS);
        byte = match status {
            LCDStatus::LYCInterrupt => set_bit(byte, val, BitIndex::I6),
            LCDStatus::Mode2OAMInterrupt => set_bit(byte, val, BitIndex::I5),
            LCDStatus::Mode1VBlankInterrupt => set_bit(byte, val, BitIndex::I4),
            LCDStatus::Mode0HBlankInterrupt => set_bit(byte, val, BitIndex::I3),
            LCDStatus::LYCFlag => set_bit(byte, val, BitIndex::I2),
            LCDStatus::ModeFlag(mode) => match mode {
                LCDStatusModeFlag::HBlank => (byte & 0b11111100) | 0,
                LCDStatusModeFlag::VBlank => (byte & 0b11111100) | 1,
                LCDStatusModeFlag::SearchingOAM => (byte & 0b11111100) | 2,
                LCDStatusModeFlag::TransferringToLCD => (byte & 0b11111100) | 3,
            },
        };
        bus.write(LCD_STATUS_ADDRESS, byte);
    }
}
