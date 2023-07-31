use pancurses::*;

use crate::zmachine::io::screen::{CellStyle, Color, InputEvent, Style, Terminal};

pub struct PCTerminal {
    window: Window,
}

fn cp(fg: i16, bg: i16) -> i16 {
    // color range 0-7, so 3 bits each
    // color pair index is 6 bits, 00ff fbbb + 1
    // pairs 1 - 64 are used by the basic colors, leaving 191 for "true" colors
    ((fg << 3) & 0x38) + (bg & 0x07) + 1
}

pub fn new_terminal() -> Box<dyn Terminal> {
    Box::new(PCTerminal::new())
}

impl PCTerminal {
    pub fn new() -> PCTerminal {
        info!(target: "app::screen", "Initialize pancurses terminal");
        let window = pancurses::initscr();
        pancurses::curs_set(0);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
        pancurses::mousemask(ALL_MOUSE_EVENTS, None);
        pancurses::set_title("mxyzptlk - a rusty z-machine interpreter");

        window.keypad(true);
        window.clear();
        window.refresh();

        // Initialize fg/bg color pairs
        for fg in 0..8 {
            for bg in 0..8 {
                pancurses::init_pair(cp(fg, bg), fg, bg);
            }
        }

        PCTerminal { window }
    }

    fn as_color(&self, color: Color) -> i16 {
        match color {
            Color::Black => COLOR_BLACK,
            Color::Red => COLOR_RED,
            Color::Green => COLOR_GREEN,
            Color::Yellow => COLOR_YELLOW,
            Color::Blue => COLOR_BLUE,
            Color::Magenta => COLOR_MAGENTA,
            Color::Cyan => COLOR_CYAN,
            Color::White => COLOR_WHITE,
        }
    }

    fn input_to_u16(&self, input: Input) -> InputEvent {
        match input {
            Input::Character(c) => super::char_to_u16(c),
            Input::KeyUp => InputEvent::from_char(129),
            Input::KeyDown => InputEvent::from_char(130),
            Input::KeyLeft => InputEvent::from_char(131),
            Input::KeyRight => InputEvent::from_char(132),
            Input::KeyF1 => InputEvent::from_char(133),
            Input::KeyF2 => InputEvent::from_char(134),
            Input::KeyF3 => InputEvent::from_char(135),
            Input::KeyF4 => InputEvent::from_char(136),
            Input::KeyF5 => InputEvent::from_char(137),
            Input::KeyF6 => InputEvent::from_char(138),
            Input::KeyF7 => InputEvent::from_char(139),
            Input::KeyF8 => InputEvent::from_char(140),
            Input::KeyF9 => InputEvent::from_char(141),
            Input::KeyF10 => InputEvent::from_char(142),
            Input::KeyF11 => InputEvent::from_char(143),
            Input::KeyF12 => InputEvent::from_char(144),
            Input::KeyBackspace => InputEvent::from_char(8),
            Input::KeyMouse => match pancurses::getmouse() {
                Ok(event) => {
                    if event.bstate & BUTTON1_CLICKED == BUTTON1_CLICKED {
                        InputEvent::from_mouse(254, event.y as u16 + 1, event.x as u16 + 1)
                    } else if event.bstate & BUTTON1_DOUBLE_CLICKED == BUTTON1_DOUBLE_CLICKED {
                        InputEvent::from_mouse(253, event.y as u16 + 1, event.x as u16 + 1)
                    } else {
                        InputEvent::no_input()
                    }
                }
                Err(e) => {
                    warn!(target: "app::screen", "Error reading mouse event: {}", e);
                    InputEvent::no_input()
                }
            },
            _ => {
                info!(target: "app::screen", "Unprocssed input: {:?}", input);
                InputEvent::no_input()
            }
        }
    }
}

impl Terminal for PCTerminal {
    fn type_name(&self) -> &str {
        "PCTerminal"
    }

    fn size(&self) -> (u32, u32) {
        let (rows, columns) = self.window.get_max_yx();
        (rows as u32, columns as u32)
    }

    fn print_at(
        &mut self,
        zchar: u16,
        row: u32,
        column: u32,
        colors: (Color, Color),
        style: &CellStyle,
        font: u8,
    ) {
        let c = super::map_output(zchar, font);
        let cp = cp(self.as_color(colors.0), self.as_color(colors.1));
        let mut attributes = 0;
        if style.is_style(Style::Bold) {
            attributes |= A_BOLD;
        }
        if style.is_style(Style::Italic) {
            if cfg!(target_os = "macos") {
                attributes |= A_UNDERLINE;
            } else {
                attributes |= A_ITALIC;
            }
        }
        if style.is_style(Style::Reverse) {
            attributes |= A_REVERSE;
        }
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.addstr(format!("{}", c));
        self.window.mv(row as i32 - 1, column as i32 - 1);
        self.window.chgat(1, attributes, cp);
    }

    fn flush(&mut self) {
        self.window.refresh();
    }

    fn read_key(&mut self, wait: bool) -> InputEvent {
        if wait {
            self.window.nodelay(false);
        } else {
            self.window.nodelay(true);
        }
        pancurses::curs_set(1);
        pancurses::raw();

        if let Some(i) = self.window.getch() {
            pancurses::curs_set(0);
            self.input_to_u16(i)
        } else {
            InputEvent::no_input()
        }
    }

    fn scroll(&mut self, row: u32) {
        self.window.mv(row as i32 - 1, 0);
        self.window.insdelln(-1);
        // self.window.deleteln();
        // let curs = self.window.get_max_yx();
        // self.window.mv(curs.0 - 1, 0);
        // self.window.deleteln();
        self.window.refresh();
    }

    fn backspace(&mut self, at: (u32, u32)) {
        let attributes = self.window.mvinch(at.0 as i32 - 1, at.1 as i32 - 1);
        let ch = (attributes & 0xFFFFFF00) | 0x20;
        self.window.mvaddch(at.0 as i32 - 1, at.1 as i32 - 1, ch);
        self.window.mv(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn beep(&mut self) {
        pancurses::beep();
    }

    fn move_cursor(&mut self, at: (u32, u32)) {
        self.window.mv(at.0 as i32 - 1, at.1 as i32 - 1);
    }

    fn reset(&mut self) {
        self.window.clear();
    }

    fn quit(&mut self) {
        info!(target: "app::screen", "Closing pancurses terminal");
        self.window.keypad(false);
        pancurses::curs_set(2);
        pancurses::mousemask(0, None);
        pancurses::endwin();
        pancurses::doupdate();
        pancurses::reset_prog_mode();
    }

    fn set_colors(&mut self, colors: (Color, Color)) {
        let cp = cp(self.as_color(colors.0), self.as_color(colors.1));
        self.window.color_set(cp);
    }

    fn error(&mut self, instruction: &str, message: &str, recoverable: bool) -> bool {
        let (rows, cols) = self.window.get_max_yx();
        let height = 7;
        let prompt_str = "Press 'c' to continue or any other key to exit";
        let width = usize::max(
            prompt_str.len(),
            usize::max(instruction.len(), message.len()),
        ) as i32
            + 8;
        let err_row = (rows - height) / 2;
        let err_col = (cols - width) / 2;

        let errwin = pancurses::newwin(height, width, err_row, err_col);
        errwin.draw_box(0, 0);
        errwin.mv(1, 2);
        errwin.addstr(message);
        errwin.mv(3, 2);
        errwin.addstr(instruction);
        errwin.mv(5, 2);
        errwin.addstr(prompt_str);
        errwin.refresh();
        errwin.nodelay(false);
        pancurses::flushinp();
        loop {
            if let Some(ch) = errwin.getch() {
                errwin.delwin();
                self.window.touch();
                self.window.refresh();

                if recoverable && (ch == Input::Character('c') || ch == Input::Character('C')) {
                    return true;
                }

                return false;
            }
        }
    }
}
