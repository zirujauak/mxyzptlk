pub mod header;
pub mod util;
pub mod state;
pub mod instruction;
pub mod log;
pub mod object;
pub mod text;

use state::State;
use instruction::Instruction;

pub struct Executor {
    pub memory_map: Vec<u8>,
    pub state: State
}

impl Executor {
    pub fn from_vec(v: Vec<u8>) -> Executor {
        let version = header::version(&v);

        let state = State::new(&v, version);
        Executor { 
            memory_map: v,
            state }
    }

    pub fn current_pc(&self) -> usize {
        self.state.current_frame().pc
    }

    pub fn instruction(&self) -> Instruction {
        Instruction::from_address(&self.memory_map, header::version(&self.memory_map), self.current_pc())
    }

    pub fn run(&mut self) {
        let version = header::version(&self.memory_map);
        let mut n = 1;
        while true {
            if self.current_pc() == 0 {
                panic!("Ending execution")
            }

            trace!("======> Instruction #{:04}", n);
            let mut i = Instruction::from_address(&self.memory_map, version, self.current_pc());
            trace!("{}", i);
            self.state.current_frame_mut().pc = i.execute(&mut self.memory_map, version, &mut self.state);
            trace!("<====== Instruction #{:04}", n);
            n = n + 1;
        }
    }
}