use std::fmt;

use iff::Chunk;

use crate::{
    error::{ErrorCode, RuntimeError},
    fatal_error,
};

#[derive(Clone, Debug)]
pub struct IFhd {
    release_number: u16,
    serial_number: Vec<u8>,
    checksum: u16,
    pc: u32,
}

impl IFhd {
    pub fn new(release_number: u16, serial_number: &[u8], checksum: u16, pc: u32) -> IFhd {
        IFhd {
            release_number,
            serial_number: serial_number.to_vec(),
            checksum,
            pc,
        }
    }

    pub fn release_number(&self) -> u16 {
        self.release_number
    }

    pub fn serial_number(&self) -> &Vec<u8> {
        &self.serial_number
    }

    pub fn checksum(&self) -> u16 {
        self.checksum
    }

    pub fn pc(&self) -> u32 {
        self.pc
    }
}

impl PartialEq for IFhd {
    fn eq(&self, other: &Self) -> bool {
        // Check everything but the PC, which will vary
        self.release_number == other.release_number
            && self.serial_number == other.serial_number
            && self.checksum == other.checksum
    }
}
impl fmt::Display for IFhd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Release: {:04x}, serial: {}, checksum: {:04x}, pc: {:06x}",
            self.release_number,
            self.serial_number
                .iter()
                .map(|x| *x as char)
                .collect::<String>(),
            self.checksum,
            self.pc
        )
    }
}

impl From<&Chunk> for IFhd {
    fn from(value: &Chunk) -> IFhd {
        let data = value.data();
        let release_number = iff::vec_as_unsigned(&data[0..2]) as u16;
        let serial_number = data[2..8].to_vec();
        let checksum = iff::vec_as_unsigned(&data[8..10]) as u16;
        let pc = iff::vec_as_unsigned(&data[10..13]) as u32;

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }
}

impl From<IFhd> for Chunk {
    fn from(value: IFhd) -> Self {
        let mut data = Vec::new();
        data.extend(iff::unsigned_as_vec(value.release_number as usize, 2));
        data.extend(&value.serial_number);
        data.extend(iff::unsigned_as_vec(value.checksum as usize, 2));
        data.extend(iff::unsigned_as_vec(value.pc as usize, 3));
        Chunk::new_chunk(0, "IFhd", data)
    }
}

pub struct Mem {
    compressed: bool,
    memory: Vec<u8>,
}

impl fmt::Debug for Mem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "compressed: {}, {} bytes",
            self.compressed,
            self.memory.len()
        )
    }
}

impl Mem {
    pub fn new(compressed: bool, memory: Vec<u8>) -> Mem {
        Mem { compressed, memory }
    }

    pub fn compressed(&self) -> bool {
        self.compressed
    }

    pub fn memory(&self) -> &Vec<u8> {
        &self.memory
    }
}

impl From<&Chunk> for Mem {
    fn from(value: &Chunk) -> Self {
        let compressed = value.id() == "CMem";
        Mem {
            compressed,
            memory: value.data().clone(),
        }
    }
}

impl From<Mem> for Chunk {
    fn from(value: Mem) -> Self {
        let id = if value.compressed { "CMem" } else { "UMem" };

        Chunk::new_chunk(0, id, value.memory().clone())
    }
}

#[derive(Debug)]
pub struct Stk {
    return_address: u32,
    flags: u8,
    result_variable: u8,
    arguments: u8,
    variables: Vec<u16>,
    stack: Vec<u16>,
}

impl Stk {
    pub fn new(
        return_address: u32,
        flags: u8,
        result_variable: u8,
        arguments: u8,
        variables: &[u16],
        stack: &[u16],
    ) -> Stk {
        Stk {
            return_address,
            flags,
            result_variable,
            arguments,
            variables: variables.to_vec(),
            stack: stack.to_vec(),
        }
    }

    pub fn return_address(&self) -> u32 {
        self.return_address
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn result_variable(&self) -> u8 {
        self.result_variable
    }

    pub fn arguments(&self) -> u8 {
        self.arguments
    }

    pub fn variables(&self) -> &Vec<u16> {
        &self.variables
    }

    pub fn stack(&self) -> &Vec<u16> {
        &self.stack
    }
}

impl From<Stk> for Vec<u8> {
    fn from(value: Stk) -> Self {
        let mut data = Vec::new();
        data.extend(iff::unsigned_as_vec(value.return_address as usize, 3));
        data.push(value.flags);
        data.push(value.result_variable);
        data.push(value.arguments);
        data.extend(iff::unsigned_as_vec(value.stack.len(), 2));
        for v in value.variables {
            data.extend(iff::unsigned_as_vec(v as usize, 2));
        }
        for v in value.stack {
            data.extend(iff::unsigned_as_vec(v as usize, 2));
        }

        data
    }
}

#[derive(Debug)]
pub struct Stks {
    stks: Vec<Stk>,
}

impl Stks {
    pub fn new(stks: Vec<Stk>) -> Stks {
        Stks { stks }
    }

    pub fn stks(&self) -> &Vec<Stk> {
        &self.stks
    }
}

impl From<&Chunk> for Stks {
    fn from(value: &Chunk) -> Self {
        let mut stks = Vec::new();
        let mut offset = 0;
        let data = value.data();
        while value.length() as usize > offset {
            let return_address = iff::vec_as_unsigned(&data[offset..offset + 3]) as u32;
            let flags = data[offset + 3];
            let result_variable = data[offset + 4];
            let arguments = data[offset + 5];
            let stack_size = iff::vec_as_unsigned(&data[offset + 6..offset + 8]);
            let mut variables = Vec::new();
            for i in 0..flags as usize & 0xf {
                let n = offset + 8 + (i * 2);
                variables.push(iff::vec_as_unsigned(&data[n..n + 2]) as u16);
            }
            let mut stack = Vec::new();
            for i in 0..stack_size {
                let n = offset + 8 + (variables.len() * 2) + (i * 2);
                stack.push(iff::vec_as_unsigned(&data[n..n + 2]) as u16);
            }

            offset += 8 + (variables.len() * 2) + (stack.len() * 2);
            stks.push(Stk::new(
                return_address,
                flags,
                result_variable,
                arguments,
                &variables,
                &stack,
            ))
        }

        Stks::new(stks)
    }
}

impl From<Stks> for Chunk {
    fn from(value: Stks) -> Self {
        let mut data = Vec::new();
        for stk in value.stks {
            data.extend(&Vec::from(stk))
        }

        Chunk::new_chunk(0, "Stks", data)
    }
}

#[derive(Debug)]
pub struct Quetzal {
    ifhd: IFhd,
    mem: Mem,
    stks: Stks,
}

impl Quetzal {
    pub fn new(ifhd: IFhd, mem: Mem, stks: Stks) -> Quetzal {
        Quetzal { ifhd, mem, stks }
    }

    pub fn ifhd(&self) -> &IFhd {
        &self.ifhd
    }

    pub fn mem(&self) -> &Mem {
        &self.mem
    }

    pub fn stks(&self) -> &Stks {
        &self.stks
    }
}

impl TryFrom<Chunk> for Quetzal {
    type Error = RuntimeError;

    fn try_from(value: Chunk) -> Result<Self, Self::Error> {
        let ifhd_chunk = value.find_chunk("IFhd", "");
        if ifhd_chunk.is_none() {
            return fatal_error!(ErrorCode::System, "No IFhd chunk");
        }
        let mem_chunk = value.find_first_chunk(vec![("CMem", ""), ("UMem", "")]);
        if mem_chunk.is_none() {
            return fatal_error!(ErrorCode::System, "No CMem or UMem chunk");
        }
        let stks_chunk = value.find_chunk("Stks", "");
        if stks_chunk.is_none() {
            return fatal_error!(ErrorCode::System, "No Stks chunk",);
        }

        Ok(Quetzal::new(
            IFhd::from(ifhd_chunk.unwrap()),
            Mem::from(mem_chunk.unwrap()),
            Stks::from(stks_chunk.unwrap()),
        ))
    }
}

impl TryFrom<Vec<u8>> for Quetzal {
    type Error = RuntimeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let chunk = Chunk::from((0, &value));
        Quetzal::try_from(chunk)
    }
}
impl From<Quetzal> for Chunk {
    fn from(value: Quetzal) -> Self {
        let ifhd = Chunk::from(value.ifhd);
        let mem = Chunk::from(value.mem);
        let stks = Chunk::from(value.stks);

        Chunk::new_form(0, "IFZS", vec![ifhd, mem, stks])
    }
}

impl From<Quetzal> for Vec<u8> {
    fn from(value: Quetzal) -> Self {
        let chunk = Chunk::from(value);
        Vec::from(&chunk)
    }
}
