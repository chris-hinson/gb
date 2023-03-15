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

struct Ppu {
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
struct ppuctrl {}
impl From<u8> for ppuctrl {
    fn from(value: u8) -> Self {
        Self {}
    }
}

struct ppustat {}
impl From<u8> for ppustat {
    fn from(value: u8) -> Self {
        Self {}
    }
}
impl Ppu {
    //NOTE: these read and write functions vary slightly from every other signature in the codebase
    //in that i only want to every read or write a single byte at a time (seeing as these are essentially MMIO regs)
    //and dealing with writing across them would fucking suck
    pub fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
        Ok(vec![0])
    }
    fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        Ok(0)
    }
}
