use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::executor::header;

use crate::interpreter::Interpreter;
use crate::interpreter::Spec;

#[derive(Debug)]
pub struct Frame {
    address: usize,
    pub pc: usize,
    local_variables: Vec<u16>,
    stack: Vec<u16>,
    result: Option<u8>,
}

fn word(high_byte: u8, low_byte: u8) -> u16 {
    ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
}

fn word_value(memory_map: &Vec<u8>, address: usize) -> u16 {
    State::word(
        byte_value(memory_map, address),
        byte_value(memory_map, address + 1),
    )
}

fn byte_value(memory_map: &Vec<u8>, address: usize) -> u8 {
    memory_map[address]
}

impl Frame {
    fn initial(_memory_map: &Vec<u8>, address: usize) -> Frame {
        Frame {
            address,
            pc: address,
            local_variables: Vec::new(),
            stack: Vec::new(),
            result: None,
        }
    }

    fn call(
        memory_map: &Vec<u8>,
        version: u8,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<u8>,
    ) -> Frame {
        let var_count = byte_value(memory_map, address) as usize;
        trace!("{} local variables", var_count);
        let (initial_pc, mut local_variables) = match version {
            1 | 2 | 3 | 4 => {
                let mut local_variables = Vec::new();
                for i in 0..var_count {
                    let addr = address + 1 + (2 * i);
                    let v = word_value(memory_map, addr);
                    local_variables.push(v);
                }
                (address + 1 + (var_count * 2), local_variables)
            }
            _ => (address + 1, vec![0 as u16; var_count]),
        };

        for i in 0..arguments.len() {
            if local_variables.len() > i {
                local_variables[i] = arguments[i];
            }
        }

        Frame {
            address,
            pc: initial_pc,
            local_variables,
            stack: Vec::new(),
            result,
        }
    }

    pub fn pop(&mut self) -> Option<u16> {
        trace!(
            "stack[{}]: pop -> #{:04x}",
            self.stack.len(),
            self.stack.last().unwrap()
        );
        self.stack.pop()
    }

    pub fn peek(&self) -> Option<&u16> {
        self.stack.last()
    }

    pub fn push(&mut self, value: u16) {
        trace!("stack[{}]: push <- #{:04x}", self.stack.len(), value);
        self.stack.push(value);
    }
}

pub struct State {
    memory_map: Vec<u8>,
    pub version: u8,
    frames: Vec<Frame>,
    pub interpreter: Box<dyn Interpreter>,
}

impl State {
    pub fn new(memory_map: &Vec<u8>, interpreter: Box<dyn Interpreter>) -> State {
        let version = byte_value(memory_map, 0);
        let f = {
            let pc = word_value(memory_map, 0x06) as usize;
            match version {
                6 => {
                    let addr = pc * 4 + word_value(memory_map, 0x28) as usize * 8;
                    Frame::call(memory_map, version, addr, &Vec::new(), None)
                }
                _ => Frame::initial(memory_map, pc),
            }
        };

        let mut frames = Vec::new();
        frames.push(f);

        State {
            memory_map: memory_map.clone(),
            version: memory_map[0],
            frames,
            interpreter,
        }
    }

    pub fn initialize(&mut self, spec: Spec) {
        // Set and clear flag bits
        for f in spec.set_flags {
            header::set_flag(self, f)
        }
        for f in spec.clear_flags {
            header::clear_flag(self, f)
        }

        // Interpreter number/version
        self.set_byte(0x1E, spec.interpreter_number);
        self.set_byte(0x1F, spec.interpreter_version);

        // Screen size
        self.set_byte(0x20, spec.screen_lines);
        self.set_byte(0x21, spec.screen_columns);

        if self.version >= 5 {
            // Character sizing
            self.set_byte(0x22, spec.column_units);
            self.set_byte(0x23, spec.line_units);

            // Default colours
            self.set_byte(0x2C, spec.background_color);
            self.set_byte(0x2D, spec.foreground_color);
        }

        // Specification
        self.set_byte(0x32, 1);
        self.set_byte(0x33, 1);
    }

    pub fn memory_map(&self) -> &Vec<u8> {
        &self.memory_map
    }

    pub fn memory_map_mut(&mut self) -> &mut Vec<u8> {
        self.memory_map.as_mut()
    }

    pub fn call(
        &mut self,
        address: usize,
        return_address: usize,
        arguments: &Vec<u16>,
        result: Option<u8>,
    ) -> usize {
        trace!(
            "Call routine @ ${:05x} with {} args",
            address,
            arguments.len()
        );

        self.current_frame_mut().pc = return_address;
        let f = Frame::call(&self.memory_map(), self.version, address, arguments, result);
        self.frames.push(f);
        self.current_frame().pc
    }

    pub fn return_fn(&mut self, result: u16) -> usize {
        let f = self.pop_frame();
        match f.result {
            Some(variable) => self.set_variable(variable, result),
            None => {}
        }

        trace!("Return to ${:05x} with result #{:04x}", self.current_frame().pc, result);
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

    pub fn variable(&mut self, var: u8) -> u16 {
        if var == 0 {
            self.current_frame_mut().pop().unwrap()
        } else if var < 16 {
            self.current_frame().local_variables[var as usize - 1]
        } else {
            self.word_value(
                header::global_variable_table(self) as usize + ((var as usize - 16) * 2),
            )
        }
    }

    pub fn peek_variable(&self, var: u8) -> u16 {
        if var == 0 {
            *self.current_frame().peek().unwrap()
        } else if var < 16 {
            self.current_frame().local_variables[var as usize - 1]
        } else {
            self.word_value(
                header::global_variable_table(self) as usize + ((var as usize - 16) * 2),
            )
        }
    }

    pub fn set_variable(&mut self, var: u8, value: u16) {
        trace!("variable: set #{:02x} to #{:04x}", var, value);
        if var == 0 {
            self.current_frame_mut().push(value)
        } else if var < 16 {
            self.current_frame_mut().local_variables[var as usize - 1] = value
        } else {
            let address = header::global_variable_table(self) as usize + ((var as usize - 16) * 2);
            self.set_word(address, value)
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

    pub fn packed_routine_address(&self, address: u16) -> usize {
        match self.version {
            1 | 2 | 3 => address as usize * 2,
            4 | 5 => address as usize * 4,
            6 | 7 => (address as usize * 4) + (header::routine_offset(self) as usize * 8),
            8 => address as usize * 8,
            // TODO: error
            _ => 0,
        }
    }

    pub fn packed_string_address(&self, address: u16) -> usize {
        match self.version {
            1 | 2 | 3 => address as usize * 2,
            4 | 5 => address as usize * 4,
            6 | 7 => (address as usize * 4) + (header::strings_offset(self) as usize * 8),
            8 => address as usize * 8,
            // TODO: error
            _ => 0,
        }
    }

    fn word(high_byte: u8, low_byte: u8) -> u16 {
        ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
    }

    pub fn word_value(&self, address: usize) -> u16 {
        word(self.byte_value(address), self.byte_value(address + 1))
    }

    pub fn byte_value(&self, address: usize) -> u8 {
        self.memory_map[address]
    }

    pub fn set_word(&mut self, address: usize, value: u16) {
        let hb = ((value >> 8) & 0xFF) as u8;
        let lb = (value & 0xFF) as u8;

        self.memory_map[address] = hb;
        self.memory_map[address + 1] = lb;

        debug!("memory: set ${:05x} to #{:04x}", address, value)
    }

    pub fn set_byte(&mut self, address: usize, value: u8) {
        self.memory_map[address] = value;

        debug!("memory: set ${:05x} to #{:02x}", address, value)
    }
}

impl Interpreter for State {
    fn buffer_mode(&mut self, mode: bool) {
        self.interpreter.buffer_mode(mode);
    }
    fn erase_line(&mut self, value: u16) {
        self.interpreter.erase_line(value);
    }
    fn erase_window(&mut self, window: i16) {
        self.interpreter.erase_window(window);
    }
    fn get_cursor(&mut self) -> (u16, u16) {
        self.interpreter.get_cursor()
    }
    fn input_stream(&mut self, stream: u16) {
        self.interpreter.input_stream(stream)
    }
    fn new_line(&mut self) {
        self.interpreter.new_line();
    }
    fn output_stream(&mut self, stream: i16, table: usize) {
        self.interpreter.output_stream(stream, table);
    }
    fn print(&mut self, text: String) {
        self.interpreter.print(text)
    }
    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        self.interpreter.print_table(text, width, height, skip);
    }
    fn read(&mut self, length: u8, time: u16) -> Vec<char> {
        self.interpreter.read(length, time)
    }
    fn read_char(&mut self, time: u16) -> char {
        self.interpreter.read_char(time)
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        self.interpreter.set_colour(foreground, background)
    }
    fn set_cursor(&mut self, line: u16, column: u16) {
        self.interpreter.set_cursor(line, column);
    }
    fn set_text_style(&mut self, style: u16) {
        self.interpreter.set_text_style(style);
    }
    fn set_window(&mut self, window: u16) {
        self.interpreter.set_window(window);
    }
    fn show_status(&mut self, location: &str, status: &str) {
        self.interpreter.show_status(location, status)
    }
    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        self.interpreter.sound_effect(number, effect, volume, repeats)
    }
    fn split_window(&mut self, lines: u16) {
        self.interpreter.split_window(lines);
    }
}
