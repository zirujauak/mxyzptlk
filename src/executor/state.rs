use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::executor::header;

use crate::interpreter::Input;
use crate::interpreter::Interpreter;
use crate::interpreter::Spec;
use crate::quetzal::Quetzal;
use crate::quetzal::StackFrame;

#[derive(Debug)]
pub struct Frame {
    _address: usize,
    pub pc: usize,
    pub local_variables: Vec<u16>,
    pub argument_count: u8,
    pub stack: Vec<u16>,
    pub result: Option<u8>,
    pub return_address: usize,
    read_char_interrupt: bool,
    read_char_interrupt_result: u16,
    read_interrupt: bool,
    read_interrupt_result: u16,
    read_input: Vec<char>,
}

fn word(high_byte: u8, low_byte: u8) -> u16 {
    ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
}

fn word_value(memory_map: &Vec<u8>, address: usize) -> u16 {
    word(
        byte_value(memory_map, address),
        byte_value(memory_map, address + 1),
    )
}

fn byte_value(memory_map: &Vec<u8>, address: usize) -> u8 {
    memory_map[address]
}

impl Frame {
    fn from_stack_frame(frame: &StackFrame) -> Frame {
        let mut argument_count = 0;
        let mut a = frame.arguments;
        while a & 1 == 1 {
            argument_count = argument_count + 1;
            a = a >> 1;
        }

        Frame {
            _address: 0,
            pc: 0,
            local_variables: frame.local_variables.clone(),
            argument_count,
            stack: frame.stack.clone(),
            result: if frame.flags & 0x10 == 0x10 {
                None
            } else {
                Some(frame.result_variable)
            },
            return_address: frame.return_address as usize,
            read_char_interrupt: false,
            read_char_interrupt_result: 0,
            read_interrupt: false,
            read_interrupt_result: 0,
            read_input: Vec::new(),
        }
    }

    fn initial(_memory_map: &Vec<u8>, address: usize) -> Frame {
        Frame {
            _address: address,
            pc: address,
            local_variables: Vec::new(),
            argument_count: 0,
            stack: Vec::new(),
            result: None,
            return_address: 0,
            read_char_interrupt: false,
            read_char_interrupt_result: 0,
            read_interrupt: false,
            read_interrupt_result: 0,
            read_input: Vec::new(),
        }
    }

    fn call(
        memory_map: &Vec<u8>,
        version: u8,
        address: usize,
        arguments: &Vec<u16>,
        result: Option<u8>,
        return_address: usize,
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
            _address: address,
            pc: initial_pc,
            local_variables,
            argument_count: arguments.len() as u8,
            stack: Vec::new(),
            result,
            return_address,
            read_char_interrupt: false,
            read_char_interrupt_result: 0,
            read_interrupt: false,
            read_interrupt_result: 0,
            read_input: Vec::new(),
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

struct OutputStreamTable {
    address: usize,
    pub data: Vec<u8>,
}

impl OutputStreamTable {
    fn new(address: usize) -> OutputStreamTable {
        OutputStreamTable {
            address,
            data: Vec::new(),
        }
    }

    fn write(&mut self, text: String) {
        for c in text.chars() {
            if c as u8 != 0 {
                self.data.push(c as u8);
            }
        }
    }

    fn close(&mut self, state: &mut State) {
        state.set_word(self.address, self.data.len() as u16);
        for i in 0..self.data.len() {
            state.set_byte(self.address + 2 + i, self.data[i]);
        }
    }
}

pub struct State {
    memory_map: Vec<u8>,
    pristine_memory_map: Vec<u8>,
    pub version: u8,
    pub frames: Vec<Frame>,
    pub interpreter: Box<dyn Interpreter>,
    pub print_in_interrupt: bool,
    pub read_interrupt: bool,
    pub read_char_interrupt: bool,
    rng: ChaCha8Rng,
    pub random_predictable: bool,
    pub random_predictable_range: u16,
    pub random_predictable_next: u16,
    undo: Option<Quetzal>,
    stream_3: Vec<OutputStreamTable>,
    output_stream: u8,
}

impl State {
    pub fn new(memory_map: &Vec<u8>, interpreter: Box<dyn Interpreter>) -> State {
        let version = byte_value(memory_map, 0);
        let f = {
            let pc = word_value(memory_map, 0x06) as usize;
            match version {
                6 => {
                    let addr = pc * 4 + word_value(memory_map, 0x28) as usize * 8;
                    Frame::call(memory_map, version, addr, &Vec::new(), None, 0)
                }
                _ => Frame::initial(memory_map, pc),
            }
        };

        let mut frames = Vec::new();
        frames.push(f);

        State {
            memory_map: memory_map.clone(),
            pristine_memory_map: memory_map.clone(),
            version: memory_map[0],
            frames,
            interpreter,
            print_in_interrupt: false,
            read_interrupt: false,
            read_char_interrupt: false,
            rng: ChaCha8Rng::from_entropy(),
            random_predictable: false,
            random_predictable_range: 0,
            random_predictable_next: 0,
            undo: None,
            stream_3: Vec::new(),
            output_stream: 1,
        }
    }

    pub fn initialize(&mut self, spec: Spec) {
        // Set and clear flag bits
        for f in spec.set_flags {
            trace!("Setting flag {:?}", f);
            header::set_flag(self, f)
        }
        for f in spec.clear_flags {
            trace!("Clearing flag {:?}", f);
            header::clear_flag(self, f)
        }

        // Interpreter number/version
        self.set_byte(0x1E, spec.interpreter_number);
        self.set_byte(0x1F, spec.interpreter_version);

        if self.version < 5 {
            // Screen size
            self.set_byte(0x20, spec.screen_lines);
            self.set_byte(0x21, spec.screen_columns);
        }

        if self.version >= 5 {
            // Character sizing
            self.set_word(0x22, spec.screen_columns as u16);
            self.set_word(0x24, spec.screen_lines as u16);
            self.set_byte(0x26, spec.column_units);
            self.set_byte(0x27, spec.line_units);

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

        let f = Frame::call(
            &self.memory_map(),
            self.version,
            address,
            arguments,
            result,
            return_address,
        );
        self.frames.push(f);
        self.current_frame().pc
    }

    pub fn return_fn(&mut self, result: u16) -> usize {
        let mut f = self.pop_frame();

        if f.read_char_interrupt {
            f.read_char_interrupt_result = result;
        } else if f.read_interrupt {
            f.read_interrupt_result = result;
        } else {
            match f.result {
                Some(variable) => self.set_variable(variable, result),
                None => {}
            }
        }

        trace!(
            "Return to ${:05x} with result #{:04x}",
            f.return_address,
            result
        );
        f.return_address
    }

    pub fn read_char_interrupt(&self) -> bool {
        self.current_frame().read_char_interrupt
    }

    pub fn read_char_interrupt_result(&self) -> u16 {
        self.current_frame().read_char_interrupt_result
    }

    pub fn set_read_char_interrupt(&mut self, value: bool) {
        self.current_frame_mut().read_char_interrupt = value;
        self.read_char_interrupt = value
    }

    pub fn call_read_char_interrupt(&mut self, address: u16, return_addr: usize) -> usize {
        self.current_frame_mut().read_char_interrupt = true;
        self.read_char_interrupt = true;
        self.call(
            self.packed_routine_address(address),
            return_addr,
            &Vec::new(),
            None,
        )
    }

    pub fn read_interrupt(&self) -> bool {
        self.current_frame().read_interrupt
    }

    pub fn read_interrupt_result(&self) -> u16 {
        self.current_frame().read_interrupt_result
    }

    pub fn read_input(&self) -> &Vec<char> {
        &self.current_frame().read_input
    }

    pub fn set_read_interrupt(&mut self, value: bool) {
        self.current_frame_mut().read_interrupt = value;
        self.read_interrupt = value;
    }

    pub fn set_read_input(&mut self, input: &Vec<char>) {
        self.current_frame_mut().read_input = input.clone()
    }
    pub fn clear_read_input(&mut self) {
        self.current_frame_mut().read_input.clear()
    }

    pub fn call_read_interrupt(&mut self, address: u16, return_addr: usize) -> usize {
        self.current_frame_mut().read_interrupt = true;
        self.read_interrupt = true;
        self.call(
            self.packed_routine_address(address),
            return_addr,
            &Vec::new(),
            None,
        )
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

    pub fn set_variable_indirect(&mut self, var: u8, value: u16) {
        trace!("variable indirect: set #{:02x} to #{:04x}", var, value);
        if var == 0 {
            self.current_frame_mut().pop();
            self.current_frame_mut().push(value)
        } else if var < 16 {
            self.current_frame_mut().local_variables[var as usize - 1] = value
        } else {
            let address = header::global_variable_table(self) as usize + ((var as usize - 16) * 2);
            self.set_word(address, value)
        }
    }
    pub fn random(&mut self, range: u16) -> u16 {
        let v = &self.rng.gen_range(1..=range);
        trace!("Random 1..{}: {}", range, v);
        *v
    }

    pub fn seed(&mut self, seed: u64) {
        self.rng = ChaCha8Rng::seed_from_u64(seed as u64);
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

    // fn word(high_byte: u8, low_byte: u8) -> u16 {
    //     ((high_byte as u16) << 8) & 0xFF00 | (low_byte as u16) & 0xFF
    // }

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

        if address == 0x10 && value & 1 == 1 && self.output_stream & 2 == 0 {
            trace!("enabling output stream 2");
            self.output_stream = self.output_stream | 2;
            self.interpreter.output_stream(2, 0);
        } else if address == 0x10 && value & 1 == 0 && self.output_stream & 2 == 2 {
            trace!("disabling output stream 2");
            self.output_stream = self.output_stream & 0xFD;
            self.interpreter.output_stream(-2, 0);
        }
        trace!("memory: set ${:05x} to #{:04x}", address, value)
    }

    pub fn set_byte(&mut self, address: usize, value: u8) {
        self.memory_map[address] = value;

        trace!("memory: set ${:05x} to #{:02x}", address, value);
    }

    pub fn checksum(&self) -> u16 {
        let mut checksum = 0 as u16;
        let size = header::length(self) as usize
            * match self.version {
                1 | 2 | 3 => 2,
                4 | 5 => 4,
                6 | 7 | 8 => 8,
                _ => 0,
            };
        for i in 0x40..size {
            checksum = u16::overflowing_add(checksum, self.pristine_memory_map[i] as u16).0;
        }

        checksum as u16
    }

    pub fn prepare_save(&self, address: usize) -> Quetzal {
        let q = Quetzal::from_state(self, address);
        trace!(
            "Quetzal: {:05x} bytes, {} stack frames",
            q.umem.data.len(),
            q.stks.stks.len()
        );
        q
    }

    pub fn restore_file(&mut self) -> Quetzal {
        let data = self.interpreter.restore();
        Quetzal::from_vec(data)
    }

    pub fn prepare_restore(&mut self, data: &Quetzal) -> usize {
        trace!(
            "Quetzal: {:05x} bytes, {} stack frames",
            data.umem.data.len(),
            data.stks.stks.len()
        );

        // TODO: Verify IFhd metadata
        // Replace dynamic memory
        let static_address = header::static_memory_base(self) as usize;
        let mut static_memory = self.memory_map[static_address..].to_vec();
        self.memory_map = data.umem.data.clone();
        self.memory_map.append(&mut static_memory);

        // Rebuild frame stack
        self.frames.clear();
        for f in &data.stks.stks {
            let frame = Frame::from_stack_frame(f);
            self.frames.push(frame);
        }

        data.ifhd.pc as usize
    }

    pub fn save_undo(&mut self, q: Quetzal) {
        self.undo = Some(q)
    }

    pub fn undo(&self) -> Option<&Quetzal> {
        self.undo.as_ref()
    }

    pub fn restore_undo(&mut self) -> Option<usize> {
        let u = self.undo.as_ref().unwrap();
        trace!(
            "Quetzal: {:05x} bytes, {} stack frames",
            u.umem.data.len(),
            u.stks.stks.len()
        );

        // TODO: Verify IFhd metadata
        // Replace dynamic memory
        let static_address = header::static_memory_base(self) as usize;
        let mut static_memory = self.memory_map[static_address..].to_vec();
        self.memory_map = u.umem.data.clone();
        self.memory_map.append(&mut static_memory);

        // Rebuild frame stack
        self.frames.clear();
        for f in &u.stks.stks {
            let frame = Frame::from_stack_frame(f);
            self.frames.push(frame);
        }

        Some(u.ifhd.pc as usize)
    }

    pub fn print_in_interrupt(&mut self) {
        self.print_in_interrupt =
            self.print_in_interrupt || self.read_char_interrupt || self.read_interrupt
    }

    fn stream_3_mut(&mut self) -> &mut OutputStreamTable {
        self.stream_3.last_mut().unwrap()
    }
}

impl Interpreter for State {
    fn buffer_mode(&mut self, mode: bool) {
        self.interpreter.buffer_mode(mode);
    }
    fn erase_line(&mut self, value: u16) {
        self.interpreter.erase_line(value);
        self.print_in_interrupt()
    }
    fn erase_window(&mut self, window: i16) {
        self.interpreter.erase_window(window);
        self.print_in_interrupt()
    }
    fn get_cursor(&mut self) -> (u16, u16) {
        self.interpreter.get_cursor()
    }
    fn input_stream(&mut self, stream: u16) {
        self.interpreter.input_stream(stream)
    }
    fn new_line(&mut self) {
        trace!(target: "app::transcript", "\n");
        self.interpreter.new_line();
        self.print_in_interrupt()
    }
    fn output_stream(&mut self, stream: i16, table: usize) {
        if stream < 0 && stream > -3 {
            trace!("Disable output stream {}", stream.abs());
            let bits = stream.abs() - 1;
            let mask = !((1 as u8) << bits);
            self.output_stream = self.output_stream & mask;
        } else if stream > 0 && stream < 3 {
            trace!("Enable output stream {}", stream.abs());
            let bits = stream - 1;
            let mask = (1 as u8) << bits;
            self.output_stream = self.output_stream | mask;
        }

        if stream == 2 {
            trace!("Flags1: {:#02x}", self.byte_value(1));
            self.set_byte(0x01, self.byte_value(0x01) | 0x1);
        } else if stream == -2 {
            trace!("Flags1: {:#02x}", self.byte_value(1));
            self.set_byte(0x01, self.byte_value(0x01) & 0xFE);
        } else if stream == -3 {
            let s = self.stream_3.pop();
            match s {
                Some(mut st) => {
                    trace!(
                        "Closing output stream 3 @ {:#06x} [{:#04x}]",
                        st.address,
                        st.data.len()
                    );
                    st.close(self)
                }
                None => {}
            }
            if !self.stream_3.is_empty() {
                trace!(
                    "Output stream 3 @ {:#06x} reactivated",
                    self.stream_3.last().unwrap().address
                );
            } else {
                self.output_stream = self.output_stream & 0xB;
            }
        } else if stream == 3 {
            trace!("Output stream 3 opened @ {:#06x}", table);
            let s = OutputStreamTable::new(table);
            self.output_stream = self.output_stream | 4;
            self.stream_3.push(s);
        }

        self.interpreter.output_stream(stream, table);
    }

    fn print(&mut self, text: String) {
        trace!(target: "app::transcript", "[{:#04b}] {}", self.output_stream, text);
        if self.output_stream & 0x4 == 0x4 {
            trace!(
                "Printing to stream 3 {:?} @ {}: {}",
                text.as_bytes(),
                self.stream_3_mut().data.len(),
                text
            );
            self.stream_3_mut().write(text);
        } else {
            self.interpreter.print(text);
            self.print_in_interrupt()
        }
    }

    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        self.interpreter.print_table(text, width, height, skip);
        self.print_in_interrupt()
    }
    fn read(
        &mut self,
        length: u8,
        time: u16,
        existing_input: &Vec<char>,
        redraw: bool,
    ) -> (Vec<char>, bool) {
        self.interpreter.read(length, time, existing_input, redraw)
    }
    fn read_char(&mut self, time: u16) -> Input {
        let input = self.interpreter.read_char(time);
        match input.zscii_value as u8 {
            253 | 254 => {
                header::set_extension_word(self, 0, input.x);
                header::set_extension_word(self, 1, input.y);
            }
            _ => {}
        }

        input
    }
    fn set_colour(&mut self, foreground: u16, background: u16) {
        self.interpreter.set_colour(foreground, background)
    }
    fn set_cursor(&mut self, line: u16, column: u16) {
        self.interpreter.set_cursor(line, column);
    }
    fn set_font(&mut self, font: u16) {
        self.interpreter.set_font(font);
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
        self.interpreter
            .sound_effect(number, effect, volume, repeats)
    }
    fn split_window(&mut self, lines: u16) {
        self.interpreter.split_window(lines);
    }
    fn save(&mut self, data: &Vec<u8>) {
        self.interpreter.save(data);
    }
    fn restore(&mut self) -> Vec<u8> {
        self.interpreter.restore()
    }
}
