use super::InputEvent;

pub mod easy_curses;
pub mod pancurses;

fn char_to_u16(c: char) -> InputEvent {
    match c {
        '\u{7f}' | '\u{08}' => InputEvent::from_char(0x08),
        '\u{0a}' | '\u{0d}' => InputEvent::from_char(0x0d),
        ' '..='~' => InputEvent::from_char(c as u16),
        '\u{e4}' => InputEvent::from_char(0x9b),
        '\u{f6}' => InputEvent::from_char(0x9c),
        '\u{fc}' => InputEvent::from_char(0x9d),
        '\u{c4}' => InputEvent::from_char(0x9e),
        '\u{d6}' => InputEvent::from_char(0x9f),
        '\u{dc}' => InputEvent::from_char(0xa0),
        '\u{df}' => InputEvent::from_char(0xa1),
        '\u{bb}' => InputEvent::from_char(0xa2),
        '\u{ab}' => InputEvent::from_char(0xa3),
        '\u{eb}' => InputEvent::from_char(0xa4),
        '\u{ef}' => InputEvent::from_char(0xa5),
        '\u{ff}' => InputEvent::from_char(0xa6),
        '\u{cb}' => InputEvent::from_char(0xa7),
        '\u{cf}' => InputEvent::from_char(0xa8),
        '\u{e1}' => InputEvent::from_char(0xa9),
        '\u{e9}' => InputEvent::from_char(0xaa),
        '\u{ed}' => InputEvent::from_char(0xab),
        '\u{f3}' => InputEvent::from_char(0xac),
        '\u{fa}' => InputEvent::from_char(0xad),
        '\u{fd}' => InputEvent::from_char(0xae),
        '\u{c1}' => InputEvent::from_char(0xaf),
        '\u{c9}' => InputEvent::from_char(0xb0),
        '\u{cd}' => InputEvent::from_char(0xb1),
        '\u{d3}' => InputEvent::from_char(0xb2),
        '\u{da}' => InputEvent::from_char(0xb3),
        '\u{dd}' => InputEvent::from_char(0xb4),
        '\u{e0}' => InputEvent::from_char(0xb5),
        '\u{e8}' => InputEvent::from_char(0xb6),
        '\u{ec}' => InputEvent::from_char(0xb7),
        '\u{f2}' => InputEvent::from_char(0xb8),
        '\u{f9}' => InputEvent::from_char(0xb9),
        '\u{c0}' => InputEvent::from_char(0xba),
        '\u{c8}' => InputEvent::from_char(0xbb),
        '\u{cc}' => InputEvent::from_char(0xbc),
        '\u{d2}' => InputEvent::from_char(0xbd),
        '\u{d9}' => InputEvent::from_char(0xbe),
        '\u{e2}' => InputEvent::from_char(0xbf),
        '\u{ea}' => InputEvent::from_char(0xc0),
        '\u{ee}' => InputEvent::from_char(0xc1),
        '\u{f4}' => InputEvent::from_char(0xc2),
        '\u{fb}' => InputEvent::from_char(0xc3),
        '\u{c2}' => InputEvent::from_char(0xc4),
        '\u{ca}' => InputEvent::from_char(0xc5),
        '\u{ce}' => InputEvent::from_char(0xc6),
        '\u{d4}' => InputEvent::from_char(0xc7),
        '\u{db}' => InputEvent::from_char(0xc8),
        '\u{e5}' => InputEvent::from_char(0xc9),
        '\u{c5}' => InputEvent::from_char(0xca),
        '\u{f8}' => InputEvent::from_char(0xcb),
        '\u{d8}' => InputEvent::from_char(0xcc),
        '\u{e3}' => InputEvent::from_char(0xcd),
        '\u{f1}' => InputEvent::from_char(0xce),
        '\u{f5}' => InputEvent::from_char(0xcf),
        '\u{c3}' => InputEvent::from_char(0xd0),
        '\u{d1}' => InputEvent::from_char(0xd1),
        '\u{d5}' => InputEvent::from_char(0xd2),
        '\u{e6}' => InputEvent::from_char(0xd3),
        '\u{c6}' => InputEvent::from_char(0xd4),
        '\u{e7}' => InputEvent::from_char(0xd5),
        '\u{c7}' => InputEvent::from_char(0xd6),
        '\u{fe}' => InputEvent::from_char(0xd7),
        '\u{f0}' => InputEvent::from_char(0xd8),
        '\u{de}' => InputEvent::from_char(0xd9),
        '\u{d0}' => InputEvent::from_char(0xda),
        '\u{a3}' => InputEvent::from_char(0xdb),
        '\u{153}' => InputEvent::from_char(0xdc),
        '\u{152}' => InputEvent::from_char(0xdd),
        '\u{a1}' => InputEvent::from_char(0xde),
        '\u{bf}' => InputEvent::from_char(0xdf),
        _ => {
            error!(target: "app::input", "Unmapped input {:02x}", c as u8);
            InputEvent::no_input()
        },
    }
}

fn map_output(zchar: u16, font: u8) -> char {
    match font {
        1 | 4 => match zchar {
            0x18 => '\u{2191}',
            0x19 => '\u{2193}',
            0x1a => '\u{2192}',
            0x1b => '\u{2190}',
            0x9b => '\u{e4}',
            0x9c => '\u{f6}',
            0x9d => '\u{fc}',
            0x9e => '\u{c4}',
            0x9f => '\u{d6}',
            0xa0 => '\u{dc}',
            0xa1 => '\u{df}',
            0xa2 => '\u{bb}',
            0xa3 => '\u{ab}',
            0xa4 => '\u{eb}',
            0xa5 => '\u{ef}',
            0xa6 => '\u{ff}',
            0xa7 => '\u{cb}',
            0xa8 => '\u{cf}',
            0xa9 => '\u{e1}',
            0xaa => '\u{e9}',
            0xab => '\u{ed}',
            0xac => '\u{f3}',
            0xad => '\u{fa}',
            0xae => '\u{fd}',
            0xaf => '\u{c1}',
            0xb0 => '\u{c9}',
            0xb1 => '\u{cd}',
            0xb2 => '\u{d3}',
            0xb3 => '\u{da}',
            0xb4 => '\u{dd}',
            0xb5 => '\u{e0}',
            0xb6 => '\u{e8}',
            0xb7 => '\u{ec}',
            0xb8 => '\u{f2}',
            0xb9 => '\u{f9}',
            0xba => '\u{c0}',
            0xbb => '\u{c8}',
            0xbc => '\u{cc}',
            0xbd => '\u{d2}',
            0xbe => '\u{d9}',
            0xbf => '\u{e2}',
            0xc0 => '\u{ea}',
            0xc1 => '\u{ee}',
            0xc2 => '\u{f4}',
            0xc3 => '\u{fb}',
            0xc4 => '\u{c2}',
            0xc5 => '\u{ca}',
            0xc6 => '\u{ce}',
            0xc7 => '\u{d4}',
            0xc8 => '\u{db}',
            0xc9 => '\u{e5}',
            0xca => '\u{c5}',
            0xcb => '\u{f8}',
            0xcc => '\u{d8}',
            0xcd => '\u{e3}',
            0xce => '\u{f1}',
            0xcf => '\u{f5}',
            0xd0 => '\u{c3}',
            0xd1 => '\u{d1}',
            0xd2 => '\u{d5}',
            0xd3 => '\u{e6}',
            0xd4 => '\u{c6}',
            0xd5 => '\u{e7}',
            0xd6 => '\u{c7}',
            0xd7 => '\u{fe}',
            0xd8 => '\u{f0}',
            0xd9 => '\u{de}',
            0xda => '\u{d0}',
            0xdb => '\u{a3}',
            0xdc => '\u{153}',
            0xdd => '\u{152}',
            0xde => '\u{a1}',
            0xdf => '\u{bf}',
            _ => (zchar as u8) as char,
        },
        3 => match zchar {
            0x20..=0x7e => (zchar as u8) as char,
            0xb3 => '\u{2502}',
            0xbf => '\u{2510}',
            0xc0 => '\u{2514}',
            0xc4 => '\u{2500}',
            0xd9 => '\u{2518}',
            0xda => '\u{250c}',
            _ => {
                warn!(target: "app::trace", "Unmapped font 3 character {:02x}", zchar as u8);
                zchar as u8 as char
            }
        },
        _ => '@',
    }
}
