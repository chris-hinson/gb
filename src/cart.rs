use crate::system::ExecutionError;
use std::{
    fmt::format,
    fs::{metadata, File},
    io::Read,
};

pub struct Cart {
    header: CartHeader,
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl Cart {
    pub fn new(rom: &mut std::fs::File) -> Result<Self, std::io::Error> {
        //let mut contents = vec![0; rom.metadata().unwrap().len() as usize];
        let mut contents = Vec::new();
        rom.read_to_end(&mut contents)?;

        let header = CartHeader::new(contents[0x0100..=0x014F].try_into().unwrap())?;

        //TODO: here we need to parse out allllllll our other ROM data
        Ok(Cart {
            header: header,
            rom: Vec::new(),
            ram: Vec::new(),
        })
    }
}
struct CartHeader {
    /*0100-0103 — Entry point
    After displaying the Nintendo logo, the built-in boot ROM jumps to the address $0100, which should then jump to the actual main program in the cartridge. Most commercial games fill this 4-byte area with a nop instruction followed by a jp $0150.
    */
    entry_point: [u8; 4],
    /*
    0104-0133 — Nintendo logo

    This area contains a bitmap image that is displayed when the Game Boy is powered on. It must match the following (hexadecimal) dump, otherwise the boot ROM won’t allow the game to run:

    CE ED 66 66 CC 0D 00 0B 03 73 00 83 00 0C 00 0D
    00 08 11 1F 88 89 00 0E DC CC 6E E6 DD DD D9 99
    BB BB 67 63 6E 0E EC CC DD DC 99 9F BB B9 33 3E
    */
    logo: [u8; (0x0134 - 0x0104)],
    /*
    0134-0143 — Title

    These bytes contain the title of the game in upper case ASCII. If the title is less than 16 characters long, the remaining bytes should be padded with $00s.

    Parts of this area actually have a different meaning on later cartridges, reducing the actual title size to 15 ($0134–$0142) or 11 ($0134–$013E) characters; see below.
    */
    title: [u8; (0x0144 - 0x134)],
    /*013F-0142 — Manufacturer code

    In older cartridges these bytes were part of the Title (see above). In newer cartridges they contain a 4-character manufacturer code (in uppercase ASCII). The purpose of the manufacturer code is unknown.

    */
    man_code: [u8; (0x0143 - 0x013F)],
    /*0143 — CGB flag

    In older cartridges this byte was part of the Title (see above). The CGB and later models interpret this byte to decide whether to enable Color mode (“CGB Mode”) or to fall back to monochrome compatibility mode (“Non-CGB Mode”).

    Typical values are:
    Value	Meaning
    $80	The game supports CGB enhancements, but is backwards compatible with monochrome Game Boys
    $C0	The game works on CGB only (the hardware ignores bit 6, so this really functions the same as $80)

    Values with bit 7 and either bit 2 or 3 set will switch the Game Boy into a special non-CGB-mode called “PGB mode”.

    Research needed

    The PGB mode is not well researched or documented yet. Help is welcome!*/
    cgb: u8,
    /*
    0144-0145 — New licensee code
    This area contains a two-character ASCII “licensee code” indicating the game’s publisher. It is only meaningful if the Old licensee is exactly $33 (which is the case for essentially all games made after the SGB was released); otherwise, the old code must be considered.
    */
    new_lic_code: [u8; (0x0146 - 0x0144)],
    /*
    0146 — SGB flag

    This byte specifies whether the game supports SGB functions. The SGB will ignore any command packets if this byte is set to a value other than $03 (typically $00).
     */
    sgb_flag: u8,
    /*0147 — Cartridge type

    This byte indicates what kind of hardware is present on the cartridge — most notably its mapper.
    Code	Type
    $00	ROM ONLY
    $01	MBC1
    $02	MBC1+RAM
    $03	MBC1+RAM+BATTERY
    $05	MBC2
    $06	MBC2+BATTERY
    $08	ROM+RAM 1
    $09	ROM+RAM+BATTERY 1
    $0B	MMM01
    $0C	MMM01+RAM
    $0D	MMM01+RAM+BATTERY
    $0F	MBC3+TIMER+BATTERY
    $10	MBC3+TIMER+RAM+BATTERY 2
    $11	MBC3
    $12	MBC3+RAM 2
    $13	MBC3+RAM+BATTERY 2
    $19	MBC5
    $1A	MBC5+RAM
    $1B	MBC5+RAM+BATTERY
    $1C	MBC5+RUMBLE
    $1D	MBC5+RUMBLE+RAM
    $1E	MBC5+RUMBLE+RAM+BATTERY
    $20	MBC6
    $22	MBC7+SENSOR+RUMBLE+RAM+BATTERY
    $FC	POCKET CAMERA
    $FD	BANDAI TAMA5
    $FE	HuC3
    $FF	HuC1+RAM+BATTERY
    1. No licensed cartridge makes use of this option. The exact behavior is unknown.
    2. MBC3 with 64 KiB of SRAM refers to MBC30, used only in Pocket Monsters: Crystal Version (the Japanese version of Pokémon Crystal Version).*/
    cart_type: u8,
    /*0148 — ROM size

    This byte indicates how much ROM is present on the cartridge. In most cases, the ROM size is given by 32 KiB × (1 << <value>):
    Value	ROM size	Number of ROM banks
    $00	32 KiB	2 (no banking)
    $01	64 KiB	4
    $02	128 KiB	8
    $03	256 KiB	16
    $04	512 KiB	32
    $05	1 MiB	64
    $06	2 MiB	128
    $07	4 MiB	256
    $08	8 MiB	512
    $52	1.1 MiB	72 3
    $53	1.2 MiB	80 3
    $54	1.5 MiB	96 3

    3. Only listed in unofficial docs. No cartridges or ROM files using these sizes are known. As the other ROM sizes are all powers of 2, these are likely inaccurate. The source of these values is unknown.*/
    rom_size: u8,

    /*0149 — RAM size

    This byte indicates how much RAM is present on the cartridge, if any.

    If the cartridge type does not include “RAM” in its name, this should be set to 0. This includes MBC2, since its 512 × 4 bits of memory are built directly into the mapper.
    Code	SRAM size	Comment
    $00	0	No RAM
    $01	–	Unused 4
    $02	8 KiB	1 bank
    $03	32 KiB	4 banks of 8 KiB each
    $04	128 KiB	16 banks of 8 KiB each
    $05	64 KiB	8 banks of 8 KiB each
    4. Listed in various unofficial docs as 2 KiB. However, a 2 KiB RAM chip was never used in a cartridge. The source of this value is unknown.

    Various “PD” ROMs (“Public Domain” homebrew ROMs, generally tagged with (PD) in the filename) are known to use the $01 RAM Size tag, but this is believed to have been a mistake with early homebrew tools, and the PD ROMs often don’t use cartridge RAM at all.*/
    ram_size: u8,

    /*014A — Destination code

    This byte specifies whether this version of the game is intended to be sold in Japan or elsewhere.

    Only two values are defined:
    Code	Destination
    $00	Japan (and possibly overseas)
    $01	Overseas only*/
    dest_code: u8,

    /*014B — Old licensee code
    This byte is used in older (pre-SGB) cartridges to specify the game’s publisher. However, the value $33 indicates that the New licensee codes must be considered instead. (The SGB will ignore any command packets unless this value is $33.)
    */
    old_lic_code: u8,

    /*014C — Mask ROM version number

    This byte specifies the version number of the game. It is usually $00.*/
    rom_version: u8,

    /*014D — Header checksum

    This byte contains an 8-bit checksum computed from the cartridge header bytes $0134–014C. The boot ROM computes the checksum as follows:

    uint8_t checksum = 0;
    for (uint16_t address = 0x0134; address <= 0x014C; address++) {
        checksum = checksum - rom[address] - 1;
    }

    The boot ROM verifies this checksum. If the byte at $014D does not match the lower 8 bits of checksum, the boot ROM will lock up and the program in the cartridge won’t run.*/
    header_checksum: u8,

    /*014E-014F — Global checksum

    These bytes contain a 16-bit (big-endian) checksum simply computed as the sum of all the bytes of the cartridge ROM (except these two checksum bytes).

    This checksum is not verified, except by Pokémon Stadium’s “GB Tower” emulator (presumably to detect Transfer Pak errors).
    */
    global_checksum: [u8; 2],
}

impl CartHeader {
    pub fn new(contents: [u8; 0x50]) -> Result<Self, std::io::Error> {
        let header = Self {
            entry_point: contents[0x0100 - 0x0100..=0x0103 - 0x0100]
                .try_into()
                .unwrap(),
            logo: contents[0x0104 - 0x0100..=0x0133 - 0x0100]
                .try_into()
                .unwrap(),
            title: contents[0x0134 - 0x0100..=0x0143 - 0x0100]
                .try_into()
                .unwrap(),
            man_code: contents[0x013F - 0x0100..=0x0142 - 0x0100]
                .try_into()
                .unwrap(),
            cgb: contents[0x0143 - 0x0100],
            new_lic_code: contents[0x0144 - 0x0100..=0x0145 - 0x0100]
                .try_into()
                .unwrap(),
            sgb_flag: contents[0x0146 - 0x0100],
            cart_type: contents[0x0147 - 0x0100],
            rom_size: contents[0x0148 - 0x0100],
            ram_size: contents[0x0149 - 0x0100],
            dest_code: contents[0x014A - 0x0100],
            old_lic_code: contents[0x014B - 0x0100],
            rom_version: contents[0x014C - 0x0100],
            header_checksum: contents[0x014D - 0x0100],
            global_checksum: contents[(0x014E - 0x0100)..=(0x014F - 0x0100)]
                .try_into()
                .unwrap(),
        };

        //println!("{:x}", contents[0x014D - 0x0100]);

        /*
        uint8_t checksum = 0;
        for (uint16_t address = 0x0134; address <= 0x014C; address++) {
            checksum = checksum - rom[address] - 1;
        } */
        let mut checksum: u8 = 0;
        for address in (0x0134 - 0x0100)..=(0x014C - 0x0100) {
            checksum = checksum.wrapping_sub(contents[address]).wrapping_sub(1);
        }

        if checksum != header.header_checksum {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "header checksum failed, got: {:x}, expected: {:x}",
                    checksum, header.header_checksum,
                ),
            ));
        }

        //TODO: do we actually care about the global checksum?
        //i am not checking it rn

        Ok(header)
    }
}

impl Cart {
    pub fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
        let value = match address {
            //0100-0103 — Entry point
            0x0100..=0x0103 => {
                if len > 4 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                self.header.entry_point[address as usize..address as usize + len].to_vec()
            }
            //nintendo logo
            0x0104..=0x0133 => {
                let address: usize = (address - 0x0104) as usize;
                self.header.logo[address..address + len as usize].to_vec()
            }
            //0134-0143 — Title
            0x0134..=0x0143 => {
                if len > (0x0144 - 0x0134) {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                let address: usize = (address - 0x0134) as usize;
                self.header.title[address..address + len].to_vec()
            }
            //013F-0142 — Manufacturer code
            //NOTE: you will need to fix this unreachable pattern based on mapper type somehow
            0x013F..=0x0142 => {
                if len > (0x0143 - 0x013F) {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                let address: usize = (address - 0x013F) as usize;
                self.header.man_code[address..address + len].to_vec()
            }
            //0143 — CGB flag
            //NOTE: same as above
            0x0143 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.cgb]
            }
            //0144-0145 — New licensee code
            0x0144..=0x0145 => {
                if len > (0x0146 - 0x0144) {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                let address: usize = (address - 0x0144) as usize;
                self.header.new_lic_code[address..address + len].to_vec()
            }
            //0146 — SGB flag
            0x0146 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.sgb_flag]
            }
            //0147 — Cartridge type
            0x0147 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.cart_type]
            }
            //0148 — ROM size
            0x0148 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.rom_size]
            }
            //0149 — RAM size
            0x0149 => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.ram_size]
            }
            //014A — Destination code
            0x014A => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.dest_code]
            }
            //014B — Old licensee code
            0x014B => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.old_lic_code]
            }
            //014C — Mask ROM version number
            0x014C => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.rom_version]
            }
            //014D — Header checksum
            0x014D => {
                if len > 1 {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                vec![self.header.header_checksum]
            }
            //014E-014F — Global checksum
            0x014E..=0x14F => {
                if len > (0x150 - 0x14E) {
                    return Err(ExecutionError::IllegalRead(address as usize));
                }
                let address: usize = (address - 0x014E) as usize;
                self.header.global_checksum[address..address + len].to_vec()
            }
            _ => {
                warn!("reading from cart region which is stubbed to just give back 0x00");
                //return Err(ExecutionError::IllegalRead(address as usize));
                vec![0; len]
            }
        };
        return Ok(value.to_vec());
    }

    pub fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        warn!("writing to cart is just going to nothingness rn");
        return Ok(data.len());
    }
}
