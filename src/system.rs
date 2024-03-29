use crate::cpu::Register16;
use crate::cpu::Register16::*;
use crate::cpu::Register8;
use crate::cpu::Register8::*;
use crate::{cart::Cart, cpu::Cpu, io::Io, FrontendCmd};
use rand::prelude::*;
use rand::rngs::ThreadRng;
use std::{
    fmt::format,
    io::Read,
    io::Write,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
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
    //mem_tx: Sender<(usize, Vec<u8>)>,
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
    pub vram: [u8; 8192],
    pub wram: [u8; 8192],
    pub hram: [u8; 126],
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
        //mem_tx: Sender<(usize, Vec<u8>)>,
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
                //mem_tx,
            },
            cpu,
            cart,
            io,
            boot_rom,
            vram: [0; 8192],
            wram: [0; 8192],
            hram: [0; 126],
            M_cycles: 0,
            status: SystemState::Running,
        }
    }

    /*pub fn run(&mut self) {
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
            //break execution loop on execution error and let the frontend know what went wrong
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
    }*/
}

pub fn run_mutex(system: Arc<Mutex<System>>) {
    'running: loop {
        let mut sys = system.lock().unwrap();
        //see if we have gotten any commands from the frontend, and process and parse them if so
        let recv_cmds = sys.comms.command_rx.try_iter();
        for cmd in recv_cmds {
            if cmd == BackendCmd::Shutdown {
                break 'running;
            }
        }

        debug!("PC: {:#04x}", sys.cpu.rf.PC);
        //fetch the opcode
        let pc = sys.cpu.rf.PC;
        let op = sys.read(pc, 1).unwrap()[0];

        //execute the opcode
        let execution = sys.execute_op(op);
        //break execution loop on execution error and let the frontend know what went wrong
        if execution.is_err() {
            sys.comms
                .log_tx
                .send(format!(
                    "emulation thread crashion on: {}",
                    execution.unwrap_err()
                ))
                .unwrap();
            break 'running;
        }

        sys.comms.cpu_tx.send(sys.cpu.clone()).unwrap();
        drop(sys);
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
            //0xA8..=0xAF => self.XOR(opcode),
            0x20 | 0x30 | 0x28 | 0x38 | 0x18 => self.JRCond(opcode),
            0x06 | 0x16 | 0x26 | 0x0E | 0x1E | 0x2E | 0x3E => self.LD8imm(opcode),
            0xE0 | 0xE2 => self.WriteIO(opcode),
            0xF0 | 0xF2 => self.ReadIO(opcode),
            0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1c | 0x2c | 0x3C => self.INC8(opcode),
            0x0A | 0x1A | 0x2A | 0x3A => self.LDA(opcode),
            //TODO decide where you actually want store hl ind to go lol
            0x02 | 0x12 | 0x22 | 0x32 => self.STRA(opcode),
            0xC4 | 0xD4 | 0xCC | 0xDC | 0xCD => self.CALL(opcode),
            0x40..=0x75 | 0x77..=0x7F => self.MV(opcode),
            0xC5 | 0xD5 | 0xE5 | 0xF5 => self.PUSH(opcode),
            0xC1 | 0xD1 | 0xE1 | 0xF1 => self.POP(opcode),
            0x07 => self.RLCA(opcode),
            0x17 => self.RLA(opcode),
            0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => self.DEC8(opcode),
            0x03 | 0x13 | 0x23 | 0x33 => self.INC16(opcode),
            0x0B | 0x1B | 0x2B | 0x3B => self.DEC16(opcode),
            0xC0 | 0xD0 | 0xC8 | 0xD8 | 0xC9 | 0xD9 => self.RET(opcode),
            0x80..=0x87 | 0xC6 => self.ADD(opcode),
            0x88..=0x8F | 0xCE => self.ADC(opcode),
            0x90..=0x97 | 0xD6 => self.SUB(opcode),
            0x98..=0x9F | 0xDE => self.SBC(opcode),
            0xA0..=0xA7 | 0xE6 => self.AND(opcode),
            0xA8..=0xAF | 0xEE => self.XOR(opcode),
            0xB0..=0xB7 | 0xF6 => self.OR(opcode),
            0xB8..=0xBF | 0xFE => self.CP(opcode),
            0xEA => self.STRA16(opcode),
            0xFA => self.LDA16(opcode),
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
                error!("crashing on unimplemented opcode: {:#02x}", opcode);
                return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            }
        }
    }
    #[allow(non_snake_case)]
    pub fn execute_CB_op(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        match opcode {
            0x00..=0x07 => self.RLC(opcode),
            0x08..=0x0F => self.RRC(opcode),
            0x10..=0x17 => self.RL(opcode),
            0x18..=0x1F => self.RR(opcode),
            0x20..=0x27 => self.SLA(opcode),
            0x28..=0x2F => self.SRA(opcode),
            0x30..=0x37 => self.SWAP(opcode),
            0x38..=0x3F => self.SRL(opcode),
            //BIT test operations
            0x40..=0x7F => self.BIT(opcode),
            0x80..=0xBf => self.RES(opcode),
            0xC0..=0xFF => self.SET(opcode),
            /*_ => {
                let log = format!("crashing on unimplemented opcode: 0xCB{:02x}", opcode);
                self.comms.log_tx.send(log.clone()).unwrap();
                error!("{log}");
                return Err(ExecutionError::UnimplmentedOpcode(opcode as usize));
            }*/
        }
    }
}

#[allow(non_snake_case)]
impl System {
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

        //self.comms.log_tx.send(log).unwrap();
        debug!("{log}");

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
        //self.comms.log_tx.send(log).unwrap();
        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
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
            0x18 => ("JR s8", true),
            _ => {
                unreachable!("panicking in JRCond on an unreachable opcode")
            }
        };

        //optionally taking the jump
        if cond {
            //println!("taking the jump!");
            self.cpu.rf.PC = self.cpu.rf.PC.wrapping_add_signed(offset as i16);
        }

        //self.comms.log_tx.send(log.to_string()).unwrap();
        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize] + if cond { 4 } else { 0 });
    }

    pub fn LD8imm(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        //figure out which register we are loading into
        let reg_mask = 0b0011_1000;
        let reg = (opcode & reg_mask) >> 3;
        let reg: crate::cpu::Register8 = (reg as usize).try_into().unwrap();

        //get the imm we are loading
        let imm8 = self.read(self.cpu.rf.PC, 1)?[0];
        self.cpu.rf.PC += 1;

        //actually do the load
        self.cpu.rf[reg] = imm8;

        let log = format!("LD {}, imm8", reg);
        debug!("{log}");
        //self.comms.log_tx.send(log).unwrap();

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn WriteIO(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let (log, mut address) = if opcode == 0xE0 {
            let imm = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            ("LD (FF00+u8),A", imm as usize)
        } else {
            ("LD (FF00+C),A", self.cpu.rf[C] as usize)
        };
        address += 0xFF00;

        self.write(address as u16, &[self.cpu.rf[A]])?;

        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn ReadIO(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let (address, log) = if opcode == 0xF0 {
            //imm
            let offset = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            (
                0xFF00 + offset as u16,
                format!("LD A, (FF00 + {:x})", offset),
            )
        } else {
            (
                0xFF00 + self.cpu.rf[C] as u16,
                "LD A, (FF00 + u8)".to_string(),
            )
        };

        self.cpu.rf[A] = self.read(address, 1)?[0];

        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn INC8(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: usize = ((opcode & reg_mask) >> 3) as usize;
        let (log, result, original) = if reg == 0b00110_0000 {
            //HL indirect
            let cur = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.write(self.cpu.rf.HL_read().wrapping_add(1), &[cur])?;
            (
                "INC (HL)".to_string(),
                self.read(self.cpu.rf.HL_read(), 1)?[0],
                cur,
            )
        } else {
            //normal reg increment
            let reg: Register8 = reg.try_into().unwrap();
            let cur = self.cpu.rf[reg];
            self.cpu.rf[reg] = cur.wrapping_add(1);
            (format!("INC {}", reg), self.cpu.rf[reg], cur)
        };

        //FLAGS!!!!!!!
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        //HC DEPENDANT ON WHAT???
        //TODO: I AM LITERALLY JUST GUESSING AT BEHAVIOR HERE WHAT IS IT SUPPOSED TO BE
        //note 3/14/23: gameboy has no instructions dependant on the HC flag so bad behavior might be fine
        self.cpu
            .rf
            .h_set((result & 0b0001_0000) != 0 && (original & 0b0000_1111) == 0b0000_1111);
        //carry untouched

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn DEC8(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: usize = ((opcode & reg_mask) >> 3) as usize;
        let (log, result, original) = if reg == 0b00110_0000 {
            //HL indirect
            let cur = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.write(self.cpu.rf.HL_read().wrapping_sub(1), &[cur])?;
            (
                "DEC (HL)".to_string(),
                self.read(self.cpu.rf.HL_read(), 1)?[0],
                cur,
            )
        } else {
            //normal reg increment
            let reg: Register8 = reg.try_into().unwrap();
            let cur = self.cpu.rf[reg];
            self.cpu.rf[reg] = cur.wrapping_sub(1);
            (format!("DEC {}", reg), self.cpu.rf[reg], cur)
        };

        //FLAGS!!!!!!!
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        //HC DEPENDANT ON WHAT???
        //TODO: I AM LITERALLY JUST GUESSING AT BEHAVIOR HERE WHAT IS IT SUPPOSED TO BE
        //note 3/14/23: gameboy has no instructions dependant on the HC flag so bad behavior might be fine
        self.cpu
            .rf
            .h_set((result & 0b0001_0000) != 0 && (original & 0b0000_1111) == 0b0000_1111);
        //carry untouched

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn INC16(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let log = match opcode {
            0x03 => {
                self.cpu.rf.BC_write(self.cpu.rf.BC_read().wrapping_add(1));
                "INC BC"
            }
            0x13 => {
                self.cpu.rf.DE_write(self.cpu.rf.DE_read().wrapping_add(1));
                "INC DE"
            }
            0x23 => {
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_add(1));
                "INC HL"
            }
            0x33 => {
                self.cpu.rf.SP = self.cpu.rf.SP.wrapping_add(1);
                "INC SP"
            }
            _ => unreachable!("crashing on an unreachable opcode in inc16"),
        };

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn DEC16(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let log = match opcode {
            0x0B => {
                self.cpu.rf.BC_write(self.cpu.rf.BC_read().wrapping_sub(1));
                "DEC BC"
            }
            0x1B => {
                self.cpu.rf.DE_write(self.cpu.rf.DE_read().wrapping_sub(1));
                "DEC DE"
            }
            0x2B => {
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_sub(1));
                "DEC HL"
            }
            0x3B => {
                self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                "DEC SP"
            }
            _ => unreachable!("crashing on an unreachable opcode in dec16"),
        };

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn LDA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let (log, value) = match opcode {
            0x0A => {
                //BC
                ("LD A, (BC)", self.read(self.cpu.rf.BC_read(), 1)?[0])
            }
            0x1A => {
                //DE
                ("LD A, (DE)", self.read(self.cpu.rf.DE_read(), 1)?[0])
            }
            0x2A => {
                //HL+
                let ret = ("LD A, (HL+)", self.read(self.cpu.rf.HL_read(), 1)?[0]);
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_add(1));
                ret
            }
            0x3a => {
                //HL-
                let ret = ("LD A, (HL-)", self.read(self.cpu.rf.HL_read(), 1)?[0]);
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_sub(1));
                ret
            }
            _ => unreachable!("panicking in LDA on unreachable opcode"),
        };

        self.cpu.rf.A = value;

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn STRA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let (log, address) = match opcode {
            0x02 => {
                //BC
                ("LD (BA), A", self.cpu.rf.BC_read())
            }
            0x12 => {
                //DE
                ("LD (DE), A", self.cpu.rf.DE_read())
            }
            0x22 => {
                //HL+
                let ret = ("LD (HL+), A", self.cpu.rf.HL_read());
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_add(1));
                ret
            }
            0x32 => {
                //HL-
                let ret = ("LD (HL-), A", self.cpu.rf.HL_read());
                self.cpu.rf.HL_write(self.cpu.rf.HL_read().wrapping_sub(1));
                ret
            }
            _ => unreachable!("panicking in LDA on unreachable opcode"),
        };

        self.write(address, &[self.cpu.rf[A]])?;

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn CALL(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let address_lower = self.read(self.cpu.rf.PC, 1)?[0];
        let address_higher = self.read(self.cpu.rf.PC + 1, 1)?[0];
        let address = ((address_higher as u16) << 8) | address_lower as u16;
        self.cpu.rf.PC += 2;

        //CD imm unconditional
        let log = match opcode {
            //NZ
            0xC4 => {
                //PC = address
                if !self.cpu.rf.c_get() {
                    //write	PC:upper->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[((self.cpu.rf.PC & 0xF0) >> 4) as u8])?;
                    //write	PC:lower->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[(self.cpu.rf.PC & 0x0F) as u8])?;
                    self.cpu.rf.PC = address;
                }
                "CALL NZ, u16".to_string()
            }
            //NC
            0xD4 => {
                //PC = address
                if !self.cpu.rf.c_get() {
                    //write	PC:upper->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[((self.cpu.rf.PC & 0xF0) >> 4) as u8])?;
                    //write	PC:lower->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[(self.cpu.rf.PC & 0x0F) as u8])?;
                    self.cpu.rf.PC = address;
                }
                "CALL NC, u16".to_string()
            }
            //Z
            0xCC => {
                //PC = address
                if self.cpu.rf.z_get() {
                    //write	PC:upper->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[((self.cpu.rf.PC & 0xF0) >> 4) as u8])?;
                    //write	PC:lower->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[(self.cpu.rf.PC & 0x0F) as u8])?;
                    self.cpu.rf.PC = address;
                }
                "CALL Z, u16".to_string()
            }
            //C
            0xDC => {
                //PC = address
                if self.cpu.rf.c_get() {
                    //write	PC:upper->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[((self.cpu.rf.PC & 0xF0) >> 4) as u8])?;
                    //write	PC:lower->(--SP)
                    self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                    self.write(self.cpu.rf.SP, &[(self.cpu.rf.PC & 0x0F) as u8])?;
                    self.cpu.rf.PC = address;
                }
                "CALL C, u16".to_string()
            }
            //uncond
            0xCD => {
                /*let address_lower = self.read(self.cpu.rf.PC, 1)?[0];
                let address_higher = self.read(self.cpu.rf.PC + 1, 1)?[0];
                let address: u16 = ((address_higher as u16) << 8) | (address_lower as u16);
                self.cpu.rf.PC += 2;*/

                debug!(
                    "high byte: {:#x}, low byte: {:#x}, full addr: {:#x}",
                    (address_higher as u16) << 8,
                    address_lower as u16,
                    address
                );

                //write	PC:upper->(--SP)
                self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                self.write(self.cpu.rf.SP, &[((self.cpu.rf.PC & 0xFF00) >> 8) as u8])?;
                //write	PC:lower->(--SP)
                self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
                self.write(self.cpu.rf.SP, &[(self.cpu.rf.PC & 0xFF) as u8])?;

                //PC = address
                self.cpu.rf.PC = address;
                format!("CALL u16 {:#x}", address)
            }
            _ => unreachable!("crashing in CALL on bad opcode"),
        };

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn MV(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let dst_mask = 0b0011_1000;
        let src_mask = 0b000_0111;
        let dst: Register8 = (((opcode & dst_mask) >> 3) as usize).try_into().unwrap();
        let src: Register8 = ((opcode & src_mask) as usize).try_into().unwrap();

        let log = format!("LD {}, {}", dst, src);

        if dst == HLInd {
            self.write(self.cpu.rf.HL_read(), &[self.cpu.rf[src]])?;
        } else if src == HLInd {
            self.cpu.rf[dst] = self.read(self.cpu.rf.HL_read(), 1)?[0];
        } else {
            self.cpu.rf[dst] = self.cpu.rf[src];
        }

        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn PUSH(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_pair: (Register8, Register8) = if opcode == 0xC5 {
            (B, C)
        } else if opcode == 0xD5 {
            (D, E)
        } else if opcode == 0xE5 {
            (H, L)
        } else
        //if opcode == 0xF5 {
        {
            (A, F)
        };

        self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
        self.write(self.cpu.rf.SP, &[self.cpu.rf[reg_pair.0]])?;
        self.cpu.rf.SP = self.cpu.rf.SP.wrapping_sub(1);
        self.write(self.cpu.rf.SP, &[self.cpu.rf[reg_pair.1]])?;

        let log = format!("PUSH {}{}", reg_pair.0, reg_pair.1);
        debug!("{log}");
        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn POP(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_pair: (Register8, Register8) = if opcode == 0xC5 {
            (B, C)
        } else if opcode == 0xD5 {
            (D, E)
        } else if opcode == 0xE5 {
            (H, L)
        } else
        //if opcode == 0xF5 {
        {
            (A, F)
        };

        self.cpu.rf[reg_pair.1] = self.read(self.cpu.rf.SP, 1)?[0];
        self.cpu.rf[reg_pair.0] = self.read(self.cpu.rf.SP, 1)?[0];
        self.cpu.rf.SP = self.cpu.rf.SP.wrapping_add(2);

        let log = format!("POP {}{}", reg_pair.1, reg_pair.0);
        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn RLC(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.cpu.rf.c_set((init_val & 0b1000_0000) != 0);
            let new_val = init_val.rotate_left(1);
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf.c_set((self.cpu.rf[reg] & 0b1000_0000) != 0);
            self.cpu.rf[reg] = self.cpu.rf[reg].rotate_left(1);
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("RLC {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn RRC(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.cpu.rf.c_set((init_val & 0b0000_0001) != 0);
            let new_val = init_val.rotate_right(1);
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf.c_set((self.cpu.rf[reg] & 0b0000_0001) != 0);
            self.cpu.rf[reg] = self.cpu.rf[reg].rotate_right(1);
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("RRC {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn RL(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let mut initial_value = self.read(self.cpu.rf.HL_read(), 1)?[0];
            initial_value <<= 1;
            initial_value |= if self.cpu.rf.c_get() { 0x1 } else { 0x0 };
            self.cpu.rf.z_set(initial_value == 0);
            self.write(self.cpu.rf.HL_read(), &[initial_value])?;
        } else {
            self.cpu.rf[reg] <<= 1;
            self.cpu.rf[reg] |= if self.cpu.rf.c_get() { 0x1 } else { 0x0 };
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("RL {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn RR(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let mut initial_value = self.read(self.cpu.rf.HL_read(), 1)?[0];
            initial_value >>= 1;
            initial_value |= if self.cpu.rf.c_get() {
                0b1000_0000
            } else {
                0x0
            };
            self.cpu.rf.z_set(initial_value == 0);
        } else {
            self.cpu.rf[reg] <<= 1;
            self.cpu.rf[reg] |= if self.cpu.rf.c_get() {
                0b1000_0000
            } else {
                0x0
            };
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("RR {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SLA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.cpu.rf.c_set((init_val & 0b1000_0000) != 0);
            let new_val = init_val << 1;
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf.c_set((self.cpu.rf[reg] & 0b1000_0000) != 0);
            self.cpu.rf[reg] <<= 0;
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("SLA {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SRA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.cpu.rf.c_set((init_val & 0b1000_0000) != 0);
            let new_val = (init_val as i8 >> 1) as u8;
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf.c_set((self.cpu.rf[reg] & 0b1000_0000) != 0);
            self.cpu.rf[reg] = ((self.cpu.rf[reg] as i8) >> 1) as u8;
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("SRA {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SWAP(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            let new_val = (init_val & 0x0F) << 4 | (init_val & 0xF0) >> 4;
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf[reg] = (self.cpu.rf[reg] & 0x0f) << 4 | (self.cpu.rf[reg] & 0xf0) >> 4;
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("SWAP {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SRL(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0000_0111;
        let reg: Register8 = ((opcode & reg_mask) as usize).try_into().unwrap();

        if reg == HLInd {
            //we are operating in memory
            let init_val = self.read(self.cpu.rf.HL_read(), 1)?[0];
            self.cpu.rf.c_set((init_val & 0b1000_0000) != 0);
            let new_val = init_val >> 1;
            self.cpu.rf.z_set(new_val == 0);
            self.write(self.cpu.rf.HL_read(), &[new_val])?;
        } else {
            self.cpu.rf.c_set((self.cpu.rf[reg] & 0b1000_0000) != 0);
            self.cpu.rf[reg] = self.cpu.rf[reg] >> 1;
            self.cpu.rf.z_set(self.cpu.rf[reg] == 0);
        }

        let log = format!("SRL {}", reg);
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn RES(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the op we just fetched
        self.cpu.rf.PC += 1;

        //Shoutout Jarrett for noticing this neat encoding pattern

        let testing_bit = (opcode & 0b0011_1000) >> 3;
        //println!("testing bit {}", testing_bit);
        let bit_mask = 2_u32.pow((testing_bit) as u32);
        //println!("bitmask: {:b}", bit_mask);
        let reg = opcode & 0b0000_0111;
        let reg: crate::cpu::Register8 = (reg as usize).try_into().unwrap();

        let log = format!("RES {},{}", testing_bit, reg);

        self.cpu.rf[reg] &= !(<u32 as TryInto<u8>>::try_into(bit_mask).unwrap());

        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SET(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the op we just fetched
        self.cpu.rf.PC += 1;

        //Shoutout Jarrett for noticing this neat encoding pattern

        let testing_bit = (opcode & 0b0011_1000) >> 3;
        //println!("testing bit {}", testing_bit);
        let bit_mask = 2_u32.pow((testing_bit) as u32);
        //println!("bitmask: {:b}", bit_mask);
        let reg = opcode & 0b0000_0111;
        let reg: crate::cpu::Register8 = (reg as usize).try_into().unwrap();

        let log = format!("RES {},{}", testing_bit, reg);

        self.cpu.rf[reg] |= <u32 as TryInto<u8>>::try_into(bit_mask).unwrap();

        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
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

        // self.comms.log_tx.send(log).unwrap();
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn RLCA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        self.cpu.rf.c_set((self.cpu.rf[A] & 0b1000_0000) != 0);
        self.cpu.rf[A] = self.cpu.rf[A].rotate_left(1);
        self.cpu.rf.z_set(false);
        self.cpu.rf.h_set(false);
        self.cpu.rf.n_set(false);

        let log = format!("RLCA");
        debug!("{log}");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn RLA(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        self.cpu.rf[A] <<= 1;
        self.cpu.rf[A] |= if self.cpu.rf.c_get() { 0x1 } else { 0x0 };
        self.cpu.rf.z_set(self.cpu.rf[A] == 0);

        let log = format!("RL A");
        debug!("{log}");

        return Ok(CB_OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn RET(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let possible_low = self.read(self.cpu.rf.SP, 1)?[0];
        let possible_high = self.read(self.cpu.rf.SP + 1, 1)?[0];
        let possible_addr = (possible_high as u16) << 8 | possible_low as u16;
        debug!(
            "possible_low: {:#02x}, possible high: {:#02x}, possible addr: {:#04x}",
            possible_low, possible_high, possible_addr
        );

        let (log, take) = if opcode == 0xC0 {
            ("RET NZ", !self.cpu.rf.z_get())
        } else if opcode == 0xD0 {
            ("RET NC", !self.cpu.rf.c_get())
        } else if opcode == 0xC8 {
            ("RET Z", self.cpu.rf.z_get())
        } else if opcode == 0xD8 {
            ("RET C", self.cpu.rf.c_get())
        } else if opcode == 0xC9 {
            //TODO: RETI stuff here
            ("RET", true)
        } else {
            ("RET", true)
        };

        if take {
            self.cpu.rf.PC = possible_addr;
            self.cpu.rf.SP = self.cpu.rf.SP.wrapping_add(2);
        }

        debug!("{log} {:#4x}", possible_addr);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn ADD(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0x6 {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let result = self.cpu.rf[A].wrapping_add(val);
        self.cpu.rf[A] = result;
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        //self.cpu.rf.h_set(val);
        //self.cpu.rf.c_set(self.cpu.rf.c_get());

        debug!("ADD A, {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn ADC(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0xE {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let (result, carry) = self.cpu.rf[A].carrying_add(val, self.cpu.rf.c_get());
        self.cpu.rf[A] = result;
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        //self.cpu.rf.h_set(val);
        self.cpu.rf.c_set(carry);

        debug!("ADC A, {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SUB(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0x6 {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let result = self.cpu.rf[A].wrapping_sub(val);
        self.cpu.rf[A] = result;
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(true);
        //self.cpu.rf.h_set(val);
        //self.cpu.rf.c_set(self.cpu.rf.c_get());

        debug!("SUB {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn SBC(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0xE {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let (result, carry) = self.cpu.rf[A].borrowing_sub(val, self.cpu.rf.c_get());
        self.cpu.rf[A] = result;
        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(true);
        //self.cpu.rf.h_set(val);
        self.cpu.rf.c_set(carry);

        debug!("SBC A, {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn AND(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0x6 {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let result = self.cpu.rf[A] & val;
        self.cpu.rf[A] = result;

        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        self.cpu.rf.h_set(true);
        self.cpu.rf.c_set(false);

        debug!("AND {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn XOR(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        //step past the opcode we fetched
        self.cpu.rf.PC += 1;

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
            0xAE => {
                let data = self.read(self.cpu.rf.HL_read(), 1)?[0];

                ("XOR (HL)".to_owned(), self.cpu.rf.A ^ data)
            }
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
        //self.comms.log_tx.send(log).unwrap();

        debug!("{log}");

        self.cpu.rf.z_set(self.cpu.rf.A == 0);
        self.cpu.rf.n_set(false);
        self.cpu.rf.h_set(false);
        self.cpu.rf.c_set(false);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn OR(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0x6 {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let result = self.cpu.rf[A] & val;
        self.cpu.rf[A] = result;

        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(false);
        self.cpu.rf.h_set(false);
        self.cpu.rf.c_set(false);

        debug!("OR {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn CP(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let reg_mask = 0b0011_1000;
        let reg: Register8 = (((opcode & reg_mask) >> 3) as usize).try_into().unwrap();

        let val = if reg == HLInd {
            self.read(self.cpu.rf.HL_read(), 1)?[0]
        } else if opcode & 0xF == 0xE {
            let v = self.read(self.cpu.rf.PC, 1)?[0];
            self.cpu.rf.PC += 1;
            v
        } else {
            self.cpu.rf[reg]
        };

        let result = self.cpu.rf[reg].wrapping_sub(val);

        self.cpu.rf.z_set(result == 0);
        self.cpu.rf.n_set(true);
        //self.cpu.rf.h_set(false);
        //self.cpu.rf.c_set(false);

        debug!("CP {}", reg);

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }

    pub fn STRA16(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let lower = self.read(self.cpu.rf.PC, 1)?[0];
        let upper = self.read(self.cpu.rf.PC + 1, 1)?[0];
        self.cpu.rf.PC += 2;
        let address = (upper as u16) << 8 | lower as u16;

        self.write(address, &[self.cpu.rf.A])?;

        debug!("LD (u16), A");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
    }
    pub fn LDA16(&mut self, opcode: u8) -> Result<usize, ExecutionError> {
        self.cpu.rf.PC += 1;

        let lower = self.read(self.cpu.rf.PC, 1)?[0];
        let upper = self.read(self.cpu.rf.PC + 1, 1)?[0];
        self.cpu.rf.PC += 2;
        let address = (upper as u16) << 8 | lower as u16;

        self.cpu.rf[A] = self.read(address, 1)?[0];

        debug!("LD A, (u16)");

        return Ok(OPCODE_TIMINGS[opcode as usize]);
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
    pub fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
        trace!(
            "receiving system read at address {:#04X} of length {:#04X}",
            address,
            len
        );
        match address {
            0x0000..=0x3FFF => {
                let mut return_vec: Vec<u8> = Vec::new();

                if self.io.bootrom_disable == 0 && address < 0x0100 {
                    let end_address = address as usize + len;
                    if end_address < 0x0100 {
                        //read from bootrom
                        Ok(self.boot_rom[address as usize..=address as usize + len].to_vec())
                    } else {
                        return_vec.append(&mut self.boot_rom[..].to_vec());
                        return_vec.append(&mut self.cart.read(0x0100, end_address)?);
                        Ok(return_vec)
                    }
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
                let address = address - 0x8000;
                Ok(self.vram[address as usize..(address as usize + len)].to_vec())
            }
            0xA000..=0xBFFF => unimplemented!("unimplemented read from cart ram"),
            0xC000..=0xCFFF => unimplemented!("unimplemented read from WRAM bank 0"),
            0xD000..=0xDFFF => unimplemented!("unimplemented read from WRAM bank 1"),
            0xE000..=0xFDFF => unimplemented!("unimplemented read from ECHO RAM"),
            0xFE00..=0xFE9F => unimplemented!("unimplemented read from OAM"),
            0xFEA0..=0xFEFF => unimplemented!("unimplemented read from UNUSABLE AREA"),
            0xFF00..=0xFF7F => self.io.read(address, len),
            0xFF80..=0xFFFE => {
                //unimplemented!("unimplemented read from HRAM (what the fuck is this even used for lol)")
                if (address as usize + len) > 0xFFFF {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                let address = address - 0xFF80;
                Ok(self.hram[address as usize..(address as usize + len)].to_vec())
            }
            0xFFFF => unimplemented!("unimplemented read from IE reg"),
        }
    }

    fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        trace!(
            "receiving system write at address {:#04X} of length {:#04X}",
            address,
            data.len()
        );

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
            0xFF80..=0xFFFE => {
                //unimplemented!("unimplemented write to HRAM (what the fuck is this even used for lol)")
                let address = address - 0xFF80;
                unsafe {
                    let src_ptr = data.as_ptr();
                    let dst_ptr = self.hram.as_mut_ptr().add(address as usize);
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, data.len());
                }
                Ok(data.len())
            }
            0xFFFF => unimplemented!("unimplemented write to IE reg"),
        };

        res
    }
}
