// use std::{thread, time};
use winit_input_helper::WinitInputHelper;
use winit::event::VirtualKeyCode;

use crate::cpu::{CPU, Cycles};
use crate::interrupts::Interrupt;
use crate::bus::Bus;
use crate::joypad::Button;
#[cfg(not(test))]
use crate::rom::{save_file};

pub struct Emulator {
    bus: Bus,
    cpu: CPU,
}

impl Emulator {
    pub fn new() -> Self {
        let bus = Bus::new();
        let cpu = match bus.cgb_mode {
            true => CPU::new_cgb(),
            false => CPU::new(),
        };
        Self {
            bus,
            cpu,
        }
    }

    pub fn close(&self) {
        println!("closing emulator");

        #[cfg(not(test))]
        match save_file(self.bus.rom.ram(), self.bus.rom.info()) {
            Err(err) => eprintln!("Could not save file: {}", err),
            _ => {},
        };
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper) {
        let mut change = false;
        if input.key_pressed(VirtualKeyCode::K) {
            change = true;
            self.bus.joypad.press(Button::A);
        }
        if input.key_pressed(VirtualKeyCode::J) {
            change = true;
            self.bus.joypad.press(Button::B);
        }
        if input.key_pressed(VirtualKeyCode::W) {
            change = true;
            self.bus.joypad.press(Button::Up);
        }
        if input.key_pressed(VirtualKeyCode::S) {
            change = true;
            self.bus.joypad.press(Button::Down);
        }
        if input.key_pressed(VirtualKeyCode::A) {
            change = true;
            self.bus.joypad.press(Button::Left);
        }
        if input.key_pressed(VirtualKeyCode::D) {
            change = true;
            self.bus.joypad.press(Button::Right);
        }
        if input.key_pressed(VirtualKeyCode::N) {
            change = true;
            self.bus.joypad.press(Button::Start);
        }
        if input.key_pressed(VirtualKeyCode::B) {
            change = true;
            self.bus.joypad.press(Button::Select);
        }

        if input.key_released(VirtualKeyCode::K) {
            change = true;
            self.bus.joypad.release(Button::A);
        }
        if input.key_released(VirtualKeyCode::J) {
            change = true;
            self.bus.joypad.release(Button::B);
        }
        if input.key_released(VirtualKeyCode::W) {
            change = true;
            self.bus.joypad.release(Button::Up);
        }
        if input.key_released(VirtualKeyCode::S) {
            change = true;
            self.bus.joypad.release(Button::Down);
        }
        if input.key_released(VirtualKeyCode::A) {
            change = true;
            self.bus.joypad.release(Button::Left);
        }
        if input.key_released(VirtualKeyCode::D) {
            change = true;
            self.bus.joypad.release(Button::Right);
        }
        if input.key_released(VirtualKeyCode::N) {
            change = true;
            self.bus.joypad.release(Button::Start);
        }
        if input.key_released(VirtualKeyCode::B) {
            change = true;
            self.bus.joypad.release(Button::Select);
        }
        if change {
            self.bus.interrupts.request(Interrupt::Joypad);
        }
    }

    pub fn run(&mut self, cpu_cycles: Cycles, frame_buffer: &mut [u8]) {
        self.cpu.reset_cycles();
        while self.cpu.get_cycles().to_t().0 <= cpu_cycles.0 {
            self.cpu.run(&mut self.bus);
            let cycles = self.cpu.get_last_op_cycles().to_t();
            self.bus.ppu.do_cycles(&mut self.bus.interrupts, cycles, frame_buffer);
            self.bus.sound.do_cycles(cycles);
            self.bus.timer.do_cycles(&mut self.bus.interrupts, cycles);
            if self.bus.double_speed_mode() {
                self.bus.timer.do_cycles(&mut self.bus.interrupts, Cycles(cycles.0 * 3.0));
            }

            // 1 CPU cycle = 238.42ns
            // thread::sleep(time::Duration::from_nanos((self.cpu.get_last_op_cycles().0 * 238).try_into().unwrap()));

        }
    }

    pub fn cpu_loop(&mut self) {
        let mut exit = false;
        let mut frame: [u8; 144 * 160 * 4] = [0; 144 * 160 * 4];
        while !exit {
            self.cpu.run(&mut self.bus);
            let cycles = self.cpu.get_last_op_cycles().to_t();
            self.bus.ppu.do_cycles(&mut self.bus.interrupts, cycles, &mut frame);
            self.bus.timer.do_cycles(&mut self.bus.interrupts, cycles);

            // exit = self.cpu.get_exec_calls_count() >= 1258895; // log 1
            exit = self.cpu.get_exec_calls_count() >= 161502; // log 2
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
