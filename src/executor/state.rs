use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::executor::header;
use crate::executor::util;

#[derive(Debug)]
pub struct Frame {
    address: usize,
    pub pc: usize,
    local_variables: Vec<u16>,
    stack: Vec<u16>,
    result: Option<u8>
}

impl Frame {
    fn initial(_memory_map: &Vec<u8>, address: usize) -> Frame {
        Frame {
            address,
            pc: address,
            local_variables: Vec::new(),
            stack: Vec::new(),
            result: None
        }
    }

    fn call(memory_map: &Vec<u8>, version: u8, address: usize, arguments: &Vec<u16>, result: Option<u8>) -> Frame {
        let var_count = util::byte_value(memory_map, address) as usize;
        let (initial_pc, mut local_variables) = match version {
            1 | 2 | 3 | 4 => {
                let mut local_variables = Vec::new();
                for i in 0..var_count {
                    let addr = address + 1 + (2 * i);
                    let v = util::word_value(memory_map, addr);
                    local_variables.push(v);
                }
                (address + 1 + (var_count * 2), local_variables)
            },
            _ => {
                (address + 1, vec![0 as u16; var_count])
            }
        };

        for i in 0..arguments.len() {
            local_variables[i] = arguments[i];
        }

        Frame {
            address,
            pc: initial_pc,
            local_variables,
            stack: Vec::new(),
            result
        }
    }

    pub fn pop(&mut self) -> Option<u16> {
        trace!("stack[{}]: pop #{:04x}", self.stack.len(), self.stack.last().unwrap());
        self.stack.pop()
    }

    pub fn push(&mut self, value: u16) {
        trace!("stack[{}]: push #{:04x}", self.stack.len(), value);
        self.stack.push(value);
    }
}

pub struct State {
    frames: Vec<Frame>
}

impl State {
    pub fn new(memory_map: &Vec<u8>, version: u8) -> State {
        let f = {
            match version {
                6 => {
                    let addr = (header::initial_pc(memory_map) as usize * 4) +
                                        (header::routine_offset(memory_map) as usize * 8);
                    Frame::call(memory_map, version, addr, &Vec::new(), None)
                }
                _ => {
                    let pc = header::initial_pc(memory_map) as usize;
                    Frame::initial(memory_map, pc)
                }
            }
        };

        let mut frames = Vec::new();
        frames.push(f);

        State {
            frames,
        }
    }

    pub fn call(&mut self, memory_map: &Vec<u8>, version: u8, address: usize, return_address: usize, arguments: &Vec<u16>, result: Option<u8>) -> usize {
        self.current_frame_mut().pc = return_address;
        let f = Frame::call(memory_map, version, address, arguments, result);
        self.frames.push(f);
        self.current_frame().pc
    }

    pub fn return_fn(&mut self, memory_map: &mut Vec<u8>, _version: u8, result: u16) -> usize {
        let f = self.pop_frame();
        match f.result {
            Some(variable) => self.set_variable(memory_map, variable, result),
            None => {}
        }

        self.current_frame().pc
    }

    pub fn current_frame(&self) -> &Frame {
        self.frames.last().unwrap()
    }

    pub fn pop_frame(&mut self) -> Frame {
        self.frames.pop().unwrap()
    }

    pub fn current_frame_mut(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    pub fn variable(&mut self, memory_map: &Vec<u8>, var: u8) -> u16 {
        if var == 0 {
            self.current_frame_mut().pop().unwrap()
        } else if var < 16 {
            self.current_frame().local_variables[var as usize - 1]
        } else {
            util::word_value(memory_map, header::global_variable_table(memory_map) as usize + ((var as usize - 16) * 2))
        }
    }

    pub fn set_variable(&mut self, memory_map: &mut Vec<u8>, var: u8, value: u16) {
        trace!("variable: set #{:02x} to #{:04x}", var, value);
        if var == 0 {
            self.current_frame_mut().push(value)
        } else if var < 16 {
            self.current_frame_mut().local_variables[var as usize - 1] = value
        } else {
            let address = header::global_variable_table(memory_map) as usize + ((var as usize - 16) * 2);
            util::set_word(memory_map, address, value)
        }
    }

    pub fn random(&self, range: u16) -> u16 {
        let v = rand::thread_rng().gen_range(1..=range);
        trace!("Random 1..{}: {}", range, v);
        v
    }

    pub fn seed(&mut self, seed: u64) {
        StdRng::seed_from_u64(seed as u64);
    }
}