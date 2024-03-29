use crate::utils::{
    BitIndex,
    get_bit,
    set_bit,
    join_bytes,
};
use crate::bus::SPRITE_ATTRIBUTE_TABLE;
use crate::cpu::Cycles;
use crate::interrupts::{Interrupts, Interrupt};

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
pub const WINDOW_Y_ADDRESS: u16 = 0xFF4A;
pub const WINDOW_X_ADDRESS: u16 = 0xFF4B;
pub const VRAM_BANK_SELECT_ADDRESS: u16 = 0xFF4F;

pub const HDMA1_ADDRESS: u16 = 0xFF51;
pub const HDMA2_ADDRESS: u16 = 0xFF52;
pub const HDMA3_ADDRESS: u16 = 0xFF53;
pub const HDMA4_ADDRESS: u16 = 0xFF54;
pub const HDMA5_ADDRESS: u16 = 0xFF55;

pub const BCPS_BGPI_ADDRESS: u16 = 0xFF68;
pub const BCPD_BGPD_ADDRESS: u16 = 0xFF69;
pub const OCPS_OBPI_ADDRESS: u16 = 0xFF6A;
pub const OCPD_OBPD_ADDRESS: u16 = 0xFF6B;

pub const TILE_MAP_ADDRESS: u16 = 0x9800;

#[derive(Debug, Copy, Clone)]
enum Pixel {
    White,
    Light,
    Dark,
    Black,
}

#[derive(Debug, Copy, Clone)]
struct RGBA(u8, u8, u8, u8);
#[derive(Debug, Copy, Clone)]
struct ColorPalette {
    white: RGBA,
    light: RGBA,
    dark:  RGBA,
    black: RGBA,
}

impl ColorPalette {
    pub fn new_cgb(cram: &[u8], palette_number: u8) -> Self {
        let addr = (palette_number as usize) * 8;
        let white = join_bytes(cram[addr + 1], cram[addr]);
        let light = join_bytes(cram[addr + 3], cram[addr + 2]);
        let dark  = join_bytes(cram[addr + 5], cram[addr + 4]);
        let black = join_bytes(cram[addr + 7], cram[addr + 6]);
        Self {
            white: extract_rgb(white),
            light: extract_rgb(light),
            dark:  extract_rgb(dark),
            black: extract_rgb(black),
        }
    }
}

fn extract_rgb(color: u16) -> RGBA {
    let red   = (color         & 0b11111).to_be_bytes()[1];
    let green = ((color >> 5)  & 0b11111).to_be_bytes()[1];
    let blue  = ((color >> 10) & 0b11111).to_be_bytes()[1];
    RGBA((red << 3) | (red >> 2), (green << 3) | (green >> 2), (blue << 3) | (blue >> 2), 0)
}

const BACKGROUND_COLORS: ColorPalette = ColorPalette {
    white: RGBA(0x83, 0xE6, 0xCD, 0),
    light: RGBA(0x66, 0xAD, 0xC6, 0),
    dark:  RGBA(0x4F, 0x53, 0xAB, 0),
    black: RGBA(0x3E, 0x24, 0x69, 0),
};

const WINDOW_COLORS: ColorPalette = ColorPalette {
    white: RGBA(0x83, 0xE6, 0xCD, 0),
    light: RGBA(0x66, 0xAD, 0xC6, 0),
    dark:  RGBA(0x4F, 0x53, 0xAB, 0),
    black: RGBA(0x3E, 0x24, 0x69, 0),
};

const SPRITE_0_COLORS: ColorPalette = ColorPalette {
    white: RGBA(0x83, 0xE6, 0xCD, 0),
    light: RGBA(0x66, 0xAD, 0xC6, 0),
    dark:  RGBA(0x4F, 0x53, 0xAB, 0),
    black: RGBA(0x3E, 0x24, 0x69, 0),
};

const SPRITE_1_COLORS: ColorPalette = ColorPalette {
    white: RGBA(0x83, 0xE6, 0xCD, 0),
    light: RGBA(0x66, 0xAD, 0xC6, 0),
    dark:  RGBA(0x4F, 0x53, 0xAB, 0),
    black: RGBA(0x3E, 0x24, 0x69, 0),
};

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

struct BgAttributes {
    bg_to_oam_priority: bool,
    vertical_flip: bool,
    horizontal_flip: bool,
    vram_bank: u8,
    palette_number: u8,
}

impl BgAttributes {
    pub fn new(byte: u8) -> Self {
        Self {
            bg_to_oam_priority: get_bit(byte, BitIndex::I7),
            vertical_flip: get_bit(byte, BitIndex::I6),
            horizontal_flip: get_bit(byte, BitIndex::I5),
            vram_bank: get_bit(byte, BitIndex::I3) as u8,
            palette_number: byte & 0b111,
        }
    }
}

struct Sprite {
    x: u8,
    y: u8,
    tile_number: u8,
    palette: u8,
    palette_zero: bool,
    x_flip: bool,
    y_flip: bool,
    over_bg: bool,
    bit_pixels: Option<[u8; 8]>,
    vram_bank: u8,
    palette_number: u8,
}

impl Sprite {
    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn get_pixel(&mut self, lcd_x: u8, lcd_y: u8, vram: &[u8], last_bg_index: u8, last_bg_priority: bool, lcd_control: u8, is_cgb: bool) -> Option<(Pixel, bool, u8)> {
        if !LCDControl::ObjectEnable.get(lcd_control) {
            return None;
        }

        if lcd_x < self.x.saturating_sub(8) || lcd_x >= self.x {
            return None;
        }

        if is_cgb && LCDControl::BackgroundPriority.get(lcd_control) && last_bg_priority {
            return None;
        }

        if self.over_bg && (last_bg_index & 0b11) != 0 {
            return None;
        }

        let is_long = LCDControl::ObjectSize.get(lcd_control);

        let height: u8 = match is_long {
            true => 16,
            false => 8,
        };
        let x = (lcd_x + 8) - self.x;
        let y = (lcd_y + 16) - self.y;
        let x = match self.x_flip {
            true => 7 - x,
            false => x,
        };
        let y = match self.y_flip {
            true => height - 1 - y,
            false => y,
        };

        let bit_pixel_index = (x as usize).rem_euclid(8);

        let bit_pixel = match self.bit_pixels {
            Some(bit_pixels_array) => {
                bit_pixels_array[bit_pixel_index]
            },
            None => {
                let mut tile_number = self.tile_number;

                if is_long && x <= 7 {
                    tile_number = tile_number & 0xFE;
                } else if is_long && x > 7 {
                    tile_number = tile_number | 0x01;
                }

                let tile_line = y.rem_euclid(height) * 2;
                let addr = 0x8000 + (tile_number as u16 * 16) + tile_line as u16;

                let vram_start = 0x8000;
                let tile_byte_1 = vram[(0x2000 * self.vram_bank as usize) + (addr - vram_start) as usize];
                let tile_byte_2 = vram[(0x2000 * self.vram_bank as usize) + (addr - vram_start + 1) as usize];
                let bit_pixels_array = PPU::get_byte_pixels(tile_byte_1, tile_byte_2);
                self.bit_pixels = Some(bit_pixels_array);

                bit_pixels_array[bit_pixel_index]
            }
        };

        if bit_pixel == 0 {
            return None;
        }

        if !is_cgb {
            return Some((PPU::get_pixel(PPU::get_palette(bit_pixel, self.palette)), self.palette_zero, 0));
        }
        Some((PPU::get_pixel(bit_pixel), self.palette_zero, self.palette_number))
    }
}

pub struct PPU {
    state: bool,
    background_priority: bool,
    window_enable: bool,
    lcd_enable: bool,
    window_drawn: bool,
    cycles: Cycles,
    sprite_buffer: Vec<Sprite>,
    window_y_counter: u8,
    last_bg_index: u8,
    last_bg_priority: bool,
    bg_palette: u8,
    lcd_control: u8,
    current_background_pixels: Option<([u8; 8], u8)>,
    current_window_pixels: Option<([u8; 8], u8)>,
    lcd_y: u8,
    lcd_x: u8,
    scroll_x: u8,
    scroll_y: u8,
    window_x: u8,
    window_y: u8,
    io_registers: [u8; 16],
    cram_registers: [u8; 4],
    vram: [u8; 0x2000 * 2],
    bg_cram: [u8; 64],
    obj_cram: [u8; 64],
    oam: [u8; 0xA0],
    vram_bank: u8,
    hdma_source: u16,
    hdma_destination: u16,
    hdma_start: u8,
    cgb_mode: bool,
}

impl PPU {
    pub fn new(cgb_mode: bool) -> Self {
        Self {
            state: false,
            background_priority: false,
            window_enable: false,
            window_drawn: false,
            lcd_enable: false,
            cycles: Cycles(0.0),
            sprite_buffer: Vec::new(),
            window_y_counter: 0,
            last_bg_index: 0,
            last_bg_priority: false,
            bg_palette: 0,
            lcd_control: 0,
            current_background_pixels: None,
            current_window_pixels: None,
            lcd_y: 0,
            lcd_x: 0,
            scroll_x: 0,
            scroll_y: 0,
            window_x: 0,
            window_y: 0,
            io_registers: [0; 16],
            cram_registers: [0; 4],
            vram: [0; 0x2000 * 2],
            bg_cram: [0; 64],
            obj_cram: [0; 64],
            oam: [0; 0xA0],
            vram_bank: 0,
            hdma_source: 0,
            hdma_destination: 0,
            hdma_start: 0,
            cgb_mode,
        }
    }

    pub fn lcd_y(&self) -> u8 {
        self.lcd_y
    }

    pub fn set_vram_bank(&mut self, bank: u8) {
        if self.cgb_mode {
            self.vram_bank = bank & 1;
        }
    }

    pub fn get_vram_bank(&self) -> u8 {
        self.vram_bank | 0xFE
    }

    pub fn hdma_source(&self) -> u16 {
        self.hdma_source
    }

    pub fn hdma_destination(&self) -> u16 {
        self.hdma_destination
    }

    pub fn is_io_register(address: u16) -> bool {
        (address >= 0xFF51 && address <= 0xFF55) ||
        (address >= 0xFF68 && address <= 0xFF6B) ||
        (address >= 0xFF40 && address <= 0xFF4F)
    }

    pub fn read_vram_external(&self, address: u16) -> u8 {
        self.vram[((address + (0x2000 * self.vram_bank as u16)) - 0x8000) as usize]
    }

    pub fn write_vram_external(&mut self, address: u16, data: u8) {
        self.vram[((address + (0x2000 * self.vram_bank as u16)) - 0x8000) as usize] = data;
    }

    fn read_vram(&self, address: u16) -> u8 {
        self.vram[(address - 0x8000) as usize]
    }

    pub fn read_oam(&self, address: u16) -> u8 {
        self.oam[(address - 0xFE00) as usize]
    }

    pub fn write_oam(&mut self, address: u16, data: u8) {
        self.oam[(address - 0xFE00) as usize] = data;
    }

    pub fn get_register(&self, address: u16) -> u8 {
        match address {
            HDMA1_ADDRESS..=HDMA5_ADDRESS => match address {
                HDMA1_ADDRESS => self.hdma_source.to_be_bytes()[0],
                HDMA2_ADDRESS => self.hdma_source.to_be_bytes()[1],
                HDMA3_ADDRESS => self.hdma_destination.to_be_bytes()[0],
                HDMA4_ADDRESS => self.hdma_destination.to_be_bytes()[1],
                HDMA5_ADDRESS => self.hdma_start,
                _ => 0x00,
            },
            0xFF68..=0xFF6B => self.cram_registers[(address as usize) - 0xFF68],
            VRAM_BANK_SELECT_ADDRESS => self.get_vram_bank(),
            LCD_CONTROL_ADDRESS => self.lcd_control,
            LCD_Y_ADDRESS => self.lcd_y,
            _ => self.io_registers[(address - 0xFF40) as usize],
        }
    }

    pub fn set_register(&mut self, address: u16, data: u8) {
        match address {
            HDMA1_ADDRESS..=HDMA5_ADDRESS => match address {
                HDMA1_ADDRESS => self.hdma_source = (self.hdma_source & 0xFF) | ((data as u16) << 8),
                HDMA2_ADDRESS => self.hdma_source = (self.hdma_source & 0xFF00) | (data as u16),
                HDMA3_ADDRESS => self.hdma_destination = (self.hdma_destination & 0xFF) | ((data as u16) << 8),
                HDMA4_ADDRESS => self.hdma_destination = (self.hdma_destination & 0xFF00) | (data as u16),
                HDMA5_ADDRESS => self.hdma_start = data,
                _ => (),
            },
            0xFF68..=0xFF6B => {
                self.cram_registers[(address as usize) - 0xFF68] = data;
                match address {
                    BCPD_BGPD_ADDRESS => {
                        if self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD)) {
                            return;
                        }
                        let byte = self.cram_registers[(BCPS_BGPI_ADDRESS as usize) - 0xFF68];
                        let auto_increment = get_bit(byte, BitIndex::I7);
                        let cram_address = byte & 0b111111;
                        self.bg_cram[cram_address as usize] = data;
                        if auto_increment {
                            self.cram_registers[(BCPS_BGPI_ADDRESS as usize) - 0xFF68] = ((byte + 1) & 0b111111) | ((auto_increment as u8) << 7);
                        }
                    },
                    OCPD_OBPD_ADDRESS => {
                        if self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD)) {
                            return;
                        }
                        let byte = self.cram_registers[(OCPS_OBPI_ADDRESS as usize) - 0xFF68];
                        let auto_increment = get_bit(byte, BitIndex::I7);
                        let cram_address = byte & 0b111111;
                        self.obj_cram[cram_address as usize] = data;
                        if auto_increment {
                            self.cram_registers[(OCPS_OBPI_ADDRESS as usize) - 0xFF68] = ((byte + 1) & 0b111111) | ((auto_increment as u8) << 7);
                        }
                    }
                    _ => {},
                }
            },
            VRAM_BANK_SELECT_ADDRESS => self.set_vram_bank(data),
            LCD_Y_ADDRESS => {},
            LCD_CONTROL_ADDRESS => {
                self.lcd_control = data;
                // Check if LCD is being turned on or off
                self.lcd_enable = get_bit(data, BitIndex::I7);
                if !get_bit(data, BitIndex::I7) || (get_bit(data, BitIndex::I7) && !get_bit(self.lcd_control, BitIndex::I7)) {
                    self.lcd_y = 0x00;
                    // Set Hblank
                    let byte = self.io_registers[LCD_STATUS_ADDRESS as usize - 0xFF40];
                    self.io_registers[LCD_STATUS_ADDRESS as usize - 0xFF40] = byte & 0b11111100;
                }
            },
            LCD_STATUS_ADDRESS => {
                let address = address - 0xFF40;
                let byte = self.io_registers[address as usize];
                self.io_registers[address as usize] = (data & 0b11111000) | (byte & 0b00000111);
            },
            _ => self.io_registers[address as usize - 0xFF40] = data,
        };
    }

    pub fn force_set_register(&mut self, address: u16, data: u8) {
        self.io_registers[address as usize - 0xFF40] = data;
    }

    pub fn reset_cycles(&mut self) {
        self.cycles.0 = 0.0;
    }

    pub fn increment_cycles(&mut self, cycles: Cycles) {
        self.cycles.0 += cycles.0;
    }

    pub fn do_cycles(&mut self, interrupts: &mut Interrupts, cycles: Cycles, frame_buffer: &mut [u8]) {
        if !self.lcd_enable {
            self.increment_cycles(cycles);
            return;
        }

        if self.lcd_y < 144 {
            if self.cycles.0 <= 80.0 && !self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM)) {
                // Mode 2 OAM scan
                self.set_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM), true);
                self.stat_interrupt(interrupts);
                self.oam_search();
            } else if self.cycles.0 > 80.0 && self.cycles.0 <= 80.0 + 172.0 {
                // Mode 3 drawing pixel line. This could also last 289 cycles
                if !self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD)) {
                    self.window_drawn = false;
                    self.set_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::TransferringToLCD), true);
                }
                self.draw_line(cycles, frame_buffer);
            } else if self.cycles.0 > 80.0 + 172.0 && self.cycles.0 <= 80.0 + 172.0 + 204.0 && !self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank)) {
                // Mode 0 Horizontal blank. This could last 87 or 204 cycles depending on the mode 3
                self.set_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank), true);
                self.stat_interrupt(interrupts);
            }
        } else if self.lcd_y >= 144 && !self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank)) {
            // Mode 1 Vertical blank
            self.set_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank), true);
            interrupts.request(Interrupt::VBlank);
            self.stat_interrupt(interrupts);
        }

        self.increment_cycles(cycles);

        // Horizontal scan completed
        if self.cycles.0 > 456.0 {
            self.reset_cycles();

            self.lcd_y = self.lcd_y.wrapping_add(1);
            self.lcd_x = 0;
            if self.window_drawn {
                self.window_y_counter = self.window_y_counter.saturating_add(1);
            }

            // Frame completed
            if self.lcd_y > 153 {
                self.window_y_counter = 0;
                self.lcd_y = 0;
            }
            self.stat_interrupt(interrupts);
        }
    }

    fn stat_interrupt(&mut self, interrupts: &mut Interrupts) {
        let prev_state = self.state;
        let lyc_compare = self.lcd_y == self.get_register(LCD_Y_COMPARE_ADDRESS);
        self.set_lcd_status(LCDStatus::LYCFlag, lyc_compare);
        self.state =
            (
                lyc_compare &&
                self.get_lcd_status(LCDStatus::LYCInterrupt)
            ) ||
            (
                self.get_lcd_status(LCDStatus::Mode2OAMInterrupt) &&
                self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::SearchingOAM))
            ) || (
                self.get_lcd_status(LCDStatus::Mode0HBlankInterrupt) &&
                self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::HBlank))
            ) || (
                self.get_lcd_status(LCDStatus::Mode1VBlankInterrupt) &&
                self.get_lcd_status(LCDStatus::ModeFlag(LCDStatusModeFlag::VBlank))
            );
        if self.state && !prev_state {
            interrupts.request(Interrupt::LCDSTAT);
        }
    }

    fn oam_search(&mut self) {
        if !self.get_lcd_control(LCDControl::ObjectEnable) {
            return;
        }
        self.sprite_buffer = Vec::new();
        let palette_0 = self.get_register(OBJECT_PALETTE_0_ADDRESS);
        let palette_1 = self.get_register(OBJECT_PALETTE_1_ADDRESS);
        let long_sprites = self.get_lcd_control(LCDControl::ObjectSize);
        let mut addr = SPRITE_ATTRIBUTE_TABLE.min().unwrap();
        while addr <= SPRITE_ATTRIBUTE_TABLE.max().unwrap() {
            if self.sprite_buffer.len() >= 10 {
                break;
            }
            let y = self.read_oam(addr);
            let x = self.read_oam(addr + 1);

            let sprite_height: u8 = match long_sprites {
                true => 16,
                false => 8,
            };

            if x == 0 {
                addr += 4;
                continue;
            } else if x >= 160 + 8 {
                addr += 4;
                continue;
            } else if y == 0 {
                addr += 4;
                continue;
            } else if y >= 144 + 16 {
                addr += 4;
                continue;
            } else if !long_sprites && y <= 8 {
                addr += 4;
                continue;
            }

            let lcd_y = self.lcd_y.saturating_add(16);

            if lcd_y < y || lcd_y > (y + sprite_height - 1) {
                addr += 4;
                continue;
            }


            let tile_number = self.read_oam(addr + 2);
            let attributes = self.read_oam(addr + 3);

            let palette_zero = !get_bit(attributes, BitIndex::I4);
            self.sprite_buffer.push(Sprite {
                x,
                y,
                tile_number,
                palette_zero,
                palette: match palette_zero {
                    true => palette_0,
                    false => palette_1,
                },
                x_flip: get_bit(attributes, BitIndex::I5),
                y_flip: get_bit(attributes, BitIndex::I6),
                over_bg: get_bit(attributes, BitIndex::I7),
                bit_pixels: None,
                vram_bank: match self.cgb_mode && get_bit(attributes, BitIndex::I3) {
                    true => 1,
                    false => 0,
                },
                palette_number: attributes & 0b111,
            });

            addr += 4;
        }
        if !self.cgb_mode {
            self.sprite_buffer.sort_by(|a, b| a.x().cmp(&b.x()));
        }
    }

    fn find_sprite_pixel(&mut self) -> Option<(Pixel, bool, u8)> {
        let lcd_y = self.lcd_y;
        for sprite in &mut self.sprite_buffer {
            if let Some(pixel) = sprite.get_pixel(self.lcd_x, lcd_y, &self.vram, self.last_bg_index, self.last_bg_priority, self.lcd_control, self.cgb_mode) {
                return Some(pixel);
            }
        }

        return None;
    }

    pub fn get_lcd_control(&mut self, control: LCDControl) -> bool {
        control.get(self.lcd_control)
    }

    pub fn get_lcd_status(&self, status: LCDStatus) -> bool {
        let byte = self.get_register(LCD_STATUS_ADDRESS);
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

    fn set_lcd_status(&mut self, status: LCDStatus, val: bool) {
        let mut byte = self.get_register(LCD_STATUS_ADDRESS);
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
        self.force_set_register(LCD_STATUS_ADDRESS, byte);
    }

    fn get_tile_bytes(&self, x: u8, y: u8, tilemap_area: u16, default_method: bool) -> (u8, u8, BgAttributes) {
        let index_x = x as u16 / 8;
        let index_y = (y as u16 / 8) * 32;
        let index = index_x + index_y;
        let tile_number = self.read_vram(tilemap_area + index as u16) as u16;
        let mut tile_line = (y).rem_euclid(8);
        let attributes = BgAttributes::new(self.read_vram(tilemap_area + 0x2000 + index as u16));
        if self.cgb_mode && attributes.vertical_flip {
            tile_line = 7 - tile_line;
        }
        tile_line = tile_line * 2;
        let addr = if default_method {
            0x8000 + tile_line as u16 + (tile_number * 16)
        } else {
            let tile_number = (tile_number as i8) as i16;
            let tile_line = tile_line as i16;
            let base = (0x9000 as u16) as i16;
            (base + tile_line + (tile_number * 16)) as u16
        };

        if !self.cgb_mode {
            return (self.read_vram(addr), self.read_vram(addr + 1), attributes);
        }
        let byte1 = self.read_vram(addr + (0x2000 * (attributes.vram_bank as u16)));
        let byte2 = self.read_vram(addr + (0x2000 * (attributes.vram_bank as u16)) + 1);
        if attributes.horizontal_flip {
            let byte1 = ((byte1 >> 7) & 0b1) |
                    ((byte1 >> 5) & 0b10) |
                    ((byte1 >> 3) & 0b100) |
                    ((byte1 >> 1) & 0b1000) |
                    ((byte1 << 1) & 0b10000) |
                    ((byte1 << 3) & 0b100000) |
                    ((byte1 << 5) & 0b1000000) |
                    ((byte1 << 7) & 0b10000000);
            let byte2 = ((byte2 >> 7) & 0b1) |
                    ((byte2 >> 5) & 0b10) |
                    ((byte2 >> 3) & 0b100) |
                    ((byte2 >> 1) & 0b1000) |
                    ((byte2 << 1) & 0b10000) |
                    ((byte2 << 3) & 0b100000) |
                    ((byte2 << 5) & 0b1000000) |
                    ((byte2 << 7) & 0b10000000);
            return (byte1, byte2, attributes);
        }
        return (byte1, byte2, attributes);
    }

    fn get_window_pixel(&mut self) -> Option<(Pixel, u8)> {
        if  !self.window_enable {
            self.last_bg_index = 0b00;
            return None;
        }
        if !self.cgb_mode {
            if !self.background_priority || !self.window_enable {
                self.last_bg_index = 0b00;
                return None;
            }
        }

        let lcd_y = self.lcd_y;
        let lcd_x = self.lcd_x;
        let window_x = self.window_x;
        let window_y = self.window_y;

        if
            lcd_y < window_y ||
            lcd_x < window_x.saturating_sub(7) ||
            window_y >= 144 ||
            window_x.saturating_sub(7) >= 160
        {
            return None;
        }

        let x = lcd_x.wrapping_sub(window_x.saturating_sub(7));
        let y = self.window_y_counter;

        let bit_pixel_index = (x as usize).rem_euclid(8);
        if bit_pixel_index == 0 {
            self.current_window_pixels = None;
        }

        let (bit_pixels_array, palette_number) = match self.current_window_pixels {
            Some(bit_pixels_array) => {
                bit_pixels_array
            },
            None => {
                let default_mode = self.get_lcd_control(LCDControl::TileAddressMode);
                let tilemap_area = match self.get_lcd_control(LCDControl::WindowTileMapAddress) {
                    true  => 0x9C00,
                    false => 0x9800,
                };
                let (tile_byte_1, tile_byte_2, info) = self.get_tile_bytes(x, y, tilemap_area, default_mode);
                let bit_pixels_array = PPU::get_byte_pixels(tile_byte_1, tile_byte_2);
                self.current_window_pixels = Some((bit_pixels_array, info.palette_number));
                self.last_bg_priority = info.bg_to_oam_priority;

                (bit_pixels_array, info.palette_number)
            },
        };
        let bit_pixel = bit_pixels_array[bit_pixel_index];
        self.last_bg_index = bit_pixel & 0b11;

        if !self.cgb_mode {
            return Some((PPU::get_pixel(PPU::get_palette(bit_pixel, self.bg_palette)), 0));
        }
        Some((PPU::get_pixel(bit_pixel), palette_number))
    }

    fn get_background_pixel(&mut self) -> Option<(Pixel, u8)> {
        if !self.cgb_mode && !self.background_priority {
            self.last_bg_index = 0b00;
            return None;
        }
        let lcd_y = self.lcd_y;
        let lcd_x = self.lcd_x;
        let y = lcd_y.wrapping_add(self.scroll_y);
        let x = lcd_x.wrapping_add(self.scroll_x);
        let bit_pixel_index = x.rem_euclid(8) as usize;

        if bit_pixel_index == 0 {
            self.current_background_pixels = None;
        }

        let (bit_pixels_array, palette_number) = match self.current_background_pixels {
            Some(bit_pixels_array) => {
                bit_pixels_array
            },
            None => {
                let default_mode = self.get_lcd_control(LCDControl::TileAddressMode);
                let tilemap_area = match self.get_lcd_control(LCDControl::BackgroundTileMapAddress) {
                    true  => 0x9C00,
                    false => 0x9800,
                };
                let (tile_byte_1, tile_byte_2, info) = self.get_tile_bytes(x, y, tilemap_area, default_mode);
                let bit_pixels_array = PPU::get_byte_pixels(tile_byte_1, tile_byte_2);
                self.current_background_pixels = Some((bit_pixels_array, info.palette_number));
                self.last_bg_priority = info.bg_to_oam_priority;

                (bit_pixels_array, info.palette_number)
            },
        };
        let bit_pixel = bit_pixels_array[bit_pixel_index];
        self.last_bg_index = bit_pixel & 0b11;

        if !self.cgb_mode {
            return Some((PPU::get_pixel(PPU::get_palette(bit_pixel, self.bg_palette)), 0));
        }
        Some((PPU::get_pixel(bit_pixel), palette_number))
    }

    fn draw_line(&mut self, cycles: Cycles, frame_buffer: &mut [u8]) {
        if self.lcd_y as u32 >= LCD_HEIGHT {
            return;
        }
        self.scroll_x = self.get_register(SCROLL_X_ADDRESS);
        self.scroll_y = self.get_register(SCROLL_Y_ADDRESS);
        self.window_x = self.get_register(WINDOW_X_ADDRESS);
        self.window_y = self.get_register(WINDOW_Y_ADDRESS);
        self.window_enable = self.get_lcd_control(LCDControl::WindowEnable);
        self.background_priority = self.get_lcd_control(LCDControl::BackgroundPriority);
        self.current_background_pixels = None;
        self.current_window_pixels = None;
        self.bg_palette = self.get_register(BACKGROUND_PALETTE_ADDRESS);
        let mut count = 0.0;
        while count < cycles.0 && (self.lcd_x as u32) < LCD_WIDTH {
            let idx = (self.lcd_x as usize + (self.lcd_y as usize * LCD_WIDTH as usize)) * 4;

            if let Some((window_pixel, palette_number)) = self.get_window_pixel() {
                self.window_drawn = true;
                let colors = match self.cgb_mode {
                    true => ColorPalette::new_cgb(&self.bg_cram, palette_number),
                    false => WINDOW_COLORS,
                };
                let rgba = PPU::get_rgba(window_pixel, colors);
                frame_buffer[idx]     = rgba[0];
                frame_buffer[idx + 1] = rgba[1];
                frame_buffer[idx + 2] = rgba[2];
            } else if let Some((background_pixel, palette_number)) = self.get_background_pixel() {
                let colors = match self.cgb_mode {
                    true => ColorPalette::new_cgb(&self.bg_cram, palette_number),
                    false => BACKGROUND_COLORS,
                };
                let rgba = PPU::get_rgba(background_pixel, colors);
                frame_buffer[idx]     = rgba[0];
                frame_buffer[idx + 1] = rgba[1];
                frame_buffer[idx + 2] = rgba[2];
            }
            if self.get_lcd_control(LCDControl::ObjectEnable) {
                if let Some((sprite_pixel, palette_zero, palette_number)) = self.find_sprite_pixel() {
                    let colors = match self.cgb_mode {
                        true => ColorPalette::new_cgb(&self.obj_cram, palette_number),
                        false => match palette_zero {
                            true => SPRITE_0_COLORS,
                            false => SPRITE_1_COLORS,
                        },
                    };
                    let rgba = PPU::get_rgba(sprite_pixel, colors);
                    frame_buffer[idx]     = rgba[0];
                    frame_buffer[idx + 1] = rgba[1];
                    frame_buffer[idx + 2] = rgba[2];
                }
            }

            self.lcd_x += 1;
            count += 1.0;
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

    fn get_rgba(pixel: Pixel, colors: ColorPalette) -> [u8; 4] {
        match pixel {
            Pixel::White => [colors.white.0, colors.white.1, colors.white.2, colors.white.3],
            Pixel::Light => [colors.light.0, colors.light.1, colors.light.2, colors.light.3],
            Pixel::Dark  => [ colors.dark.0,  colors.dark.1,  colors.dark.2,  colors.dark.3],
            Pixel::Black => [colors.black.0, colors.black.1, colors.black.2, colors.black.3],
        }
    }

    fn get_byte_pixels(byte1: u8, byte2: u8) -> [u8; 8] {
        [
            ((byte1 >> 7) & 0b01) | ((byte2 >> 6) & 0b10),
            ((byte1 >> 6) & 0b01) | ((byte2 >> 5) & 0b10),
            ((byte1 >> 5) & 0b01) | ((byte2 >> 4) & 0b10),
            ((byte1 >> 4) & 0b01) | ((byte2 >> 3) & 0b10),
            ((byte1 >> 3) & 0b01) | ((byte2 >> 2) & 0b10),
            ((byte1 >> 2) & 0b01) | ((byte2 >> 1) & 0b10),
            ((byte1 >> 1) & 0b01) | (byte2        & 0b10),
            (byte1        & 0b01) | ((byte2 << 1) & 0b10),
        ]
    }
}
