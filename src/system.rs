use crate::{cart::Cart, cpu::Cpu, io::Io, FrontendCmd};
use rand::prelude::*;
use rand::rngs::ThreadRng;
use std::{
    fmt::format,
    io::Read,
    io::Write,
    sync::mpsc::{Receiver, Sender},
};

#[rustfmt::skip]
static OPCODE_TIMINGS: [usize; 256] = [
//  0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F 
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x1
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x2
    0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x3
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x4
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x5
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x6
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xA
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xB
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xC
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xD
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xE
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xF
];

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
    cpu_tx: Sender<Cpu>,
    repaint_frontend_callback: Box<dyn Fn() + Send>,
}

#[derive(PartialEq)]
pub enum BackendCmd {
    Shutdown,
}

enum SystemState {
    Running,
    Crashed,
    BreakpointHit,
}

//this represents our entire GB system, both physical hardware units, as well as frontend communications abstractions
pub struct System {
    comms: Comms,
    cpu: Cpu,
    cart: Cart,
    io: Io,
    boot_rom: [u8; 0x100],
    M_cycles: usize,
    status: SystemState,
}

impl System {
    pub fn new(
        log_tx: Sender<String>,
        screen_tx: Sender<Vec<u8>>,
        command_tx: Sender<FrontendCmd>,
        command_rx: Receiver<BackendCmd>,
        cpu_tx: Sender<Cpu>,
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
                cpu_tx,
                repaint_frontend_callback,
            },
            cpu,
            cart,
            io,
            boot_rom,
            M_cycles: 0,
            status: SystemState::Running,
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

            //execute the opcode
            let execution = self.execute_op(op);
            if execution.is_err() {
                self.comms
                    .log_tx
                    .send(format!(
                        "emulation thread crashion on: {}",
                        execution.unwrap_err()
                    ))
                    .unwrap();
                break 'running;
            }

            self.comms.cpu_tx.send(self.cpu.clone()).unwrap();
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    UnimplmentedOpcode(usize),
}

impl std::error::Error for ExecutionError {}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::UnimplmentedOpcode(o) => {
                write!(f, "Error while executing opcode: {:#02X}", o)
            }
        }
    }
}

impl System {
    pub fn execute_op(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        match opcode {
            0x31 => self.LDSP(opcode),
            0xA8..=0xAF => self.XOR(opcode),
            _ => {
                self.comms
                    .log_tx
                    .send(format!("crashing on unimplemented opcode: {:#02x}", opcode))
                    .unwrap();
                return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            }
        }
    }
}

impl System {
    pub fn LDSP(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;
        let data = self.read(self.cpu.rf.PC, 2).unwrap();
        self.cpu.rf.PC += 2;
        let data: u16 = (data[1] as u16) << 8 | data[0] as u16;
        self.cpu.rf.SP = data;

        self.M_cycles += OPCODE_TIMINGS[opcode as usize] / 4;

        self.comms
            .log_tx
            .send(format!("LD SP,{:#04X}", data))
            .unwrap();
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn XOR(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the opcode we fetched
        self.cpu.rf.PC += 1;

        /*
        if opcode == 0xB8:
            result = A ^ B
            A = result
            flags.Z = 1 if result == 0 else 0
            flags.N = 0
            flags.H = 0
            flags.C = 0 */

        let (log, result) = match opcode {
            //B
            0xA8 => ("XOR B".to_owned(), self.cpu.rf.A ^ self.cpu.rf.B),
            //C
            0xA9 => ("XOR C".to_owned(), self.cpu.rf.A ^ self.cpu.rf.C),
            //D
            0xAA => ("XOR D".to_owned(), self.cpu.rf.A ^ self.cpu.rf.D),
            //E
            0xAB => ("XOR E".to_owned(), self.cpu.rf.A ^ self.cpu.rf.E),
            //H
            0xAC => ("XOR H".to_owned(), self.cpu.rf.A ^ self.cpu.rf.H),
            //L
            0xAD => ("XOR L".to_owned(), self.cpu.rf.A ^ self.cpu.rf.L),
            //(HL)
            //TODO: need to return an error if we read from a bad address
            0xAE => {
                let data = self.read(self.cpu.rf.HL_read(), 1).unwrap()[0];
                ("XOR (HL)".to_owned(), self.cpu.rf.A ^ data)
            } //return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            //A
            0xAF => ("XOR A".to_owned(), self.cpu.rf.A ^ self.cpu.rf.A),
            _ => unreachable!(
                "panicking in XOR becuase we somehow decided to execute an op that doesnt exist"
            ),
        };
        self.cpu.rf.A = result;
        self.comms.log_tx.send(log).unwrap();

        self.cpu.rf.z_set(self.cpu.rf.A == 0);
        self.cpu.rf.n_set(false);
        self.cpu.rf.h_set(false);
        self.cpu.rf.c_set(false);

        return if opcode == 0xAE { Ok(8) } else { Ok(4) };
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
