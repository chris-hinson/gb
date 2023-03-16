use std::default;

use crate::system::ExecutionError;

/*$FF40	LCDC    LCD control                 	R/W	All
$FF41	STAT    LCD status	                    Mixed	All
$FF42	SCY	    Viewport Y position	            R/W	All
$FF43	SCX	    Viewport X position	            R/W	All
$FF44	LY	    LCD Y coordinate	            R	All
$FF45	LYC	    LY compare	                    R/W	All
$FF46	DMA	    OAM DMA source address & start	R/W	All
$FF47	BGP	    BG palette data	                R/W	DMG
$FF48	OBP0	OBJ palette 0 data	            R/W	DMG
$FF49	OBP1	OBJ palette 1 data	            R/W	DMG
$FF4A	WY	    Window Y position	            R/W	All
$FF4B	WX	    Window X position plus 7	    R/W	All */

#[derive(Debug, Clone, Copy, Default)]
pub struct Ppu {
    LCDC: ppuctrl,
    STAT: ppustat,
    SCY: u8,
    SCX: u8,
    LY: u8,
    LYC: u8,
    DMA: u8,
    BGP: u8,
    OBP0: u8,
    OBP1: u8,
    WY: u8,
    WX: u8,
}

/*
7	LCD and PPU enable	0=Off, 1=On
6	Window tile map area	0=9800-9BFF, 1=9C00-9FFF
5	Window enable	0=Off, 1=On
4	BG and Window tile data area	0=8800-97FF, 1=8000-8FFF
3	BG tile map area	0=9800-9BFF, 1=9C00-9FFF
2	OBJ size	0=8x8, 1=8x16
1	OBJ enable	0=Off, 1=On
0	BG and Window enable/priority	0=Off, 1=On */
#[derive(Debug, Clone, Copy, Default)]
struct ppuctrl {
    enable: bool,
    tilemap_area: bool,
    window_enable: bool,
    BGWindow_area: bool,
    BG_tilemap_area: bool,
    OBJ_size: bool,
    OBJ_enable: bool,
    BGWindow_enable: bool,
}
impl From<u8> for ppuctrl {
    fn from(value: u8) -> Self {
        Self {
            enable: (value & 0b1000_0000) != 0,
            tilemap_area: (value & 0b0100_0000) != 0,
            window_enable: (value & 0b0010_0000) != 0,
            BGWindow_area: (value & 0b0001_0000) != 0,
            BG_tilemap_area: (value & 0b0000_1000) != 0,
            OBJ_size: (value & 0b0000_0100) != 0,
            OBJ_enable: (value & 0b0000_0010) != 0,
            BGWindow_enable: (value & 0b0000_0001) != 0,
        }
    }
}
impl Into<u8> for ppuctrl {
    fn into(self) -> u8 {
        let mut value: u8 = 0;
        if self.enable {
            value |= 0b1000_0000
        };
        if self.tilemap_area {
            value |= 0b0100_0000
        };
        if self.window_enable {
            value |= 0b0010_0000
        };
        if self.BGWindow_area {
            value |= 0b0001_0000
        };
        if self.BG_tilemap_area {
            value |= 0b0000_1000
        };
        if self.OBJ_size {
            value |= 0b0000_0100
        };
        if self.OBJ_enable {
            value |= 0b0000_0010
        };
        if self.BGWindow_enable {
            value |= 0b0000_0001
        };
        value
    }
}

/*
Bit 6 - LYC=LY STAT Interrupt source         (1=Enable) (Read/Write)
Bit 5 - Mode 2 OAM STAT Interrupt source     (1=Enable) (Read/Write)
Bit 4 - Mode 1 VBlank STAT Interrupt source  (1=Enable) (Read/Write)
Bit 3 - Mode 0 HBlank STAT Interrupt source  (1=Enable) (Read/Write)
Bit 2 - LYC=LY Flag                          (0=Different, 1=Equal) (Read Only)
Bit 1-0 - Mode Flag                          (Mode 0-3, see below) (Read Only)
          0: HBlank
          1: VBlank
          2: Searching OAM
          3: Transferring Data to LCD Controller */
#[derive(Clone, Copy, Debug, Default)]
struct ppustat {
    LYCLY_int: bool,
    Mode2_int: bool,
    Mode1_int: bool,
    Mode0_int: bool,
    LYCLY: bool,
    Mode: PpuMode,
}

#[derive(Debug, Clone, Copy, Default)]
enum PpuMode {
    #[default]
    Hblank,
    VBlank,
    OAMsearch,
    Transfer,
}
impl From<u8> for ppustat {
    fn from(value: u8) -> Self {
        Self {
            LYCLY_int: (value & 0b0100_0000) != 0,
            Mode2_int: (value & 0b0010_0000) != 0,
            Mode1_int: (value & 0b0001_0000) != 0,
            Mode0_int: (value & 0b0000_1000) != 0,
            LYCLY: (value & 0b0000_0100) != 0,
            Mode: match value & 0b0000_0011 {
                0x0 => PpuMode::Hblank,
                0x1 => PpuMode::VBlank,
                0x2 => PpuMode::OAMsearch,
                0x3 => PpuMode::Transfer,
                _ => unreachable!("somehow panicking in converting a u8 to a ppustat"),
            },
        }
    }
}
impl Into<u8> for ppustat {
    fn into(self) -> u8 {
        let mut value: u8 = 0;
        if self.LYCLY_int {
            value |= 0b0100_0000
        };
        if self.Mode2_int {
            value |= 0b0010_0000
        };
        if self.Mode1_int {
            value |= 0b0001_0000
        };
        if self.Mode0_int {
            value |= 0b0000_1000
        };
        if self.LYCLY {
            value |= 0b0000_0100
        };
        match self.Mode {
            PpuMode::Hblank => value |= 0x0,
            PpuMode::VBlank => value |= 0x1,
            PpuMode::OAMsearch => value |= 0x2,
            PpuMode::Transfer => value |= 0x3,
        }

        value
    }
}
/*$FF40	LCDC    LCD control                 	R/W	All
$FF41	STAT    LCD status	                    Mixed	All
$FF42	SCY	    Viewport Y position	            R/W	All
$FF43	SCX	    Viewport X position	            R/W	All
$FF44	LY	    LCD Y coordinate	            R	All
$FF45	LYC	    LY compare	                    R/W	All
$FF46	DMA	    OAM DMA source address & start	R/W	All
$FF47	BGP	    BG palette data	                R/W	DMG
$FF48	OBP0	OBJ palette 0 data	            R/W	DMG
$FF49	OBP1	OBJ palette 1 data	            R/W	DMG
$FF4A	WY	    Window Y position	            R/W	All
$FF4B	WX	    Window X position plus 7	    R/W	All */

impl Ppu {
    //NOTE: these read and write functions vary slightly from every other signature in the codebase
    //in that i only want to every read or write a single byte at a time (seeing as these are essentially MMIO regs)
    //and dealing with writing across them would fucking suck
    pub fn read(&mut self, address: u16) -> Result<u8, ExecutionError> {
        let value = match address {
            0xFF40 => self.LCDC.into(),
            0xFF41 => self.STAT.into(),
            0xFF42 => self.SCY,
            0xFF43 => self.SCX,
            0xFF44 => self.LY,
            0xFF45 => self.LYC,
            0xFF46 => self.DMA,
            0xFF47 => self.BGP,
            0xFF48 => self.OBP0,
            0xFF49 => self.OBP1,
            0xFF4A => self.WY,
            0xFF4B => self.WX,
            _ => unreachable!("PPU trying to service a READ outside of its memory mapping range"),
        };
        Ok(value)
    }
    pub fn write(&mut self, address: u16, data: u8) -> Result<usize, ExecutionError> {
        match address {
            0xFF40 => self.LCDC = data.into(),
            0xFF41 => self.STAT = data.into(),
            0xFF42 => self.SCY = data,
            0xFF43 => self.SCX = data,
            0xFF44 => self.LY = data,
            0xFF45 => self.LYC = data,
            0xFF46 => self.DMA = data,
            0xFF47 => self.BGP = data,
            0xFF48 => self.OBP0 = data,
            0xFF49 => self.OBP1 = data,
            0xFF4A => self.WY = data,
            0xFF4B => self.WX = data,
            _ => unreachable!("PPU trying to service a READ outside of its memory mapping range"),
        };

        Ok(1)
    }
}
