use crate::audio::Audio;
use crate::system::ExecutionError;

//memory mapped registers/ other assorted IO

/*IO memory map
$FF00		    DMG	Joypad input
$FF01	$FF02	DMG	Serial transfer
$FF04	$FF07	DMG	Timer and divider
$FF10	$FF26	DMG	Audio
$FF30	$FF3F	DMG	Wave pattern
$FF40	$FF4B	DMG	LCD Control, Status, Position, Scrolling, and Palettes
$FF4F		    CGB	VRAM Bank Select
$FF50		    DMG	Set to non-zero to disable boot ROM
$FF51	$FF55	CGB	VRAM DMA
$FF68	$FF69	CGB	BG / OBJ Palettes
$FF70		    CGB	WRAM Bank Select */
#[derive(Default)]
pub struct Io {
    //joypad: u8,
    pub bootrom_disable: u8,
    pub audio: Audio,
    //dmg_serial_transfer: [u8;2]
}

impl Io {
    pub fn new() -> Self {
        Io {
            ..Default::default()
        }
    }

    pub fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
        match address {
            0xFF00 => unimplemented!("tried to read joypad input"),
            0xFF01..=0xFF02 => unimplemented!("tried to read DMG serial transfer"),
            0xFF04..=0xFF07 => unimplemented!("tried to read DMG timer and divider"),
            0xFF10..=0xFF26 => {
                //unimplemented!("tried to read DMG audio")
                self.audio.read(address, len)
            }
            0xFF30..=0xFF3F => unimplemented!("tried to read DMG wave pattern"),
            0xFF40..=0xFF4B => unimplemented!("tried to read LCD control stuff"),
            0xFF4F => unimplemented!("tried to read CGB VRAM bank select"),
            0xFF50 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                Ok(vec![self.bootrom_disable])
            }
            0xFF51..=0xFF55 => unimplemented!("tried to read CGB VRAM DMA"),
            0xFF68..=0xFF69 => unimplemented!("tried to read CGB BG/OBJ PPalettes"),
            0xFF70 => unimplemented!("tried to read CGB WRAM bank select"),
            _ => panic!("read from bad address in I/O range"),
        }
    }

    pub fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        match address {
            0xFF00 => unimplemented!("tried to read joypad input"),
            0xFF01..=0xFF02 => unimplemented!("tried to read DMG serial transfer"),
            0xFF04..=0xFF07 => unimplemented!("tried to read DMG timer and divider"),
            0xFF10..=0xFF26 => {
                //unimplemented!("tried to read DMG audio")
                self.audio.write(address, data)
            }
            0xFF30..=0xFF3F => unimplemented!("tried to read DMG wave pattern"),
            0xFF40..=0xFF4B => unimplemented!("tried to read LCD control stuff"),
            0xFF4F => unimplemented!("tried to read CGB VRAM bank select"),
            0xFF50 => {
                if data.len() > 1 {
                    Err(ExecutionError::IllegalWrite(address as usize))
                } else {
                    self.bootrom_disable = data[0];
                    return Ok(1);
                }
            }
            0xFF51..=0xFF55 => unimplemented!("tried to read CGB VRAM DMA"),
            0xFF68..=0xFF69 => unimplemented!("tried to read CGB BG/OBJ PPalettes"),
            0xFF70 => unimplemented!("tried to read CGB WRAM bank select"),
            _ => panic!("read from bad address in I/O range"),
        }
    }
}
