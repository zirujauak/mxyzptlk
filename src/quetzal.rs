use crate::executor::{header, state::State};

pub fn usize_as_vec(d: usize, bytes: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in (0..bytes).rev() {
        data.push(((d >> (8 * i)) & 0xFF) as u8);
    }
    data
}

pub fn vec_as_usize(v: Vec<u8>, bytes: usize) -> usize {
    let mut u: usize = 0;
    for i in 0..bytes {
        u = u | ((v[i] as usize) << ((bytes - 1 - i) * 8));
    }

    u
}

pub fn id_as_vec(id: &str) -> Vec<u8> {
    id.as_bytes()[0..4].to_vec()
}

pub fn chunk(id: &str, data: &mut Vec<u8>) -> Vec<u8> {
    let mut chunk = id_as_vec(id);
    let data_length = data.len();
    chunk.append(&mut usize_as_vec(data.len(), 4));
    chunk.append(data);
    if data_length % 2 == 1 {
        // Padding byte, not included in chunk length
        chunk.push(0);
    }

    chunk
}

pub struct IFhd {
    pub release_number: u16,
    pub serial_number: Vec<u8>,
    pub checksum: u16,
    pub pc: u32,
}

impl IFhd {
    pub fn from_state(state: &State, address: usize) -> IFhd {
        IFhd {
            release_number: header::release_number(state),
            serial_number: header::serial_number(state),
            checksum: header::checksum(state),
            pc: address as u32 & 0xFFFFFF,
        }
    }

    pub fn from_vec(chunk: Vec<u8>) -> IFhd {
        let release_number = vec_as_usize(chunk[0..2].to_vec(), 2) as u16;
        let serial_number = chunk[2..8].to_vec();
        let checksum = vec_as_usize(chunk[8..10].to_vec(), 2) as u16;
        let pc = vec_as_usize(chunk[10..13].to_vec(), 3) as u32;

        IFhd {
            release_number,
            serial_number,
            checksum,
            pc,
        }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.append(&mut usize_as_vec(self.release_number as usize, 2));
        data.append(&mut self.serial_number.clone());
        data.append(&mut usize_as_vec(self.checksum as usize, 2));
        data.append(&mut usize_as_vec(self.pc as usize, 3));

        chunk("IFhd", &mut data)
    }
}

pub struct UMem {
    pub data: Vec<u8>,
}

impl UMem {
    pub fn from_state(state: &State) -> UMem {
        UMem {
            data: state.memory_map()[0..header::static_memory_base(state) as usize].to_vec(),
        }
    }

    pub fn from_vec(chunk: Vec<u8>) -> UMem {
        UMem {
            data: chunk.clone(),
        }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        chunk("UMem", &mut self.data.clone())
    }
}

pub struct StackFrame {
    pub return_address: u32,
    pub flags: u8,
    pub result_variable: u8,
    pub arguments: u8,
    pub stack_size: u16,
    pub local_variables: Vec<u16>,
    pub stack: Vec<u16>,
}

pub struct Stks {
    pub stks: Vec<StackFrame>,
}

impl Stks {
    pub fn from_state(state: &State) -> Stks {
        let mut stks = Vec::new();
        for f in &state.frames {
            trace!("Frame: {}", f.local_variables.len());
            let flags = match f.result {
                Some(_) => 0x00,
                None => 0x10,
            } | f.local_variables.len();
            let mut arguments = 0;
            for _ in 0..f.argument_count {
                arguments = (arguments << 1) + 1;
            }

            stks.push(StackFrame {
                return_address: f.return_address as u32,
                flags: flags as u8,
                result_variable: match f.result {
                    Some(v) => v,
                    None => 0,
                },
                arguments,
                stack_size: f.stack.len() as u16,
                local_variables: f.local_variables.clone(),
                stack: f.stack.clone(),
            });
        }

        Stks { stks }
    }

    pub fn from_vec(chunk: Vec<u8>) -> Stks {
        let mut position = 0;
        let mut stks = Vec::new();
        while chunk.len() - position > 1 {
            trace!("Reading frame from {} [{}]", position, chunk.len());
            let return_address = vec_as_usize(chunk[position..position+3].to_vec(), 3) as u32;
            let flags = chunk[position+3];
            let result_variable = chunk[position+4];
            let arguments = chunk[position+5];

            trace!("Return address: {:#06x}", return_address);
            trace!("Flags: {:#08b}", flags);
            trace!("Result variable: {}", result_variable);
            trace!("Arguments: {:#08b}", arguments);

            let offset = position + 6;
            let stack_size = vec_as_usize(chunk[offset..offset+2].to_vec(), 2) as u16;
            trace!("Stack size: {}", stack_size);

            let mut local_variables = Vec::new();
            for i in 0..flags as usize & 0xF {
                let offset = position + 8 + (i * 2);
                local_variables.push(vec_as_usize(chunk[offset..offset+2].to_vec(), 2) as u16);
            }
            trace!("Local variable count: {}", local_variables.len());

            let mut stack = Vec::new();
            for i in 0..stack_size as usize {
                let offset = position + 10 + (local_variables.len() * 2) + (i * 2);
                stack.push(vec_as_usize(chunk[offset..offset+2].to_vec(), 2) as u16)
            }
            trace!("Stack count: {}", stack.len());
            position = position + 8 + (local_variables.len() * 2) + (stack_size as usize * 2);

            stks.push(StackFrame {
                return_address,
                flags,
                result_variable,
                arguments,
                local_variables,
                stack_size,
                stack,
            });
        }
        Stks {
            stks
        }
    }

    pub fn to_chunk(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for stk in &self.stks {
            data.append(&mut usize_as_vec(stk.return_address as usize, 3));
            data.push(stk.flags);
            data.push(stk.result_variable);
            data.push(stk.arguments);
            data.append(&mut usize_as_vec(stk.stack_size as usize, 2));
            for i in 0..stk.local_variables.len() {
                data.append(&mut usize_as_vec(stk.local_variables[i] as usize, 2));
            }
            for i in 0..stk.stack.len() {
                data.append(&mut usize_as_vec(stk.stack[i] as usize, 2));
            }
        }
        chunk("Stks", &mut data)
    }
}

pub struct Quetzal {
    pub ifhd: IFhd,
    pub umem: UMem,
    pub stks: Stks,
}

impl Quetzal {
    pub fn from_state(state: &State, instruction_address: usize) -> Quetzal {
        let ifhd = IFhd::from_state(state, instruction_address);
        let umem = UMem::from_state(state);
        let stks = Stks::from_state(state);

        Quetzal { ifhd, umem, stks }
    }

    pub fn from_vec(data: Vec<u8>) -> Quetzal {
        // Offset 0: "FORM"
        // Offset 4: 32-bit length
        let length = vec_as_usize(data[4..8].to_vec(), 4);
        // Offset 8: "IFZS"
        // Offset C: First chunk
        let mut position = 0xC as usize;
        let mut ifhd = None;
        let mut umem = None;
        let mut stks = None;
        while length as isize - position as isize > 1 {
            let id = String::from_utf8(data[position..position + 4].to_vec()).unwrap();
            let len = vec_as_usize(data[position + 4..position + 8].to_vec(), 4);
            match id.as_str() {
                "IFhd" => {
                    let chunk_data = data[position + 8..position + 8 + len].to_vec();
                    ifhd = Some(IFhd::from_vec(chunk_data));
                }
                "UMem" => {
                    let chunk_data = data[position + 8..position + 8 + len].to_vec();
                    umem = Some(UMem::from_vec(chunk_data));
                }
                "Stks" => {
                    let chunk_data = data[position + 8..position + 8 + len].to_vec();
                    stks = Some(Stks::from_vec(chunk_data));
                }
                _ => {}
            }
            position = position + 8 + len;
            if len % 2 == 1 {
                position = position + 1;
            }
        }

        Quetzal {
            ifhd: ifhd.unwrap(),
            umem: umem.unwrap(),
            stks: stks.unwrap(),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        // IFF FORM
        let mut form = id_as_vec("FORM");

        let mut ifzs = id_as_vec("IFZS");
        ifzs.append(&mut self.ifhd.to_chunk());
        ifzs.append(&mut self.umem.to_chunk());
        ifzs.append(&mut self.stks.to_chunk());

        form.append(&mut usize_as_vec(ifzs.len(), 4));
        form.append(&mut ifzs);
        if form.len() % 2 == 1 {
            form.push(0);
        }

        form
    }
}
