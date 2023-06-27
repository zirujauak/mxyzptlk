use crate::{
    error::RuntimeError,
    iff::quetzal::{
        cmem::CMem,
        ifhd::IFhd,
        stks::{StackFrame, Stks},
        Quetzal,
    }, zmachine::state::header::{self, HeaderField},
};

use super::State;

pub fn compress(memory: &Vec<u8>, dynamic: &Vec<u8>) -> Vec<u8> {
    let mut save_data: Vec<u8> = Vec::new();
    let mut run_length = 0;
    let dynamic_len = dynamic.len();
    for i in 0..dynamic_len {
        let b = memory[i] ^ dynamic[i];
        if b == 0 {
            if run_length == 255 {
                save_data.push(0);
                save_data.push(run_length);
                run_length = 0;
            } else {
                run_length = run_length + 1;
            }
        } else {
            if run_length > 0 {
                save_data.push(0);
                save_data.push(run_length - 1);
                run_length = 0;
            }
            save_data.push(b);
        }
    }

    if run_length > 0 {
        save_data.push(0);
        save_data.push(run_length - 1);
    }

    save_data
}

impl TryFrom<&State> for CMem {
    type Error = RuntimeError;

    fn try_from(value: &State) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building CMem chunk from state");
        let compressed_memory = compress(value.memory().buffer(), value.dynamic());
        let cmem = CMem::new(&compressed_memory);
        debug!(target: "app::quetzal", "CMem: {}", cmem);
        Ok(cmem)
    }
}

pub fn decompress(cmem: &CMem, dynamic: &Vec<u8>) -> Vec<u8> {
    let mut data = Vec::new();
    let mut iter = cmem.data().iter();
    let mut done = false;

    while !done {
        let b = iter.next();
        match b {
            Some(b) => {
                let i = data.len();
                if *b == 0 {
                    let l = *iter.next().unwrap() as usize;
                    for j in 0..l + 1 {
                        data.push(dynamic[i + j]);
                    }
                } else {
                    data.push(b ^ dynamic[i])
                }
            }
            None => done = true,
        }
    }

    data
}

impl TryFrom<(&State, usize)> for IFhd {
    type Error = RuntimeError;

    fn try_from((state, pc): (&State, usize)) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building IFhd chunk from state");

        let release_number = header::field_word(state, HeaderField::Release)?;
        let mut serial_number = Vec::new();
        for i in 0..6 {
            serial_number.push(state.read_byte(HeaderField::Serial as usize + i)?);
        }
        let checksum = header::field_word(state, HeaderField::Checksum)?;

        let ifhd = IFhd::new(release_number, &serial_number, checksum, (pc as u32) & 0xFFFFFF);
        debug!(target: "app::quetzal", "IFhd: {}", ifhd);
        Ok(ifhd)
    }
}

impl TryFrom<&State> for Stks {
    type Error = RuntimeError;

    fn try_from(value: &State) -> Result<Self, Self::Error> {
        debug!(target: "app::quetzal", "Building Stks chunk from state");
        let mut frames = Vec::new();
        for f in &value.frames {
            // Flags: 0b000rvvvv
            //  r = 1 if the frame routine does not store a result
            //  vvvv = the number of local variables (0 - 15)
            let flags = match f.result() {
                Some(_) => 0x00,
                None => 0x10,
            } | f.local_variables().len();

            // Arguments: 0b87654321
            //  bits are set for each argument
            let mut arguments = 0;
            for _ in 0..f.argument_count() {
                arguments = (arguments << 1) | 0x01;
            }

            // Store result, or 0 if the routine doesn't store a result.
            // Note that "0" is also the stack if bit 4 of flags is set
            let result_variable = match f.result() {
                Some(r) => r.variable(),
                None => 0,
            };

            let frame = StackFrame::new(
                f.return_address() as u32,
                flags as u8,
                result_variable,
                arguments,
                &f.local_variables().clone(),
                &f.stack().clone(),
            );
            debug!(target: "app::quetzal", "Frame: {}", frame);
            frames.push(frame);
        }

        let stks = Stks::new(frames);
        Ok(stks)
    }
}

pub fn quetzal(state: &State, pc: usize) -> Result<Quetzal, RuntimeError> {
    let ifhd = IFhd::try_from((state, pc))?;
    let cmem = CMem::try_from(state)?;
    let stks = Stks::try_from(state)?;

    let quetzal = Quetzal::new(ifhd, None, Some(cmem), stks);
    Ok(quetzal)
}
