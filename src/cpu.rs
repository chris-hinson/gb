use std::fs::File;

use crate::cart::Cart;
use Register16::*;
use Register8::*;

#[derive(Clone)]
pub struct Cpu {
    pub rf: RegisterFile,
}

impl Cpu {
    pub fn new() -> Result<Self, std::io::Error> {
        Ok(Cpu {
            rf: RegisterFile::new(),
        })
    }
}
impl std::fmt::Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rf)
    }
}

pub enum Register8 {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
}
impl TryFrom<usize> for Register8 {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            //B
            0x0 => Ok(B),
            //C
            0x1 => Ok(C),
            //D
            0x2 => Ok(D),
            //E
            0x3 => Ok(E),
            //H
            0x4 => Ok(H),
            //L
            0x5 => Ok(L),
            //(HL)
            0x6 => Err("indirect HL encoding"),
            //
            0x7 => Ok(A),
            _ => panic!("cannot convert this usize to an r8"),
        }
    }
}

pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
    HLInd,
    SP,
    PC,
}

impl std::ops::Index<Register8> for RegisterFile {
    type Output = u8;

    fn index(&self, index: Register8) -> &Self::Output {
        match index {
            Register8::A => &self.A,
            Register8::F => &self.F,
            Register8::B => &self.B,
            Register8::C => &self.C,
            Register8::D => &self.D,
            Register8::E => &self.E,
            Register8::H => &self.H,
            Register8::L => &self.L,
        }
    }
}

impl std::ops::IndexMut<Register8> for RegisterFile {
    fn index_mut(&mut self, index: Register8) -> &mut Self::Output {
        match index {
            Register8::A => &mut self.A,
            Register8::F => &mut self.F,
            Register8::B => &mut self.B,
            Register8::C => &mut self.C,
            Register8::D => &mut self.D,
            Register8::E => &mut self.E,
            Register8::H => &mut self.H,
            Register8::L => &mut self.L,
        }
    }
}

/*impl std::ops::Index<Register16> for RegisterFile {
    type Output = (&u8, &u8);
    fn index(&self, index: Register16) -> &Self::Output {
        match index {
            Register16::AF => &(&self.A, &self.F), //&((((self.A as u16) << 8) | self.F as u16) as u16).to_owned(),
            Register16::BC => &(&self.B, &self.C), //&((((self.B as u16) << 8) | self.C as u16) as u16),
            Register16::DE => &(&self.D, &self.E), //&((((self.D as u16) << 8) | self.E as u16) as u16),
            Register16::HL => &(&self.H, &self.L), //&((((self.H as u16) << 8) | self.L as u16) as u16),
            Register16::SP => &(&self.A, &self.F), //&self.SP,
            Register16::PC => &(&self.A, &self.F), //&self.PC,
        }
    }
}*/

/*
16-bit	Hi	Lo	Name/Function
AF	    A	-	Accumulator & Flags
BC	    B  	C	BC
DE	    D	E	DE
HL	    H	L	HL
SP	    -	-	Stack Pointer
PC	    -	-	Program Counter/Pointer */
#[allow(non_snake_case)]
#[derive(Default, Clone)]
pub struct RegisterFile {
    pub A: u8,
    pub F: u8,
    pub B: u8,
    pub C: u8,
    pub D: u8,
    pub E: u8,
    pub H: u8,
    pub L: u8,
    pub SP: u16,
    pub PC: u16,
}
impl std::fmt::Display for RegisterFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PC: {:#04X}\nSP: {:#04X}\nAF: {:#02X} {:#02X}\nBC: {:#02X} {:#02X}\nDE: {:#02X} {:#02X}\nHL: {:#02X} {:#02X}\n",self.PC,self.SP,self.A,self.F,self.B,self.C,self.D,self.E,self.H,self.L)
    }
}

impl RegisterFile {
    //TODO: i think the startup state is non-zero, might want to bring this in line with that
    pub fn new() -> Self {
        Self {
            ..Default::default() /*A: 0,
                                 F: 0,
                                 B: 0,
                                 C: 0,
                                 D: 0,
                                 E: 0,
                                 H: 0,
                                 L: 0,
                                 SP: 0,
                                 PC: 0,*/
        }
    }

    pub fn AF_read(&self) -> u16 {
        //(((self.A as u16) << 8) | self.F as u16) as u16
        (((self[A] as u16) << 8) | self[F] as u16) as u16
    }
    pub fn AF_write(&mut self, val: u16) {
        self.A = ((val & 0xFF00) >> 8) as u8;
        self.F = (val & 0x00FF) as u8;
    }

    pub fn BC_read(&self) -> u16 {
        (((self.B as u16) << 8) | self.C as u16) as u16
    }
    pub fn BC_write(&mut self, val: u16) {
        self.B = ((val & 0xFF00) >> 8) as u8;
        self.C = (val & 0x00FF) as u8;
    }

    pub fn DE_read(&self) -> u16 {
        (((self.D as u16) << 8) | self.E as u16) as u16
    }
    pub fn DE_write(&mut self, val: u16) {
        self.D = ((val & 0xFF00) >> 8) as u8;
        self.E = (val & 0x00FF) as u8;
    }

    pub fn HL_read(&self) -> u16 {
        (((self.H as u16) << 8) | self.L as u16) as u16
    }
    pub fn HL_write(&mut self, val: u16) {
        self.H = ((val & 0xFF00) >> 8) as u8;
        self.L = (val & 0x00FF) as u8;
    }

    pub fn rf_read16(&self, reg: Register16) -> u16 {
        match reg {
            AF => self.AF_read(),
            BC => self.BC_read(),
            DE => self.DE_read(),
            HL => self.HL_read(),
            SP => self.SP,
            PC => self.PC,
            _ => panic!("do not call rf_read16 with a HLInd arg thank you"),
        }
    }
    pub fn rf_write16(&mut self, reg: Register16, val: u16) {
        match reg {
            AF => self.AF_write(val),
            BC => self.BC_write(val),
            DE => self.DE_write(val),
            HL => self.HL_write(val),
            SP => self.SP = val,
            PC => self.PC = val,
            _ => panic!("do not call rf_write16 with an HLInd arg thank you"),
        }
    }

    /*
    7	z	Zero flag
    6	n	Subtraction flag (BCD)
    5	h	Half Carry flag (BCD)
    4	c	Carry flag*/
    pub fn z_get(&self) -> bool {
        if (self.F & 0b1000_0000) != 0 {
            true
        } else {
            false
        }
    }
    pub fn z_set(&mut self, val: bool) {
        self.F &= if val { 0b1000_0000 } else { 0 }
    }

    pub fn n_get(&self) -> bool {
        if (self.F & 0b0100_0000) != 0 {
            true
        } else {
            false
        }
    }
    pub fn n_set(&mut self, val: bool) {
        self.F &= if val { 0b0100_0000 } else { 0 }
    }

    pub fn h_get(&self) -> bool {
        if (self.F & 0b0010_0000) != 0 {
            true
        } else {
            false
        }
    }
    pub fn h_set(&mut self, val: bool) {
        self.F &= if val { 0b0010_0000 } else { 0 }
    }

    pub fn c_get(&self) -> bool {
        if (self.F & 0b0001_0000) != 0 {
            true
        } else {
            false
        }
    }
    pub fn c_set(&mut self, val: bool) {
        self.F &= if val { 0b0001_0000 } else { 0 }
    }
}
