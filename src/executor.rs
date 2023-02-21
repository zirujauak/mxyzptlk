pub mod header;
pub mod state;
pub mod instruction;
pub mod log;
pub mod object;
pub mod text;
pub mod event;
pub mod interpreter;

use pancurses::{Window, Input};
use state::State;
use instruction::Instruction;

use self::interpreter::Interpreter;

pub struct Executor {
    state: State,
}

impl Executor {
    pub fn from_vec(v: Vec<u8>) -> Executor {
        let version = v[0];

        let interpreter = Curses::new(version);
        let state = State::new(&v, Box::new(interpreter));
        Executor { 
            state,
        }
    }

    pub fn run(&mut self) {
        let mut n = 1;
        loop {
            if self.state.current_frame().pc == 0 {
                self.state.interpreter.read_char(0);
                self.state.new_line();
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

struct Curses {
    version: u8,
    window: Window,
    lines: u16,
    columns: u16,
    selected_window: u8,
    split_line: Option<u16>,
    cursor_position: Vec<(u16,u16)>,
    output_streams: Vec<bool>,
    buffering: bool
}

impl Curses {
    fn new(version: u8) -> Curses {
        let mut window = pancurses::initscr();
        window.resize(24, 80);
        window.erase();

        let cursor:(u16,u16) = if version < 5 {
            (24, 1)
        } else {
            (1,1)
        };

        let output_streams = if version < 3 {
            vec![true, false]
        } else {
            vec![true, false, false, false]
        };

        let cursor_position = vec![cursor, (1,1)];

        Self {
            version,
            window: pancurses::initscr(),
            lines: 24,
            columns: 80,
            selected_window: 0,
            split_line: None,
            cursor_position,
            output_streams,
            buffering: true
        }
    }
}

impl Interpreter for Curses {
    fn buffer_mode(&mut self, mode: bool) {
        self.buffering = mode
    }

    fn erase_line(&mut self, value: u16) {
        todo!()
    }

    fn erase_window(&mut self, window: i16) {
        match window {
                -1 => {
                self.selected_window = 0;
                self.split_line = None;
                self.window.erase();
            },
            -2 => {
                self.window.erase();
            }
            _ => {
                trace!("TODO: ERASE_WINDOW {}", window)
            }
        }
    }

    fn get_cursor(&mut self) -> (u16, u16) {
        self.cursor_position[self.selected_window as usize]
    }

    fn input_stream(&mut self, stream: u16) {
        todo!()
    }
    fn new_line(&mut self) {
        self.window.addch('\n');
    }
    fn output_stream(&mut self, stream: i16, table: usize) {
        let stream_index = stream.abs() as usize - 1;
        self.output_streams[stream_index] = stream > 0;
    }
    fn print(&mut self, text: String) {
        if self.output_streams[0] {
            if self.buffering {
                // Split the text string on spaces
                let frags = text.split_inclusive(&[' ']);
                // Iterate over the fragments
                for s in frags {
                    let position = (self.window.get_cur_y() + 1, self.window.get_cur_x() + 1);
                    trace!("buffer check: {} {}", position.1, s.len());
                    if self.columns as i32 - position.1 < s.len() as i32 {
                        self.window.addch('\n');
                        self.window.addstr(s);
                    } else {
                        self.window.addstr(s);
                    }
                }
            } else {
                self.window.addstr(text);
            }
            self.window.refresh();
            self.cursor_position[self.selected_window as usize] = (self.window.get_cur_y() as u16 + 1, self.window.get_cur_x() as u16 + 1);
        }

        trace!("cursor: {},{}", self.cursor_position[0].0, self.cursor_position[0].1)
    }

    fn print_table(&mut self, text: String, width: u16, height: u16, skip: u16) {
        todo!()
    }

    fn read(&mut self, length: u8, time: u16) -> String {
        todo!()
    }

    fn read_char(&mut self, time: u16) -> char {
        pancurses::noecho();
        match self.window.getch().unwrap() {
            Input::Character(c) => c,
            _ => ' '
        }
    }

    fn set_colour(&mut self, foreground: u16, background: u16) {
        todo!()
    }

    fn set_cursor(&mut self, line: u16, column: u16) {
        self.window.mv(line as i32 - 1, column as i32 - 1);
    }

    fn set_text_style(&mut self, style: u16) {
        todo!()
    }

    fn set_window(&mut self, window: u16) {
        todo!()
    }

    fn sound_effect(&mut self, number: u16, effect: u16, volume: u8, repeats: u8) {
        todo!()
    }

    fn split_window(&mut self, lines: u16) {
        todo!()
    }
}