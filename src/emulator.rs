use std::{thread, time};

use crate::cpu::{CPU, Cycles};
use crate::ppu::PPU;
use crate::bus::Bus;

pub struct Emulator {
    cpu: CPU,
    ppu: PPU,
    bus: Bus,
}

impl Emulator {
    pub fn new() -> Self {
        Self {
            cpu: CPU::new(),
            ppu: PPU::new(),
            bus: Bus::new(),
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
            self.ppu.do_cycles(&mut self.bus, self.cpu.get_last_op_cycles());
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
