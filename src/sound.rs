use std::env;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use cpal::{Stream, StreamConfig, Device, Sample};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::cpu::Cycles;
use crate::utils::join_bytes;

pub const NR10_ADDRESS: u16 = 0xFF10;
pub const NR11_ADDRESS: u16 = 0xFF11;
pub const NR12_ADDRESS: u16 = 0xFF12;
pub const NR13_ADDRESS: u16 = 0xFF13;
pub const NR14_ADDRESS: u16 = 0xFF14;

pub const NR21_ADDRESS: u16 = 0xFF16;
pub const NR22_ADDRESS: u16 = 0xFF17;
pub const NR23_ADDRESS: u16 = 0xFF18;
pub const NR24_ADDRESS: u16 = 0xFF19;

pub const NR30_ADDRESS: u16 = 0xFF1A;
pub const NR31_ADDRESS: u16 = 0xFF1B;
pub const NR32_ADDRESS: u16 = 0xFF1C;
pub const NR33_ADDRESS: u16 = 0xFF1D;
pub const NR34_ADDRESS: u16 = 0xFF1E;

pub const NR41_ADDRESS: u16 = 0xFF20;
pub const NR42_ADDRESS: u16 = 0xFF21;
pub const NR43_ADDRESS: u16 = 0xFF22;
pub const NR44_ADDRESS: u16 = 0xFF23;

pub const NR50_ADDRESS: u16 = 0xFF24;
pub const NR51_ADDRESS: u16 = 0xFF25;
pub const NR52_ADDRESS: u16 = 0xFF26;

pub const WAVE_PATTERN_RAM: RangeInclusive<u16> = 0xFF30..=0xFF3F;

const WAVE_DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 1, 1],
    [0, 0, 0, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 0, 0],
];

struct ChannelTwo {
    #[allow(dead_code)]
    stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    frequency_timer: u16,
    duty_position: usize,
    sample_timer: usize,
    sample_rate: usize,
    buffer_pos: usize,
}

impl ChannelTwo {
    pub fn new(device: &Device, config: &StreamConfig, sample_rate: usize) -> Self {
        let mut count: usize = 0;
        // let mut count: f32 = 0.0;
        let buffer = Arc::new(Mutex::new(vec![0.0; sample_rate]));
        let buffer_clone = buffer.clone();
        let stream = device.build_output_stream(&config, move |data: &mut [f32], _| {
            /* for sample in data.iter_mut() {
                let y: f32 = ((count / (sample_rate as f32)) * 440.0 * 2.0 * 3.14159).sin().clamp(-1.0, 1.0);
                *sample = Sample::from(&y);
                count += 1.0;
                if count >= sample_rate as f32 {
                    count = 0.0;
                }
            } */
            let b = buffer_clone.lock().unwrap();
            for sample in data.iter_mut() {
                *sample = Sample::from(&b[count]);
                count += 1;
                if count >= b.len() {
                    count = 0;
                }
            }
        }, |err| eprintln!("An error occurred on the channel two: {}", err)).unwrap();
        stream.play().unwrap();

        Self {
            stream,
            frequency_timer: 0,
            duty_position: 0,
            sample_timer: 0,
            buffer_pos: 0,
            sample_rate,
            buffer,
        }
    }

    pub fn update_buffer(&mut self, duty_pattern: u8) {
        let sample = match WAVE_DUTY_PATTERNS[duty_pattern as usize][self.duty_position as usize] {
            0 => -1.0,
            1 => 1.0,
            _ => unreachable!(),
        };
        let clone = self.buffer.clone();
        let mut buffer = clone.lock().unwrap();
        buffer[self.buffer_pos] = sample;

        self.buffer_pos += 1;
        if self.buffer_pos >= buffer.len() {
            self.buffer_pos = 0;
        }
    }

    pub fn cycle(&mut self, duty: u8, frequency: u16) {
        self.frequency_timer = self.frequency_timer.saturating_sub(1);
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - frequency) * 4;
            self.duty_position += 1;
            if self.duty_position > 7 {
                self.duty_position = 0;
            }
        }
        self.sample_timer = self.sample_timer.saturating_add(self.sample_rate);
        if self.sample_timer >= 4194304 {
            self.update_buffer(duty);
            self.sample_timer = 0;
        }
    }
}

pub struct Sound {
    io_registers: [u8; 48],
    channel_two: Option<ChannelTwo>,
}


/* Sine Wave
let channel_two = device.build_output_stream(&config, move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
    for sample in data.iter_mut() {
        let y: f32 = ((sound.count / (sample_rate as f32)) * 440.0 * 2.0 * 3.14159).sin().clamp(-1.0, 1.0);
        *sample = Sample::from(&y);
        sound.count += 1.0;
    }
}, err_fn).unwrap();
channel_two.play().unwrap();
sound.channel_two = Some(channel_two); */

impl Sound {
    pub fn new() -> Self {
        if !env::var("SOUND_ENABLE").is_err() {
            let host = cpal::default_host();
            let device = host.default_output_device().expect("no output device available");
            let mut supported_configs_range = device.supported_output_configs()
                .expect("error while querying configs");
            let supported_config = supported_configs_range.next()
                .expect("no supported config?!")
                .with_max_sample_rate();
            // let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
            let sample_rate = supported_config.sample_rate().0;
            let config: StreamConfig = supported_config.into();
            return Self {
                io_registers: [0; 48],
                channel_two: Some(ChannelTwo::new(&device, &config, sample_rate as usize)),
            };
        }

        Self {
            io_registers: [0; 48],
            channel_two: None,
        }
    }

    pub fn channel_two_duty(&self) -> u8 {
        (self.get_register(NR21_ADDRESS) >> 6) & 0b11
    }

    pub fn channel_two_frequency(&self) -> u16 {
        join_bytes(self.get_register(NR24_ADDRESS), self.get_register(NR23_ADDRESS)) & 0x7F
    }

    pub fn is_io_register(address: u16) -> bool {
        address >= 0xFF10 && address <= 0xFF3F
    }

    pub fn get_register(&self, address: u16) -> u8 {
        self.io_registers[(address - 0xFF10) as usize]
    }

    pub fn set_register(&mut self, address: u16, data: u8) {
        self.io_registers[(address - 0xFF10) as usize] = data;
    }

    pub fn do_cycles(&mut self, cycles: Cycles) {
        let mut count = 0;
        while count < cycles.0 {
            self.cycle();
            count += 1;
        }
    }

    fn cycle(&mut self) {
        if self.channel_two.is_some() {
            let duty = self.channel_two_duty();
            let frequency = self.channel_two_frequency();
            self.channel_two.as_mut().unwrap().cycle(duty, frequency);
        }
    }
}
