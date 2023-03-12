pub mod header;
pub mod instruction;
pub mod log;
pub mod object;
pub mod state;
pub mod text;

use instruction::Instruction;
use state::State;

use crate::interpreter::curses_v2::CursesV2;
use crate::interpreter::Interpreter;

pub struct Executor {
    pub state: State,
}

impl Executor {
    pub fn from_vec(name: String, v: Vec<u8>) -> Executor {
        log::init(&name);
        let version = v[0];

        let interpreter = CursesV2::new(version, name);
        let spec = interpreter.spec(version);
        let mut state = State::new(&v, Box::new(interpreter));
        state.initialize(spec);
        Executor { state }
    }

    pub fn run(&mut self) {
        let mut n = 1;
        loop {
            if self.state.current_frame().pc == 0 {
                self.state.interpreter.read_char(0);
                self.state.new_line();
                pancurses::reset_shell_mode();
                pancurses::curs_set(1);
                trace!("Unimplemented instruction ... ending execution");
                panic!("Ending execution")
            }

            trace!("======> Instruction #{:04}", n);
            let mut i = Instruction::from_address(&self.state, self.state.current_frame().pc);
            i.trace_instruction(&self.state);
            self.state.current_frame_mut().pc = i.execute(&mut self.state);
            trace!("<====== Instruction #{:04}", n);
            n = n + 1;
        }
    }
}
