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
        log_mdc::insert("instruction_count", "0");
        let version = v[0];

        let interpreter = CursesV2::new(version, name);
        let mut state = State::new(&v, Box::new(interpreter));
        state.initialize();
        Executor { state }
    }

    fn log_stack(&self) {
        let mut s = String::new();
        s.push_str("[");
        if self.state.current_frame().stack.len() > 0 {
            s.push_str(&format!("{:04x}", self.state.current_frame().stack[0]));
        }
        for i in 1..self.state.current_frame().stack.len() {
            s.push_str(&format!(",{:04x}", self.state.current_frame().stack[i]))
        }

        info!(target: "app::stack", "{}]", s.trim())
    }

    fn log_local_vars(&self) {
        let mut s = String::new();
        s.push_str("Local vars: [");
        if self.state.current_frame().local_variables.len() > 0 {
            s.push_str(&format!(
                "#{:04x}",
                self.state.current_frame().local_variables[0]
            ));
        }
        for i in 1..self.state.current_frame().local_variables.len() {
            s.push_str(&format!(
                ",#{:04x}",
                self.state.current_frame().local_variables[i]
            ));
        }

        info!(target: "app::variable", "{}]", s.trim())
    }

    fn log_global_vars(&self) {
        let address = header::global_variable_table(&self.state) as usize;
        for i in (0..240 as usize).step_by(16) {
            let mut s = String::new();
            s.push_str(&format!(
                "[{:04x}",
                self.state.word_value(address + (i * 2))
            ));
            for j in 1..16 {
                s.push_str(&format!(
                    ",{:04x}",
                    self.state.word_value(address + ((i + j) * 2))
                ));
            }
            info!(target: "app::variable", "{}]", s)
        }
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

            log_mdc::insert("instruction_count", n.to_string());
            self.log_stack();
            self.log_local_vars();
            self.log_global_vars();
            let mut i = Instruction::from_address(&self.state, self.state.current_frame().pc);
            i.trace_instruction(&self.state);
            self.state.current_frame_mut().pc = i.execute(&mut self.state);
            n = n + 1;
        }
    }
}
