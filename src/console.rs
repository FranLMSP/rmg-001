use std::{thread, time};

use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::bus::Bus;

pub struct Console {
    cpu: CPU,
    ppu: PPU,
    bus: Bus,
}

impl Console {
    pub fn new() -> Self {
        Self {
            cpu: CPU::new(),
            ppu: PPU::new(),
            bus: Bus::new(),
        }
    }

    pub fn cpu_run(&mut self) {
        let mut exit = false;
        while !exit {
            self.cpu.run(&mut self.bus);

            // thread::sleep(time::Duration::from_millis(100));
            // exit = self.cpu.get_exec_calls_count() >= 1258895; // log 1
            // exit = self.cpu.get_exec_calls_count() >= 1068422; // log 3
            // exit = self.cpu.get_exec_calls_count() >= 1262766; // log 4
            // exit = self.cpu.get_exec_calls_count() >= 1763388; // log 5
            // exit = self.cpu.get_exec_calls_count() >= 1763388; // log 5
            // exit = self.cpu.get_exec_calls_count() >= 243272; // log 6
            // exit = self.cpu.get_exec_calls_count() >= 287416; // log 7
            // exit = self.cpu.get_exec_calls_count() >= 223892; // log 8
            // exit = self.cpu.get_exec_calls_count() >= 4420382; // log 9
            exit = self.cpu.get_exec_calls_count() >= 6714723; // log 10
        }
    }
}
