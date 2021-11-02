use std::{thread, time};
use winit_input_helper::WinitInputHelper;
use winit::event::{VirtualKeyCode};

use crate::cpu::{CPU, Cycles, Interrupt};
use crate::ppu::PPU;
use crate::bus::Bus;
use crate::timer::Timer;
use crate::joypad::{Joypad, Button, JOYPAD_ADDRESS};

pub struct Emulator {
    cpu: CPU,
    ppu: PPU,
    bus: Bus,
    timer: Timer,
    joypad: Joypad,
}

impl Emulator {
    pub fn new() -> Self {
        let mut joypad: Joypad = Joypad::new();
        Self {
            cpu: CPU::new(),
            ppu: PPU::new(),
            bus: Bus::new(),
            timer: Timer::new(),
            joypad: Joypad::new(),
        }
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper) {
        let mut change = false;
        if input.key_pressed(VirtualKeyCode::K) {
            change = true;
            self.joypad.press(Button::A);
        }
        if input.key_pressed(VirtualKeyCode::J) {
            change = true;
            self.joypad.press(Button::B);
        }
        if input.key_pressed(VirtualKeyCode::W) {
            change = true;
            self.joypad.press(Button::Up);
        }
        if input.key_pressed(VirtualKeyCode::S) {
            change = true;
            self.joypad.press(Button::Down);
        }
        if input.key_pressed(VirtualKeyCode::A) {
            change = true;
            self.joypad.press(Button::Left);
        }
        if input.key_pressed(VirtualKeyCode::D) {
            change = true;
            self.joypad.press(Button::Right);
        }
        if input.key_pressed(VirtualKeyCode::N) {
            change = true;
            self.joypad.press(Button::Start);
        }
        if input.key_pressed(VirtualKeyCode::B) {
            change = true;
            self.joypad.press(Button::Select);
        }

        if input.key_released(VirtualKeyCode::K) {
            change = true;
            self.joypad.release(Button::A);
        }
        if input.key_released(VirtualKeyCode::J) {
            change = true;
            self.joypad.release(Button::B);
        }
        if input.key_released(VirtualKeyCode::W) {
            change = true;
            self.joypad.release(Button::Up);
        }
        if input.key_released(VirtualKeyCode::S) {
            change = true;
            self.joypad.release(Button::Down);
        }
        if input.key_released(VirtualKeyCode::A) {
            change = true;
            self.joypad.release(Button::Left);
        }
        if input.key_released(VirtualKeyCode::D) {
            change = true;
            self.joypad.release(Button::Right);
        }
        if input.key_released(VirtualKeyCode::N) {
            change = true;
            self.joypad.release(Button::Start);
        }
        if input.key_released(VirtualKeyCode::B) {
            change = true;
            self.joypad.release(Button::Select);
        }
        if change {
            self.joypad.update(&mut self.bus);
            self.bus.set_interrupt_flag(Interrupt::Joypad, true);
        }
    }

    pub fn draw(&mut self, frame: &mut [u8]) {
        let ppu_frame = self.ppu.get_rgba_frame();
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&ppu_frame[i]);
        }
    }

    pub fn run(&mut self, cpu_cycles: Cycles) {
        self.cpu.reset_cycles();
        while self.cpu.get_cycles().0 <= cpu_cycles.0 {
            self.cpu.run(&mut self.bus);
            if self.bus.reset_timer {
                self.bus.reset_timer = false;
                self.timer.reset(&mut self.bus);
            }
            self.ppu.do_cycles(&mut self.bus, self.cpu.get_last_op_cycles());
            self.timer.do_cycles(&mut self.bus, self.cpu.get_last_op_cycles());
        }
    }

    pub fn cpu_loop(&mut self) {
        let mut exit = false;
        while !exit {
            self.cpu.run(&mut self.bus);

            // exit = self.cpu.get_exec_calls_count() >= 1258895; // log 1
            // exit = self.cpu.get_exec_calls_count() >= 1068422; // log 3
            // exit = self.cpu.get_exec_calls_count() >= 1262766; // log 4
            // exit = self.cpu.get_exec_calls_count() >= 1763388; // log 5
            // exit = self.cpu.get_exec_calls_count() >= 1763388; // log 5
            // exit = self.cpu.get_exec_calls_count() >= 243272; // log 6
            // exit = self.cpu.get_exec_calls_count() >= 287416; // log 7
            // exit = self.cpu.get_exec_calls_count() >= 223892; // log 8
            // exit = self.cpu.get_exec_calls_count() >= 4420382; // log 9
            // exit = self.cpu.get_exec_calls_count() >= 6714723; // log 10
            // exit = self.cpu.get_exec_calls_count() >= 7429762; // log 11
        }
    }
}
