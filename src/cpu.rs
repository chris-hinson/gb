use std::fs::File;

use crate::cart::Cart;
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

/*
16-bit	Hi	Lo	Name/Function
AF	    A	-	Accumulator & Flags
BC	    B  	C	BC
DE	    D	E	DE
HL	    H	L	HL
SP	    -	-	Stack Pointer
PC	    -	-	Program Counter/Pointer */
#[allow(non_snake_case)]
#[derive(Default)]
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
        (((self.A as u16) << 8) | self.F as u16) as u16
    }
    pub fn AF_write(&mut self, AF: u16) {
        self.A = ((AF & 0xFF00) >> 8) as u8;
        self.F = (AF & 0x00FF) as u8;
    }

    pub fn BC_read(&self) -> u16 {
        (((self.B as u16) << 8) | self.C as u16) as u16
    }
    pub fn BC_write(&mut self, BC: u16) {
        self.B = ((BC & 0xFF00) >> 8) as u8;
        self.C = (BC & 0x00FF) as u8;
    }

    pub fn DE_read(&self) -> u16 {
        (((self.D as u16) << 8) | self.E as u16) as u16
    }
    pub fn DE_write(&mut self, DE: u16) {
        self.D = ((DE & 0xFF00) >> 8) as u8;
        self.E = (DE & 0x00FF) as u8;
    }

    pub fn HL_read(&self) -> u16 {
        (((self.H as u16) << 8) | self.L as u16) as u16
    }
    pub fn HL_write(&mut self, HL: u16) {
        self.H = ((HL & 0xFF00) >> 8) as u8;
        self.L = (HL & 0x00FF) as u8;
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
