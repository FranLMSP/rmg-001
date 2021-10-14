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
        self.cpu.run(&mut self.bus);
    }
}
