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
//opcode timings IN T_CYCLES
static OPCODE_TIMINGS: [usize; 256] = [
//  0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F 
    0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x0
    0, 12, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x1
    8, 12, 8, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, //0x2
    8, 12, 0, 0, 0, 0, 12, 0, 8, 0, 0, 0, 0, 0, 0, 0, //0x3
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x4
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x5
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x6
    8, 8, 8, 8, 8, 8, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, //0x7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x8
    0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4, //0x9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xA
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xB
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xC
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xD
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, //0xE
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xF
];

#[rustfmt::skip]
//opcode timings IN T_CYCLES
static CB_OPCODE_TIMINGS: [usize; 256] = [
//  0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F 
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x1
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x2
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x3
    4, 4, 4, 4, 4, 4, 12, 4, 4, 4, 4, 4, 4, 4, 12, 4, //0x4
    4, 4, 4, 4, 4, 4, 12, 4, 4, 4, 4, 4, 4, 4, 12, 4, //0x5
    4, 4, 4, 4, 4, 4, 12, 4, 4, 4, 4, 4, 4, 4, 12, 4, //0x6
    4, 4, 4, 4, 4, 4, 12, 4, 4, 4, 4, 4, 4, 4, 12, 4, //0x7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0x9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xA
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xB
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xC
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //0xD
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, //0xE
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
    mem_tx: Sender<(usize, Vec<u8>)>,
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
    vram: [u8; 8192],
    wram: [u8; 8192],
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
        mem_tx: Sender<(usize, Vec<u8>)>,
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
                mem_tx,
            },
            cpu,
            cart,
            io,
            boot_rom,
            vram: [0; 8192],
            wram: [0; 8192],
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
    IllegalRead(usize),
    IllegalWrite(usize),
}

impl std::error::Error for ExecutionError {}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::UnimplmentedOpcode(o) => {
                write!(f, "[Execution error]: unimplemented opcode: {:#02X}", o)
            }
            ExecutionError::IllegalRead(a) => {
                write!(f, "[Execution error]: Illegal read at address: {:#04X}", a)
            }
            ExecutionError::IllegalWrite(a) => {
                write!(f, "[Execution error]: Illegal write at address: {:#04X}", a)
            }
        }
    }
}

#[allow(non_snake_case)]
impl System {
    pub fn execute_op(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        match opcode {
            //0x31 => self.LDSP(opcode),
            0x01 | 0x11 | 0x21 | 0x31 => self.LD16imm(opcode),
            0x70..=0x75 | 0x77 | 0x22 | 0x32 | 0x36 => self.STRHL(opcode),
            0xA8..=0xAF => self.XOR(opcode),
            0x20 | 0x30 | 0x28 | 0x38 => self.JRCond(opcode),
            0xCB => {
                self.cpu.rf.PC += 1;
                let second_byte = self.read(self.cpu.rf.PC, 1)?[0];
                self.execute_CB_op(second_byte)
            }
            _ => {
                self.comms
                    .log_tx
                    .send(format!("crashing on unimplemented opcode: {:#02x}", opcode))
                    .unwrap();
                return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            }
        }
    }
    #[allow(non_snake_case)]
    pub fn execute_CB_op(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        match opcode {
            //BIT test operations
            0x40..=0x7F => self.BIT(opcode),
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

#[allow(non_snake_case)]
impl System {
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
                //let data = self.read(self.cpu.rf.HL_read(), 1).unwrap()[0];
                let data = self.read(self.cpu.rf.HL_read(), 1)?[0];

                ("XOR (HL)".to_owned(), self.cpu.rf.A ^ data)
            } //return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            //A
            0xAF => ("XOR A".to_owned(), self.cpu.rf.A ^ self.cpu.rf.A),
            //imm
            0xEE => {
                let data = self.read(self.cpu.rf.PC, 1)?[0];
                //dont forget to step another byte since we are using an immeadiate
                self.cpu.rf.PC += 1;
                (format!("XOR d8 [{:x}]", data), self.cpu.rf.A ^ data)
            }
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

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn LD16imm(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the opcode we fetched
        self.cpu.rf.PC += 1;

        let data = self.read(self.cpu.rf.PC, 2)?;
        //let data: u16 = ((data[0] as u16) << 8) | data[1] as u16;
        let data: u16 = (data[0] as u16) | ((data[1] as u16) << 8);

        let log = match opcode {
            //BC
            0x01 => {
                self.cpu.rf.BC_write(data);
                format!("LD BC, {:#04x}", data)
            }
            //DE
            0x11 => {
                self.cpu.rf.DE_write(data);
                format!("LD DE, {:#04x}", data)
            }
            //HL
            0x21 => {
                self.cpu.rf.HL_write(data);
                format!("LD HL, {:#04x}", data)
            }
            //SP
            0x31 => {
                self.cpu.rf.SP = data;
                format!("LD SP, {:#04x}", data)
            }

            _ => unreachable!("panicking in LD16imm on an unreachable opcode"),
        };
        //make sure we step past the data we just read
        self.cpu.rf.PC += 2;

        self.comms.log_tx.send(log).unwrap();
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn STRHL(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the fetched opcode
        self.cpu.rf.PC += 1;

        let log = match opcode {
            //B
            0x70 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.B])?;
                format!("LD (HL), B")
            }
            //C
            0x71 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.C])?;
                format!("LD (HL), C")
            }
            //D
            0x72 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.D])?;
                format!("LD (HL), D")
            }
            //E
            0x73 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.E])?;
                format!("LD (HL), E")
            }
            //H
            0x74 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.H])?;
                format!("LD (HL), H")
            }
            //L
            0x75 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.L])?;
                format!("LD (HL), L")
            }
            //A
            0x77 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.A])?;
                format!("LD (HL), A")
            }
            //HL increment
            0x22 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.A])?;
                self.cpu.rf.HL_write(self.cpu.rf.HL_read() + 1);
                format!("LD (HL+), A")
            }
            //HL decrement
            0x32 => {
                self.write(self.cpu.rf.HL_read(), &[self.cpu.rf.A])?;
                self.cpu.rf.HL_write(self.cpu.rf.HL_read() - 1);
                format!("LD (HL-), A")
            }
            //immeadiate
            0x36 => {
                let data = self.read(self.cpu.rf.PC, 1)?[0];
                self.cpu.rf.PC += 1;
                self.write(self.cpu.rf.HL_read(), &[data])?;
                format!("LD (HL), imm8")
            }
            _ => {
                unreachable!("panicking in STRHL on an unreachable opcode")
            }
        };
        self.comms.log_tx.send(log).unwrap();

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn BIT(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the op we just fetched
        self.cpu.rf.PC += 1;

        //Shoutout Jarrett for noticing this neat encoding pattern

        let testing_bit = (opcode & 0b0011_1000) >> 3;
        //println!("testing bit {}", testing_bit);
        let bit_mask = 2_u32.pow((testing_bit) as u32);
        //println!("bitmask: {:b}", bit_mask);
        let reg = opcode & 0b0000_0111;
        let reg: crate::cpu::Register8 = (reg as usize).try_into().unwrap();
        /*println!(
            "register: {} with value {:x}  {:b}",
            reg, self.cpu.rf[reg], self.cpu.rf[reg]
        );*/

        let log = format!("BIT {},{}", testing_bit, reg);

        let result = (self.cpu.rf[reg] as u32 & bit_mask) >> testing_bit == 1;
        //println!("result says {}", result);

        self.cpu.rf.z_set(!result);
        self.cpu.rf.n_set(false);
        self.cpu.rf.h_set(true);
        //do not touch carry

        self.comms.log_tx.send(log).unwrap();

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn JRCond(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //jump past the op we just fetched
        self.cpu.rf.PC += 1;

        //possible offset is
        let offset: i8 = self.read(self.cpu.rf.PC, 1)?[0] as i8;
        //move past this byte we just fetched
        self.cpu.rf.PC += 1;

        /*println!(
            "flags are: Z:{} N:{} H:{} C:{}",
            self.cpu.rf.z_get(),
            self.cpu.rf.n_get(),
            self.cpu.rf.h_get(),
            self.cpu.rf.c_get()
        );*/

        let (log, cond) = match opcode {
            0x20 => ("JR NZ,i8", !self.cpu.rf.z_get()),
            0x30 => ("JR NC,i8", !self.cpu.rf.c_get()),
            0x28 => ("JR Z,i8", self.cpu.rf.z_get()),
            0x38 => ("JR C,i8", self.cpu.rf.c_get()),
            _ => {
                unreachable!("panicking in JRCond on an unreachable opcode")
            }
        };

        //optionally taking the jump
        if cond {
            //println!("taking the jump!");
            self.cpu.rf.PC = self.cpu.rf.PC.wrapping_add_signed(offset as i16);
        }

        self.comms.log_tx.send(log.to_string()).unwrap();

        return Ok(OPCODE_TIMINGS[opcode as usize] + if cond { 4 } else { 0 });
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
    fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
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
            0x8000..=0x9FFF => {
                //protect against reading off the end of vram
                if (address as usize + len) > 0xA000 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                Ok(self.vram[address as usize..=(address as usize + len)].to_vec())
            }
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

    fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        let res = match address {
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
            0x8000..=0x9FFF => {
                //protect against writing off the end of vram
                if (address as usize + data.len()) > 0xA000 {
                    self.comms
                        .log_tx
                        .send(format!(
                            "address: {:#04x}, data len: {}",
                            address,
                            data.len()
                        ))
                        .unwrap();
                    return Err(ExecutionError::IllegalWrite(address as usize));
                }

                let address = address - 0x8000;
                unsafe {
                    let src_ptr = data.as_ptr();
                    let dst_ptr = self.vram.as_mut_ptr().add(address as usize);
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, data.len());
                }

                Ok(data.len())
            }
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
        };

        //on a SUCCESSFUL write, update the frontend's memory
        self.comms
            .mem_tx
            .send((address as usize, data.to_vec()))
            .unwrap();
        res
    }
}
