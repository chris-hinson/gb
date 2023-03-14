use crate::system::ExecutionError;
/*
$FF10	NR10	Sound channel 1 sweep	R/W	All
$FF11	NR11	Sound channel 1 length timer & duty cycle	Mixed	All
$FF12	NR12	Sound channel 1 volume & envelope	R/W	All
$FF13	NR13	Sound channel 1 wavelength low	W	All
$FF14	NR14	Sound channel 1 wavelength high & control	Mixed	All
$FF16	NR21	Sound channel 2 length timer & duty cycle	Mixed	All
$FF17	NR22	Sound channel 2 volume & envelope	R/W	All
$FF18	NR23	Sound channel 2 wavelength low	W	All
$FF19	NR24	Sound channel 2 wavelength high & control	Mixed	All
$FF1A	NR30	Sound channel 3 DAC enable	R/W	All
$FF1B	NR31	Sound channel 3 length timer	W	All
$FF1C	NR32	Sound channel 3 output level	R/W	All
$FF1D	NR33	Sound channel 3 wavelength low	W	All
$FF1E	NR34	Sound channel 3 wavelength high & control	Mixed	All
$FF20	NR41	Sound channel 4 length timer	W	All
$FF21	NR42	Sound channel 4 volume & envelope	R/W	All
$FF22	NR43	Sound channel 4 frequency & randomness	R/W	All
$FF23	NR44	Sound channel 4 control	Mixed	All
$FF24	NR50	Master volume & VIN panning	R/W	All
$FF25	NR51	Sound panning	R/W	All
$FF26	NR52	Sound on/off	Mixed	All
*/

#[derive(Default, Debug)]
pub struct Audio {
    //TODO:
    //how to best structure these? maybe a custom struct for each channel?
}

impl Audio {
    pub fn read(&mut self, address: u16, len: usize) -> Result<Vec<u8>, ExecutionError> {
        warn!("reading from unimplemented audio. returning 0x00s");
        return Ok(vec![0; len]);
    }

    pub fn write(&mut self, address: u16, data: &[u8]) -> Result<usize, ExecutionError> {
        warn!("writing to unimplmented audio. no effects");
        return Ok(data.len());
    }
}
