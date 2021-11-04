use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
    to_bit_index,
};
use crate::bus::{Bus, AddressRange, BANK_ZERO, VIDEO_RAM};
use crate::cpu::{Cycles, Interrupt};

pub const LCD_WIDTH: u32 = 160;
pub const LCD_HEIGHT: u32 = 144;
pub const WIDTH: u32 = LCD_WIDTH;
pub const HEIGHT: u32 = LCD_HEIGHT;
pub const FRAME_BUFFER_LENGTH: u32 = WIDTH * HEIGHT;

pub const LCD_CONTROL_ADDRESS: u16 = 0xFF40;
pub const LCD_STATUS_ADDRESS: u16 = 0xFF41;

pub const SCROLL_Y_ADDRESS: u16 = 0xFF42;
pub const SCROLL_X_ADDRESS: u16 = 0xFF43;
pub const LCD_Y_ADDRESS: u16 = 0xFF44;
pub const LCD_Y_COMPARE_ADDRESS: u16 = 0xFF45;
pub const DMA_ADDRESS: u16 = 0xFF46;
pub const BACKGROUND_PALETTE_ADDRESS: u16 = 0xFF47;
pub const OBJECT_PALETTE_0_ADDRESS: u16 = 0xFF48;
pub const OBJECT_PALETTE_1_ADDRESS: u16 = 0xFF49;
pub const WINDOW_X_ADDRESS: u16 = 0xFF4A;
pub const WINDOW_Y_ADDRESS: u16 = 0xFF4B;
pub const TILE_MAP_ADDRESS: u16 = 0x9800;

#[derive(Debug, Copy, Clone)]
enum Pixel {
    White,
    Light,
    Dark,
    Black,
}

#[derive(Debug, Copy, Clone)]
struct ColorPalette(u8, u8, u8, u8);

#[derive(Debug, Copy, Clone)]
pub enum LCDControl {
    LCDEnable,
    WindowTileMapAddress,
    WindowEnable,
    TileAddressMode,
    BackgroundTileMapAddress,
    ObjectSize,
    ObjectEnable,
    BackgroundPriority,
}

impl LCDControl {
    fn index(&self) -> BitIndex {
        match self {
            LCDControl::LCDEnable                => BitIndex::I7,
            LCDControl::WindowTileMapAddress     => BitIndex::I6,
            LCDControl::WindowEnable             => BitIndex::I5,
            LCDControl::TileAddressMode          => BitIndex::I4,
            LCDControl::BackgroundTileMapAddress => BitIndex::I3,
            LCDControl::ObjectSize               => BitIndex::I2,
            LCDControl::ObjectEnable             => BitIndex::I1,
            LCDControl::BackgroundPriority       => BitIndex::I0,
        }
    }

    pub fn get(&self, byte: u8) -> bool {
        get_bit(byte, self.index())
    }

    pub fn set(&self, byte: u8, val: bool) -> u8 {
        set_bit(byte, val, self.index())
    }
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

pub struct PPU {
    prev_state: bool,
    cycles: Cycles,
    rgba_frame: [[u8; 4]; FRAME_BUFFER_LENGTH as usize],
}

enum TileNumber {
    Base(u16),
    Absolute(u8),
}

impl PPU {
    pub fn new() -> Self {
        Self {
            prev_state: false,
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

    pub fn do_cycles(&mut self, bus: &mut Bus, cycles: Cycles, frame_buffer: &mut [u8]) {
        let mut count = 0;
        while count < cycles.0 {
            self.cycle(bus, frame_buffer);
            count += 1;
        }
    }

    pub fn cycle(&mut self, bus: &mut Bus, frame_buffer: &mut [u8]) {
        if !PPU::get_lcd_control(bus, LCDControl::LCDEnable) {
            self.increment_cycles(Cycles(1));
            return;
        }

        if PPU::get_lcd_y(bus) < 144 {
            if self.cycles.0 == 0 {
                // Mode 2 OAM scan
                PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM), true);
                self.stat_interrupt(bus);
            } else if self.cycles.0 == 80 + 1 {
                // Mode 3 drawing pixel line. This could also last 289 cycles
                self.draw_line(bus, frame_buffer);
                PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD), true);
            } else if self.cycles.0 == 80 + 172 + 1 {
                // Mode 0 Horizontal blank. This could last 87 or 204 cycles depending on the mode 3
                PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank), true);
                self.stat_interrupt(bus);
            }
        } else if PPU::get_lcd_y(bus) == 144 && self.cycles.0 == 0 {
            // Mode 1 Vertical blank
            bus.set_interrupt_flag(Interrupt::VBlank, true);
            PPU::set_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank), true);
            self.stat_interrupt(bus);
        }

        self.increment_cycles(Cycles(1));

        // Horizontal scan completed
        if self.cycles.0 > 456 {
            self.reset_cycles();

            PPU::set_lcd_y(bus, PPU::get_lcd_y(bus).wrapping_add(1));
            // Frame completed
            if PPU::get_lcd_y(bus) > 153 {
                PPU::set_lcd_y(bus, 0);
            }
            self.check_lyc(bus);
        }
    }

    fn stat_interrupt(&mut self, bus: &mut Bus) {
        let state = self.prev_state;
        self.prev_state = (
            (PPU::get_lcd_status(bus, LCDStatus::Mode2OAMInterrupt)    && PPU::get_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM))) ||
            (PPU::get_lcd_status(bus, LCDStatus::Mode0HBlankInterrupt) && PPU::get_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank))) ||
            (PPU::get_lcd_status(bus, LCDStatus::Mode1VBlankInterrupt) && PPU::get_lcd_status(bus, LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank)))
        );
        if self.prev_state && !state {
            bus.set_interrupt_flag(Interrupt::LCDSTAT, true);
        }
    }

    fn check_lyc(&mut self, bus: &mut Bus) {
        let lyc_compare = PPU::get_lcd_y(bus) == bus.read(LCD_Y_COMPARE_ADDRESS);
        PPU::set_lcd_status(bus, LCDStatus::LYCFlag, lyc_compare);
        if lyc_compare && PPU::get_lcd_status(bus, LCDStatus::LYCInterrupt) {
            bus.set_interrupt_flag(Interrupt::LCDSTAT, true);
            self.prev_state = true;
        }
    }

    fn get_lcd_y(bus: &Bus) -> u8 {
        bus.read(LCD_Y_ADDRESS)
    }

    fn set_lcd_y(bus: &mut Bus, val: u8) {
        bus.force_write(LCD_Y_ADDRESS, val);
    }

    fn get_scroll_x(bus: &Bus) -> u8 {
        bus.read(SCROLL_X_ADDRESS)
    }

    fn get_scroll_y(bus: &Bus) -> u8 {
        bus.read(SCROLL_Y_ADDRESS)
    }

    fn get_window_x(bus: &Bus) -> u8 {
        bus.read(WINDOW_X_ADDRESS)
    }

    fn get_window_y(bus: &Bus) -> u8 {
        bus.read(WINDOW_Y_ADDRESS)
    }

    pub fn get_lcd_control(bus: &Bus, control: LCDControl) -> bool {
        let byte = bus.read(LCD_CONTROL_ADDRESS);
        control.get(byte)
    }

    fn set_lcd_control(bus: &mut Bus, control: LCDControl, val: bool) {
        let mut byte = bus.read(LCD_CONTROL_ADDRESS);
        bus.force_write(LCD_CONTROL_ADDRESS, control.set(byte, val));
    }

    pub fn get_lcd_status(bus: &Bus, status: LCDStatus) -> bool {
        let byte = bus.read(LCD_STATUS_ADDRESS);
        match status {
            LCDStatus::LYCInterrupt         => get_bit(byte, BitIndex::I6),
            LCDStatus::Mode2OAMInterrupt    => get_bit(byte, BitIndex::I5),
            LCDStatus::Mode1VBlankInterrupt => get_bit(byte, BitIndex::I4),
            LCDStatus::Mode0HBlankInterrupt => get_bit(byte, BitIndex::I3),
            LCDStatus::LYCFlag              => get_bit(byte, BitIndex::I2),
            LCDStatus::ModeFlag(mode) => match mode {
                LCDStatusModeFlag::HBlank            => (byte & 0b00000011) == 0,
                LCDStatusModeFlag::VBlank            => (byte & 0b00000011) == 1,
                LCDStatusModeFlag::SearchingOAM      => (byte & 0b00000011) == 2,
                LCDStatusModeFlag::TransferringToLCD => (byte & 0b00000011) == 3,
            },
        }
    }

    fn set_lcd_status(bus: &mut Bus, status: LCDStatus, val: bool) {
        let mut byte = bus.read(LCD_STATUS_ADDRESS);
        byte = match status {
            LCDStatus::LYCInterrupt         => set_bit(byte, val, BitIndex::I6),
            LCDStatus::Mode2OAMInterrupt    => set_bit(byte, val, BitIndex::I5),
            LCDStatus::Mode1VBlankInterrupt => set_bit(byte, val, BitIndex::I4),
            LCDStatus::Mode0HBlankInterrupt => set_bit(byte, val, BitIndex::I3),
            LCDStatus::LYCFlag              => set_bit(byte, val, BitIndex::I2),
            LCDStatus::ModeFlag(mode) => match mode {
                LCDStatusModeFlag::HBlank            => (byte & 0b11111100) | 0,
                LCDStatusModeFlag::VBlank            => (byte & 0b11111100) | 1,
                LCDStatusModeFlag::SearchingOAM      => (byte & 0b11111100) | 2,
                LCDStatusModeFlag::TransferringToLCD => (byte & 0b11111100) | 3,
            },
        };
        bus.force_write(LCD_STATUS_ADDRESS, byte);
    }

    fn get_tile_bytes(x: u8, y: u8, tile_number_type: TileNumber, default_method: bool, bus: &Bus) -> (u8, u8) {
        let index_x = x as u16 / 8;
        let index_y = (y as u16 / 8) * 32;
        let index = index_x + index_y;
        let tile_line = (y).rem_euclid(8) * 2;
        let tile_number =  match tile_number_type {
            TileNumber::Base(base) => bus.read(base + index as u16),
            TileNumber::Absolute(num) => bus.read(0x8000 + num as u16),
        } as u16;
        let addr = if default_method {
            0x8000 + tile_line as u16 + (tile_number * 16)
        } else {
            let tile_number = (tile_number as i8) as i16;
            let tile_line = tile_line as i16;
            let base = (0x9000 as u16) as i16;
            (base + tile_line + (tile_number * 16)) as u16
        };

        (bus.read(addr), bus.read(addr + 1))
    }

    fn get_window_pixel(lcd_x: u8, bus: &Bus) -> Option<Pixel> {
        let lcd_y = PPU::get_lcd_y(bus);
        let window_x = PPU::get_window_x(bus);
        let window_y = PPU::get_window_y(bus);

        if !PPU::get_lcd_control(bus, LCDControl::WindowEnable) || lcd_x < (window_x.saturating_sub(7)) || window_y != lcd_y {
            return None;
        }

        let x = lcd_x.wrapping_sub(window_x.saturating_sub(7));
        let y = lcd_y.wrapping_sub(window_y);

        let default_mode = PPU::get_lcd_control(bus, LCDControl::TileAddressMode);
        let tilemap_area = match PPU::get_lcd_control(bus, LCDControl::WindowTileMapAddress) {
            true  => 0x9C00,
            false => 0x9800,
        };
        let (tile_byte_1, tile_byte_2) = PPU::get_tile_bytes(x, y, TileNumber::Base(tilemap_area), default_mode, bus);

        let palette = bus.read(BACKGROUND_PALETTE_ADDRESS);
        let pixels = PPU::get_byte_pixels(tile_byte_1, tile_byte_2, palette);

        Some(pixels[(x as usize).rem_euclid(8)])
    }

    fn draw_line(&mut self, bus: &Bus, frame_buffer: &mut [u8]) {
        let palette = bus.read(BACKGROUND_PALETTE_ADDRESS);
        let lcd_y = PPU::get_lcd_y(bus);
        if lcd_y as u32 >= LCD_HEIGHT {
            return;
        }
        let mut lcd_x: u8 = 0;
        while (lcd_x as u32) < LCD_WIDTH {
            let y = lcd_y.wrapping_add(PPU::get_scroll_y(bus));
            let x = lcd_x.wrapping_add(PPU::get_scroll_x(bus));

            let default_mode = PPU::get_lcd_control(bus, LCDControl::TileAddressMode);
            let tilemap_area = match PPU::get_lcd_control(bus, LCDControl::BackgroundTileMapAddress) {
                true  => 0x9C00,
                false => 0x9800,
            };
            let (tile_byte_1, tile_byte_2) = PPU::get_tile_bytes(x, y, TileNumber::Base(tilemap_area), default_mode, bus);

            let bg_pixels = PPU::get_byte_pixels(tile_byte_1, tile_byte_2, palette);

            let idx = (lcd_x as usize + (lcd_y as usize * LCD_WIDTH as usize)) * 4;
            let pixel = bg_pixels[x.rem_euclid(8) as usize];
            let rgba = PPU::get_rgba(pixel);
            frame_buffer[idx]     = rgba[0];
            frame_buffer[idx + 1] = rgba[1];
            frame_buffer[idx + 2] = rgba[2];
            if let Some(window_pixel) = PPU::get_window_pixel(lcd_x, bus) {
                let rgba = PPU::get_rgba(pixel);
                frame_buffer[idx]     = rgba[0];
                frame_buffer[idx + 1] = rgba[1];
                frame_buffer[idx + 2] = rgba[2];
            }

            lcd_x += 1;

            /* for pixel in bg_pixels {
                let idx = (lcd_x as usize + (lcd_y as usize * LCD_WIDTH as usize)) * 4;
                let rgba = PPU::get_rgba(pixel);
                frame_buffer[idx]     = rgba[0];
                frame_buffer[idx + 1] = rgba[1];
                frame_buffer[idx + 2] = rgba[2];
                if let Some(window_pixel) = PPU::get_window_pixel(lcd_x, bus) {
                    let rgba = PPU::get_rgba(pixel);
                    frame_buffer[idx]     = rgba[0];
                    frame_buffer[idx + 1] = rgba[1];
                    frame_buffer[idx + 2] = rgba[2];
                }

                lcd_x += 1;
            } */
        }
    }

    fn get_palette(index: u8, palette_byte: u8) -> u8 {
        match index {
            0b00 => palette_byte & 0b11,
            0b01 => (palette_byte >> 2) & 0b11,
            0b10 => (palette_byte >> 4) & 0b11,
            0b11 => (palette_byte >> 6) & 0b11,
            _ => unreachable!(),
        }
    }

    fn get_pixel(two_bit_pixel: u8) -> Pixel {
        match two_bit_pixel {
            0b00 => Pixel::White,
            0b01 => Pixel::Light,
            0b10 => Pixel::Dark,
            0b11 => Pixel::Black,
            _ => unreachable!(),
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

    fn get_byte_pixels(byte1: u8, byte2: u8, palette: u8) -> [Pixel; 8] {
        [
            PPU::get_pixel(PPU::get_palette(((byte1 >> 7) & 0b01) | ((byte2 >> 6) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 6) & 0b01) | ((byte2 >> 5) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 5) & 0b01) | ((byte2 >> 4) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 4) & 0b01) | ((byte2 >> 3) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 3) & 0b01) | ((byte2 >> 2) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 2) & 0b01) | ((byte2 >> 1) & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette(((byte1 >> 1) & 0b01) | (byte2        & 0b10), palette)),
            PPU::get_pixel(PPU::get_palette((byte1        & 0b01) | ((byte2 << 1) & 0b10), palette)),
        ]
    }
}
