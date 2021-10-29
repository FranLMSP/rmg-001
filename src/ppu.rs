use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
    to_bit_index,
};
use crate::bus::{Bus, AddressRange, BANK_ZERO, VIDEO_RAM};
use crate::cpu::{Cycles};

#[derive(Debug, Copy, Clone)]
enum Pixel {
    White,
    Light,
    Dark,
    Black,
}

#[derive(Debug, Copy, Clone)]
struct ColorPalette(u8, u8, u8, u8);

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

pub const LCD_WIDTH: u32 = 160;
pub const LCD_HEIGHT: u32 = 144;
pub const WIDTH: u32 = LCD_WIDTH;
pub const HEIGHT: u32 = LCD_HEIGHT;
pub const FRAME_BUFFER_LENGTH: u32 = WIDTH * HEIGHT;

const LCD_CONTROL_ADDRESS: u16 = 0xFF40;
const LCD_STATUS_ADDRESS: u16 = 0xFF41;

const SCROLL_Y_ADDRESS: u16 = 0xFF42;
const SCROLL_X_ADDRESS: u16 = 0xFF43;
const LCD_Y_ADDRESS: u16 = 0xFF44;
const LCD_Y_COMPARE_ADDRESS: u16 = 0xFF45;
const DMA_ADDRESS: u16 = 0xFF46;
const BACKGROUND_PALETTE_ADDRESS: u16 = 0xFF47;
const OBJECT_PALETTE_0_ADDRESS: u16 = 0xFF48;
const OBJECT_PALETTE_1_ADDRESS: u16 = 0xFF49;
const WINDOW_X_ADDRESS: u16 = 0xFF4A;
const WINDOW_Y_ADDRESS: u16 = 0xFF4B;
const TILE_MAP_ADDRESS: u16 = 0x9800;

pub struct PPU {
    cycles: Cycles,
    rgba_frame: [[u8; 4]; FRAME_BUFFER_LENGTH as usize],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            cycles: Cycles(0),
            rgba_frame: [[0xFF, 0xFF, 0xFF, 0]; FRAME_BUFFER_LENGTH as usize],
        }
    }

    pub fn reset_cycles(&mut self) {
        self.cycles.0 = 0;
    }

    pub fn increment_cycles(&mut self, cycles: Cycles) {
        self.cycles.0 += cycles.0;
    }

    pub fn do_cycle(&mut self, bus: &mut Bus) {
        // Mode 1 Vertical blank
        if PPU::get_lcd_y(bus) >= 144 {
            PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank), true);
        } else {
            if self.cycles.0 <= 80 {
                // Mode 2 OAM scan
                PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM), true);
            } else if self.cycles.0 <= 80 + 172 {
                // Mode 3 drawing pixel line. This could also last 289 cycles
                if !PPU::get_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD)) {
                    self.draw_line(bus);
                    PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD), true);
                }
            } else if self.cycles.0 <= 80 + 172 + 204 {
                // Mode 0 Horizontal blank. This could last 87 or 204 cycles depending on the mode 3
                PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank), true);
            }
        }

        self.increment_cycles(Cycles(1));

        // Horizontal scan completed
        if self.cycles.0 > 456 {
            self.reset_cycles();
            PPU::set_lcd_y(bus, PPU::get_lcd_y(bus) + 1);

            // Frame completed
            if PPU::get_lcd_y(bus) > 153 {
                PPU::set_lcd_y(bus, 0);
            }
        }
    }

    fn get_lcd_y(bus: &Bus) -> u8 {
        bus.read(LCD_Y_ADDRESS)
    }

    fn set_lcd_y(bus: &mut Bus, val: u8) {
        bus.write(LCD_Y_ADDRESS, val);
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

    pub fn draw_line(&mut self, bus: &Bus) {
        let lcd_y = PPU::get_lcd_y(bus);
        if lcd_y as u32 >= LCD_HEIGHT {
            return;
        }
        let mut lcd_x: u8 = 0;
        while (lcd_x as u32) < LCD_WIDTH {
            let y = lcd_y.wrapping_add(PPU::get_scroll_y(bus));
            let x = lcd_x.wrapping_add(PPU::get_scroll_x(bus));
            let index_x = x as u16 / 8;
            let index_y = (y as u16 / 8) * 32;
            let index = index_x + index_y;
            let tile_line = (y).rem_euclid(8) * 2;
            let tile_number = bus.read(0x9800 + index as u16) as u16;
            let addr = 0x8000 + tile_line as u16 + (tile_number * 16);

            let tile_byte_1 = bus.read(addr);
            let tile_byte_2 = bus.read(addr + 1);

            let pixels = PPU::get_byte_pixels(tile_byte_1, tile_byte_2);

            for pixel in pixels {
                let idx = lcd_x as usize + (lcd_y as usize * LCD_WIDTH as usize);
                self.rgba_frame[idx] = PPU::get_rgba(pixel);
                lcd_x += 1;
            }
        }
    }
    
    pub fn draw_background(&mut self, bus: &mut Bus) {
        let mut lcd_y: u8 = 0;
        PPU::set_lcd_y(bus, lcd_y);
        while lcd_y < 144 {
            self.draw_line(bus);
            lcd_y += 1;
            PPU::set_lcd_y(bus, lcd_y);
        }
    }
    
    /* pub fn draw_background_old(&mut self, bus: &Bus) {
        let mut idx = 0;
        // let mut tile_line: u16 = 0;
        let mut lcd_y: u8 = 0;
        while lcd_y < 144 {
            let mut lcd_x: u8 = 0;
            while lcd_x < 160 {
                let y = lcd_y.wrapping_add(PPU::get_scroll_y(bus));
                let x = lcd_x.wrapping_add(PPU::get_scroll_x(bus));
                let index_x = (x as u16 / 8);
                let index_y = (y as u16 / 8) * 32;
                let index = index_x + index_y;
                let tile_line = (y).rem_euclid(8) * 2;
                let index_byte = (bus.read(0x9800 + index as u16) as u16) * 16;

                let tile_byte_1 = bus.read(0x8000 + tile_line as u16 + index_byte);
                let tile_byte_2 = bus.read(0x8000 + tile_line as u16 + index_byte + 1);

                let pixels = PPU::get_byte_pixels(tile_byte_1, tile_byte_2);

                for pixel in pixels {
                    self.rgba_frame[idx] = PPU::get_rgba(pixel);
                    idx += 1;
                }

                lcd_x += 8;
            }
            lcd_y += 1;
            // tile_line += 2;
        }
    } */

    fn get_pixel(two_bit_pixel: u8) -> Pixel {
        match two_bit_pixel {
            0x00 => Pixel::White,
            0x01 => Pixel::Light,
            0x10 => Pixel::Dark,
            0x11 => Pixel::Black,
            _ => Pixel::Black,
        }
    }

    fn get_rgba(pixel: Pixel) -> [u8; 4] {
        match pixel {
            Pixel::White => [255, 255, 255, 0],
            Pixel::Light => [192, 192, 192, 0],
            Pixel::Dark  => [81, 81, 81, 0],
            Pixel::Black => [0, 0, 0, 0],
        }
    }

    fn get_byte_pixels(byte1: u8, byte2: u8) -> [Pixel; 8] {
        let mut pixels: [Pixel; 8] = [Pixel::White; 8];
        pixels[0] = PPU::get_pixel(((get_bit(byte1, BitIndex::I7) as u8) << 1) | (get_bit(byte2, BitIndex::I7) as u8));
        pixels[1] = PPU::get_pixel(((get_bit(byte1, BitIndex::I6) as u8) << 1) | (get_bit(byte2, BitIndex::I6) as u8));
        pixels[2] = PPU::get_pixel(((get_bit(byte1, BitIndex::I5) as u8) << 1) | (get_bit(byte2, BitIndex::I5) as u8));
        pixels[3] = PPU::get_pixel(((get_bit(byte1, BitIndex::I4) as u8) << 1) | (get_bit(byte2, BitIndex::I4) as u8));
        pixels[4] = PPU::get_pixel(((get_bit(byte1, BitIndex::I3) as u8) << 1) | (get_bit(byte2, BitIndex::I3) as u8));
        pixels[5] = PPU::get_pixel(((get_bit(byte1, BitIndex::I2) as u8) << 1) | (get_bit(byte2, BitIndex::I2) as u8));
        pixels[6] = PPU::get_pixel(((get_bit(byte1, BitIndex::I1) as u8) << 1) | (get_bit(byte2, BitIndex::I1) as u8));
        pixels[7] = PPU::get_pixel(((get_bit(byte1, BitIndex::I0) as u8) << 1) | (get_bit(byte2, BitIndex::I0) as u8));
        pixels
    }

    pub fn get_rgba_frame(&self) -> &[[u8; 4]; FRAME_BUFFER_LENGTH as usize] {
        &self.rgba_frame
    }
}
