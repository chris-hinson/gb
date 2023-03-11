use crate::{cart::Cart, cpu::Cpu, io::Io, FrontendCmd};
use rand::prelude::*;
use rand::rngs::ThreadRng;
use std::{
    io::Read,
    io::Write,
    sync::mpsc::{Receiver, Sender},
};

//this is just a convinience struct to bundle all of the comms data for backend->frontend comms and vice versa
//essentially anything that is not actually related to the system's operation
struct Comms {
    //(160x144)*3 = 69120
    screen_data: Vec<u8>,
    startup: std::time::Instant,
    log_tx: Sender<String>,
    screen_tx: Sender<Vec<u8>>,
    command_tx: Sender<FrontendCmd>,
    command_rx: Receiver<BackendCmd>,
    repaint_frontend_callback: Box<dyn Fn() + Send>,
}

#[derive(PartialEq)]
pub enum BackendCmd {
    Shutdown,
}

//this represents our entire GB system, both physical hardware units, as well as frontend communications abstractions
pub struct System {
    comms: Comms,
    cpu: Cpu,
    cart: Cart,
    io: Io,
    boot_rom: [u8; 0x100],
}

impl System {
    pub fn new(
        log_tx: Sender<String>,
        screen_tx: Sender<Vec<u8>>,
        command_tx: Sender<FrontendCmd>,
        command_rx: Receiver<BackendCmd>,
        repaint_frontend_callback: Box<dyn Fn() + Send>,
        cpu: Cpu,
        cart: Cart,
        io: Io,
        boot_rom: [u8; 0x100],
    ) -> Self {
        Self {
            comms: Comms {
                screen_data: vec![0; 69120],
                startup: std::time::Instant::now(),
                log_tx,
                screen_tx,
                command_tx,
                command_rx,
                repaint_frontend_callback,
            },
            cpu,
            cart,
            io,
            boot_rom,
        }
    }

    pub fn run(&mut self) {
        //let mut last_sent = u64::MAX;
        //let mut rand = rand::thread_rng();

        //1 loop iter = 1 M? cycle
        'running: loop {
            //see if we have gotten any commands from the frontend, and process and parse them if so
            let recv_cmds = self.comms.command_rx.try_iter();
            for cmd in recv_cmds {
                if cmd == BackendCmd::Shutdown {
                    break 'running;
                }
            }

            //fetch the opcode
            let op = self.read(self.cpu.rf.PC, 1).unwrap()[0];
            self.execute_op(op);

            //execute the opcode
        }
    }
}

impl System {
    pub fn execute_op(&mut self, opcode: u8) {
        match opcode {
            _ => unimplemented!("panicking on unimplemented opcode: {:x}", opcode),
        }
    }
}

//general memory_map
/*
0000	3FFF	16 KiB ROM bank 00	            From cartridge, usually a fixed bank
4000	7FFF	16 KiB ROM Bank 01~NN	        From cartridge, switchable bank via mapper (if any)
8000	9FFF	8 KiB Video RAM (VRAM)	        In CGB mode, switchable bank 0/1
A000	BFFF	8 KiB External RAM	            From cartridge, switchable bank if any
C000	CFFF	4 KiB Work RAM (WRAM)
D000	DFFF	4 KiB Work RAM (WRAM)	        In CGB mode, switchable bank 1~7
E000	FDFF	Mirror of C000~DDFF (ECHO RAM)	Nintendo says use of this area is prohibited.
FE00	FE9F	Sprite attribute table (OAM)
FEA0	FEFF	Not Usable	                    Nintendo says use of this area is prohibited
FF00	FF7F	I/O Registers
FF80	FFFE	High RAM (HRAM)
FFFF	FFFF	Interrupt Enable register (IE)	*/

impl System {
    //TOD: you're gonna need some nuance to support reading across different memory regions.
    //what happens if you read from 0x3FFF with len >1
    fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, std::io::Error> {
        match address {
            0x0000..=0x3FFF => {
                if self.io.bootrom_disable == 0 && address < 0x0100 {
                    //read from bootrom
                    Ok(self.boot_rom[address as usize..=address as usize + len].to_vec())
                } else {
                    //read from cart rom bank 0
                    self.cart.read(address, len)
                }
            }
            0x4000..=0x7FFF => unimplemented!("unimplemented read from cart rom bank 01~NN"),
            0x8000..=0x9FFF => unimplemented!("unimplemented read from VRAM"),
            0xA000..=0xBFFF => unimplemented!("unimplemented read from cart ram"),
            0xC000..=0xCFFF => unimplemented!("unimplemented read from WRAM bank 0"),
            0xD000..=0xDFFF => unimplemented!("unimplemented read from WRAM bank 1"),
            0xE000..=0xFDFF => unimplemented!("unimplemented read from ECHO RAM"),
            0xFE00..=0xFE9F => unimplemented!("unimplemented read from OAM"),
            0xFEA0..=0xFEFF => unimplemented!("unimplemented read from UNUSABLE AREA"),
            0xFF00..=0xFF7F => self.io.read(address, len),
            0xFF80..=0xFFFE => unimplemented!(
                "unimplemented read from HRAM (what the fuck is this even used for lol)"
            ),
            0xFFFF => unimplemented!("unimplemented read from IE reg"),
        }
    }

    fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, std::io::Error> {
        match address {
            0x0000..=0x3FFF => {
                if self.io.bootrom_disable == 0 && address < 0x0100 {
                    //write from bootrom?
                    //TODO: find out about legality of this. i dont think it should ever happen unless we write a
                    //self modifying bootrom?
                    panic!("tried to write to bootrom?")
                    //Ok(self.boot_rom[address as usize..=address as usize + len].to_vec())
                } else {
                    //read from cart rom bank 0
                    self.cart.write(address, data)
                }
            }
            0x4000..=0x7FFF => unimplemented!("unimplemented write to cart rom bank 01~NN"),
            0x8000..=0x9FFF => unimplemented!("unimplemented write to VRAM"),
            0xA000..=0xBFFF => unimplemented!("unimplemented write to cart ram"),
            0xC000..=0xCFFF => unimplemented!("unimplemented write to WRAM bank 0"),
            0xD000..=0xDFFF => unimplemented!("unimplemented write to WRAM bank 1"),
            0xE000..=0xFDFF => unimplemented!("unimplemented write to ECHO RAM"),
            0xFE00..=0xFE9F => unimplemented!("unimplemented write to OAM"),
            0xFEA0..=0xFEFF => unimplemented!("unimplemented write to UNUSABLE AREA"),
            0xFF00..=0xFF7F => self.io.write(address, data),
            0xFF80..=0xFFFE => unimplemented!(
                "unimplemented write to HRAM (what the fuck is this even used for lol)"
            ),
            0xFFFF => unimplemented!("unimplemented write to IE reg"),
        }
    }
}
