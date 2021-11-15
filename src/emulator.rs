// use std::{thread, time};
use std::rc::Rc;
use std::cell::RefCell;
use winit_input_helper::WinitInputHelper;
use winit::event::{VirtualKeyCode};

use crate::cpu::{CPU, Cycles, Interrupt};
use crate::ppu::PPU;
use crate::bus::Bus;
use crate::timer::Timer;
use crate::joypad::{Joypad, Button};

pub struct Emulator {
    bus: Bus,
    cpu: CPU,
    ppu: Rc<RefCell<PPU>>,
    timer: Rc<RefCell<Timer>>,
    joypad: Rc<RefCell<Joypad>>,
}

impl Emulator {
    pub fn new() -> Self {
        let joypad = Rc::new(RefCell::new(Joypad::new()));
        let timer  = Rc::new(RefCell::new(Timer::new()));
        let ppu    = Rc::new(RefCell::new(PPU::new()));
        Self {
            bus: Bus::new(Rc::clone(&ppu), Rc::clone(&joypad), Rc::clone(&timer)),
            cpu: CPU::new(),
            ppu,
            timer,
            joypad,
        }
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper) {
        let mut change = false;
        let mut joypad = self.joypad.borrow_mut();
        if input.key_pressed(VirtualKeyCode::K) {
            change = true;
            joypad.press(Button::A);
        }
        if input.key_pressed(VirtualKeyCode::J) {
            change = true;
            joypad.press(Button::B);
        }
        if input.key_pressed(VirtualKeyCode::W) {
            change = true;
            joypad.press(Button::Up);
        }
        if input.key_pressed(VirtualKeyCode::S) {
            change = true;
            joypad.press(Button::Down);
        }
        if input.key_pressed(VirtualKeyCode::A) {
            change = true;
            joypad.press(Button::Left);
        }
        if input.key_pressed(VirtualKeyCode::D) {
            change = true;
            joypad.press(Button::Right);
        }
        if input.key_pressed(VirtualKeyCode::N) {
            change = true;
            joypad.press(Button::Start);
        }
        if input.key_pressed(VirtualKeyCode::B) {
            change = true;
            joypad.press(Button::Select);
        }

        if input.key_released(VirtualKeyCode::K) {
            change = true;
            joypad.release(Button::A);
        }
        if input.key_released(VirtualKeyCode::J) {
            change = true;
            joypad.release(Button::B);
        }
        if input.key_released(VirtualKeyCode::W) {
            change = true;
            joypad.release(Button::Up);
        }
        if input.key_released(VirtualKeyCode::S) {
            change = true;
            joypad.release(Button::Down);
        }
        if input.key_released(VirtualKeyCode::A) {
            change = true;
            joypad.release(Button::Left);
        }
        if input.key_released(VirtualKeyCode::D) {
            change = true;
            joypad.release(Button::Right);
        }
        if input.key_released(VirtualKeyCode::N) {
            change = true;
            joypad.release(Button::Start);
        }
        if input.key_released(VirtualKeyCode::B) {
            change = true;
            joypad.release(Button::Select);
        }
        if change {
            self.bus.set_interrupt_flag(Interrupt::Joypad, true);
        }
    }

    pub fn run(&mut self, cpu_cycles: Cycles, frame_buffer: &mut [u8]) {
        self.cpu.reset_cycles();
        let mut ppu = self.ppu.borrow_mut();
        let mut timer = self.timer.borrow_mut();
        while self.cpu.get_cycles().to_t().0 <= cpu_cycles.0 {
            self.cpu.run(&mut self.bus);
            let cycles = self.cpu.get_last_op_cycles().to_t();
            ppu.do_cycles(&mut self.bus, cycles, frame_buffer);
            timer.do_cycles(&mut self.bus, cycles);

            // 1 CPU cycle = 238.42ns
            // thread::sleep(time::Duration::from_nanos((self.cpu.get_last_op_cycles().0 * 238).try_into().unwrap()));

        }
    }

    pub fn cpu_loop(&mut self) {
        let mut exit = false;
        let mut frame: [u8; 144 * 160 * 4] = [0; 144 * 160 * 4];
        while !exit {
            self.cpu.run(&mut self.bus);
            self.ppu.borrow_mut().do_cycles(&mut self.bus, self.cpu.get_last_op_cycles().to_t(), &mut frame);
            self.timer.borrow_mut().do_cycles(&mut self.bus, self.cpu.get_last_op_cycles().to_t());

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
