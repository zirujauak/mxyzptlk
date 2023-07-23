use super::InputEvent;

#[cfg(not(test))]
pub mod pancurses;
#[cfg(test)]
pub mod test_terminal;

fn char_to_u16(c: char) -> InputEvent {
    match c {
        // Mac | Windows - slight differences in character values for backspace and return
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
        }
    }
}

fn map_output(zchar: u16, font: u8) -> char {
    match font {
        1 | 4 => match zchar {
            0x18 => '\u{2191}',
            0x19 => '\u{2193}',
            0x1a => '\u{2192}',
            0x1b => '\u{2190}',
            0x20..=0x7E => (zchar as u8) as char,
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
            _ => {
                error!(target: "app::input", "Unmapped font {} character {:04x}", font, zchar);
                zchar as u8 as char
            }
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
                warn!(target: "app::trace", "Unmapped font 3 character {:04x}", zchar);
                zchar as u8 as char
            }
        },
        _ => '@',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_char_to_u16(c: char, i: InputEvent) {
        let ie = char_to_u16(c);
        assert_eq!(ie, i, "{} should map to {:?}, was {:?}", c, i, ie);
    }

    #[test]
    fn test_char_to_u16() {
        // Backspace, Return
        assert_char_to_u16('\u{08}', InputEvent::from_char(0x08));
        assert_char_to_u16('\u{7f}', InputEvent::from_char(0x08));
        assert_char_to_u16('\u{0a}', InputEvent::from_char(0x0d));
        assert_char_to_u16('\u{0d}', InputEvent::from_char(0x0d));
        // ASCII
        for c in ' '..='~' {
            assert_char_to_u16(c, InputEvent::from_char(c as u16));
        }
        // Accented characters
        assert_char_to_u16('\u{a1}', InputEvent::from_char(0xde));
        assert_char_to_u16('\u{a3}', InputEvent::from_char(0xdb));
        assert_char_to_u16('\u{ab}', InputEvent::from_char(0xa3));
        assert_char_to_u16('\u{bb}', InputEvent::from_char(0xa2));
        assert_char_to_u16('\u{bf}', InputEvent::from_char(0xdf));
        assert_char_to_u16('\u{c0}', InputEvent::from_char(0xba));
        assert_char_to_u16('\u{c1}', InputEvent::from_char(0xaf));
        assert_char_to_u16('\u{c2}', InputEvent::from_char(0xc4));
        assert_char_to_u16('\u{c3}', InputEvent::from_char(0xd0));
        assert_char_to_u16('\u{c4}', InputEvent::from_char(0x9e));
        assert_char_to_u16('\u{c5}', InputEvent::from_char(0xca));
        assert_char_to_u16('\u{c6}', InputEvent::from_char(0xd4));
        assert_char_to_u16('\u{c7}', InputEvent::from_char(0xd6));
        assert_char_to_u16('\u{c8}', InputEvent::from_char(0xbb));
        assert_char_to_u16('\u{c9}', InputEvent::from_char(0xb0));
        assert_char_to_u16('\u{ca}', InputEvent::from_char(0xc5));
        assert_char_to_u16('\u{cb}', InputEvent::from_char(0xa7));
        assert_char_to_u16('\u{cc}', InputEvent::from_char(0xbc));
        assert_char_to_u16('\u{cd}', InputEvent::from_char(0xb1));
        assert_char_to_u16('\u{ce}', InputEvent::from_char(0xc6));
        assert_char_to_u16('\u{cf}', InputEvent::from_char(0xa8));
        assert_char_to_u16('\u{d0}', InputEvent::from_char(0xda));
        assert_char_to_u16('\u{d1}', InputEvent::from_char(0xd1));
        assert_char_to_u16('\u{d2}', InputEvent::from_char(0xbd));
        assert_char_to_u16('\u{d3}', InputEvent::from_char(0xb2));
        assert_char_to_u16('\u{d4}', InputEvent::from_char(0xc7));
        assert_char_to_u16('\u{d5}', InputEvent::from_char(0xd2));
        assert_char_to_u16('\u{d6}', InputEvent::from_char(0x9f));
        assert_char_to_u16('\u{d8}', InputEvent::from_char(0xcc));
        assert_char_to_u16('\u{d9}', InputEvent::from_char(0xbe));
        assert_char_to_u16('\u{da}', InputEvent::from_char(0xb3));
        assert_char_to_u16('\u{db}', InputEvent::from_char(0xc8));
        assert_char_to_u16('\u{dc}', InputEvent::from_char(0xa0));
        assert_char_to_u16('\u{dd}', InputEvent::from_char(0xb4));
        assert_char_to_u16('\u{de}', InputEvent::from_char(0xd9));
        assert_char_to_u16('\u{df}', InputEvent::from_char(0xa1));
        assert_char_to_u16('\u{e0}', InputEvent::from_char(0xb5));
        assert_char_to_u16('\u{e1}', InputEvent::from_char(0xa9));
        assert_char_to_u16('\u{e2}', InputEvent::from_char(0xbf));
        assert_char_to_u16('\u{e3}', InputEvent::from_char(0xcd));
        assert_char_to_u16('\u{e4}', InputEvent::from_char(0x9b));
        assert_char_to_u16('\u{e5}', InputEvent::from_char(0xc9));
        assert_char_to_u16('\u{e6}', InputEvent::from_char(0xd3));
        assert_char_to_u16('\u{e7}', InputEvent::from_char(0xd5));
        assert_char_to_u16('\u{e8}', InputEvent::from_char(0xb6));
        assert_char_to_u16('\u{e9}', InputEvent::from_char(0xaa));
        assert_char_to_u16('\u{ea}', InputEvent::from_char(0xc0));
        assert_char_to_u16('\u{eb}', InputEvent::from_char(0xa4));
        assert_char_to_u16('\u{ec}', InputEvent::from_char(0xb7));
        assert_char_to_u16('\u{ed}', InputEvent::from_char(0xab));
        assert_char_to_u16('\u{ee}', InputEvent::from_char(0xc1));
        assert_char_to_u16('\u{ef}', InputEvent::from_char(0xa5));
        assert_char_to_u16('\u{f0}', InputEvent::from_char(0xd8));
        assert_char_to_u16('\u{f1}', InputEvent::from_char(0xce));
        assert_char_to_u16('\u{f2}', InputEvent::from_char(0xb8));
        assert_char_to_u16('\u{f3}', InputEvent::from_char(0xac));
        assert_char_to_u16('\u{f4}', InputEvent::from_char(0xc2));
        assert_char_to_u16('\u{f5}', InputEvent::from_char(0xcf));
        assert_char_to_u16('\u{f6}', InputEvent::from_char(0x9c));
        assert_char_to_u16('\u{f8}', InputEvent::from_char(0xcb));
        assert_char_to_u16('\u{f9}', InputEvent::from_char(0xb9));
        assert_char_to_u16('\u{fa}', InputEvent::from_char(0xad));
        assert_char_to_u16('\u{fb}', InputEvent::from_char(0xc3));
        assert_char_to_u16('\u{fc}', InputEvent::from_char(0x9d));
        assert_char_to_u16('\u{fd}', InputEvent::from_char(0xae));
        assert_char_to_u16('\u{fe}', InputEvent::from_char(0xd7));
        assert_char_to_u16('\u{ff}', InputEvent::from_char(0xa6));
        assert_char_to_u16('\u{152}', InputEvent::from_char(0xdd));
        assert_char_to_u16('\u{153}', InputEvent::from_char(0xdc));

        // Unmapped
        assert_char_to_u16('\u{255}', InputEvent::no_input());
    }

    fn assert_u16_to_char(zchar: u16, font: u8, c: char) {
        let ch = map_output(zchar, font);
        assert_eq!(
            ch, c,
            "Font {} {:04x} should map to '{}', was '{}'",
            font, zchar, c, ch
        );
    }

    #[test]
    fn test_map_output_font_1_and_4() {
        let mut font = 1;
        while font < 5 {
            // Arrows
            assert_u16_to_char(0x18, 1, '\u{2191}');
            assert_u16_to_char(0x19, 1, '\u{2193}');
            assert_u16_to_char(0x1A, 1, '\u{2192}');
            assert_u16_to_char(0x1B, 1, '\u{2190}');
            // ASCII
            for c in b' '..=b'~' {
                assert_u16_to_char(c as u16, 1, c as char);
            }
            // Accented characters
            assert_u16_to_char(0x9B, 1, '\u{e4}');
            assert_u16_to_char(0x9C, 1, '\u{f6}');
            assert_u16_to_char(0x9D, 1, '\u{fc}');
            assert_u16_to_char(0x9E, 1, '\u{c4}');
            assert_u16_to_char(0x9F, 1, '\u{d6}');
            assert_u16_to_char(0xA0, 1, '\u{dc}');
            assert_u16_to_char(0xA1, 1, '\u{df}');
            assert_u16_to_char(0xA2, 1, '\u{bb}');
            assert_u16_to_char(0xA3, 1, '\u{ab}');
            assert_u16_to_char(0xA4, 1, '\u{eb}');
            assert_u16_to_char(0xA5, 1, '\u{ef}');
            assert_u16_to_char(0xA6, 1, '\u{ff}');
            assert_u16_to_char(0xA7, 1, '\u{cb}');
            assert_u16_to_char(0xA8, 1, '\u{cf}');
            assert_u16_to_char(0xA9, 1, '\u{e1}');
            assert_u16_to_char(0xAA, 1, '\u{e9}');
            assert_u16_to_char(0xAB, 1, '\u{ed}');
            assert_u16_to_char(0xAC, 1, '\u{f3}');
            assert_u16_to_char(0xAD, 1, '\u{fa}');
            assert_u16_to_char(0xAE, 1, '\u{fd}');
            assert_u16_to_char(0xAF, 1, '\u{c1}');
            assert_u16_to_char(0xB0, 1, '\u{c9}');
            assert_u16_to_char(0xB1, 1, '\u{cd}');
            assert_u16_to_char(0xB2, 1, '\u{d3}');
            assert_u16_to_char(0xB3, 1, '\u{da}');
            assert_u16_to_char(0xB4, 1, '\u{dd}');
            assert_u16_to_char(0xB5, 1, '\u{e0}');
            assert_u16_to_char(0xB6, 1, '\u{e8}');
            assert_u16_to_char(0xB7, 1, '\u{ec}');
            assert_u16_to_char(0xB8, 1, '\u{f2}');
            assert_u16_to_char(0xB9, 1, '\u{f9}');
            assert_u16_to_char(0xBA, 1, '\u{c0}');
            assert_u16_to_char(0xBB, 1, '\u{c8}');
            assert_u16_to_char(0xBC, 1, '\u{cc}');
            assert_u16_to_char(0xBD, 1, '\u{d2}');
            assert_u16_to_char(0xBE, 1, '\u{d9}');
            assert_u16_to_char(0xBF, 1, '\u{e2}');
            assert_u16_to_char(0xC0, 1, '\u{ea}');
            assert_u16_to_char(0xC1, 1, '\u{ee}');
            assert_u16_to_char(0xC2, 1, '\u{f4}');
            assert_u16_to_char(0xC3, 1, '\u{fb}');
            assert_u16_to_char(0xC4, 1, '\u{c2}');
            assert_u16_to_char(0xC5, 1, '\u{ca}');
            assert_u16_to_char(0xC6, 1, '\u{ce}');
            assert_u16_to_char(0xC7, 1, '\u{d4}');
            assert_u16_to_char(0xC8, 1, '\u{db}');
            assert_u16_to_char(0xC9, 1, '\u{e5}');
            assert_u16_to_char(0xCA, 1, '\u{c5}');
            assert_u16_to_char(0xCB, 1, '\u{f8}');
            assert_u16_to_char(0xCC, 1, '\u{d8}');
            assert_u16_to_char(0xCD, 1, '\u{e3}');
            assert_u16_to_char(0xCE, 1, '\u{f1}');
            assert_u16_to_char(0xCF, 1, '\u{f5}');
            assert_u16_to_char(0xD0, 1, '\u{c3}');
            assert_u16_to_char(0xD1, 1, '\u{d1}');
            assert_u16_to_char(0xD2, 1, '\u{d5}');
            assert_u16_to_char(0xD3, 1, '\u{e6}');
            assert_u16_to_char(0xD4, 1, '\u{c6}');
            assert_u16_to_char(0xD5, 1, '\u{e7}');
            assert_u16_to_char(0xD6, 1, '\u{c7}');
            assert_u16_to_char(0xD7, 1, '\u{fe}');
            assert_u16_to_char(0xD8, 1, '\u{f0}');
            assert_u16_to_char(0xD9, 1, '\u{de}');
            assert_u16_to_char(0xDA, 1, '\u{d0}');
            assert_u16_to_char(0xDB, 1, '\u{a3}');
            assert_u16_to_char(0xDC, 1, '\u{153}');
            assert_u16_to_char(0xDD, 1, '\u{152}');
            assert_u16_to_char(0xDE, 1, '\u{a1}');
            assert_u16_to_char(0xDF, 1, '\u{bf}');

            // Unmapped
            assert_u16_to_char(0x7F, 1, '\u{7F}');
            font += 3;
        }
    }

    #[test]
    fn test_map_output_font_3() {
        // ASCII
        for b in 0x20..=0x73 {
            assert_u16_to_char(b, 3, b as u8 as char);
        }

        // Boxes, incomplete
        assert_u16_to_char(0xb3, 3, '\u{2502}');
        assert_u16_to_char(0xbf, 3, '\u{2510}');
        assert_u16_to_char(0xc0, 3, '\u{2514}');
        assert_u16_to_char(0xc4, 3, '\u{2500}');
        assert_u16_to_char(0xd9, 3, '\u{2518}');
        assert_u16_to_char(0xda, 3, '\u{250c}');

        // Unmapped
        assert_u16_to_char(0x7F, 3, '\u{7F}');
    }
}
