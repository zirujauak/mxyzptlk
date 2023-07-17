use std::{cell::RefCell, collections::VecDeque};

use crate::{
    config::Config,
    error::RuntimeError,
    instruction::{
        Branch, Instruction, Opcode, OpcodeForm, Operand, OperandCount, OperandType, StoreResult,
    },
    sound::Manager,
    zmachine::{
        state::{memory::Memory, State},
        ZMachine,
    },
};

thread_local! {
    pub static PRINT:RefCell<String> = RefCell::new(String::new());
    pub static INPUT:RefCell<VecDeque<char>> = RefCell::new(VecDeque::new());
    pub static INPUT_DELAY:RefCell<u64> = RefCell::new(0);
    pub static INPUT_TIMEOUT:RefCell<bool> = RefCell::new(false);
    pub static COLORS:RefCell<(u8, u8)> = RefCell::new((0, 0));
    pub static SPLIT:RefCell<u8> = RefCell::new(0);
    pub static WINDOW:RefCell<u8> = RefCell::new(0);
    pub static ERASE_WINDOW:RefCell<Vec<i8>> = RefCell::new(Vec::new());
    pub static ERASE_LINE:RefCell<bool> = RefCell::new(false);
    pub static STYLE:RefCell<u8> = RefCell::new(0);
    pub static BUFFER:RefCell<u16> = RefCell::new(0);
    pub static STREAM:RefCell<(u8, Option<usize>)> = RefCell::new((0, None));
    pub static BEEP:RefCell<bool> = RefCell::new(false);
    pub static PLAY_SOUND:RefCell<(usize, u8, u8)> = RefCell::new((0, 0, 0));
    pub static CURSOR:RefCell<(u32, u32)> = RefCell::new((0, 0));
    pub static SCROLL:RefCell<u32> = RefCell::new(0);
    pub static BACKSPACE:RefCell<(u32, u32)> = RefCell::new((0, 0));
    pub static RESET:RefCell<bool> = RefCell::new(false);
    pub static QUIT:RefCell<bool> = RefCell::new(false);
}

pub fn print_char(c: char) {
    PRINT.with(|x| x.borrow_mut().push(c));
}

fn print() -> String {
    PRINT.with(|x| x.borrow().to_string())
}

pub fn input(i: &[char]) {
    for c in i {
        INPUT.with(|x| x.borrow_mut().push_back(*c));
    }
}

pub fn input_char() -> Option<char> {
    INPUT.with(|x| x.borrow_mut().pop_front())
}

pub fn input_delay() -> u64 {
    INPUT_DELAY.with(|x| x.borrow().to_owned())
}

pub fn set_input_delay(msec: u64) {
    INPUT_DELAY.with(|x| x.swap(&RefCell::new(msec)));
}

pub fn input_timeout() -> bool {
    INPUT_TIMEOUT.with(|x| x.borrow().to_owned())
}

pub fn set_input_timeout() {
    INPUT_TIMEOUT.with(|x| x.swap(&RefCell::new(true)))
}

pub fn colors() -> (u8, u8) {
    COLORS.with(|x| x.borrow().to_owned())
}

pub fn set_colors(colors: (u8, u8)) {
    COLORS.with(|x| x.swap(&RefCell::new(colors)));
}

pub fn split() -> u8 {
    SPLIT.with(|x| x.borrow().to_owned())
}

pub fn set_split(lines: u8) {
    SPLIT.with(|x| x.swap(&RefCell::new(lines)));
}

pub fn window() -> u8 {
    WINDOW.with(|x| x.borrow().to_owned())
}

pub fn set_window(window: u8) {
    WINDOW.with(|x| x.swap(&RefCell::new(window)))
}

pub fn erase_window() -> Vec<i8> {
    ERASE_WINDOW.with(|x| x.borrow().clone())
}

pub fn set_erase_window(window: i8) {
    ERASE_WINDOW.with(|x| x.borrow_mut().push(window));
}

pub fn erase_line() -> bool {
    ERASE_LINE.with(|x| x.borrow().to_owned())
}

pub fn set_erase_line() {
    ERASE_LINE.with(|x| x.swap(&RefCell::new(true)));
}

pub fn style() -> u8 {
    STYLE.with(|x| x.borrow().to_owned())
}

pub fn set_style(style: u8) {
    STYLE.with(|x| x.swap(&RefCell::new(style)));
}

pub fn buffer_mode() -> u16 {
    BUFFER.with(|x| x.borrow().to_owned())
}

pub fn set_buffer_mode(mode: u16) {
    BUFFER.with(|x| x.swap(&RefCell::new(mode)));
}

pub fn output_stream() -> (u8, Option<usize>) {
    STREAM.with(|x| x.borrow().to_owned())
}

pub fn set_output_stream(mask: u8, table: Option<usize>) {
    STREAM.with(|x| x.swap(&RefCell::new((mask, table))));
}

pub fn beep() -> bool {
    BEEP.with(|x| x.borrow().to_owned())
}

pub fn set_beep() {
    BEEP.with(|x| x.swap(&RefCell::new(true)));
}

pub fn play_sound() -> (usize, u8, u8) {
    PLAY_SOUND.with(|x| x.borrow().to_owned())
}

pub fn set_play_sound(size: usize, volume: u8, repeats: u8) {
    PLAY_SOUND.with(|x| x.swap(&RefCell::new((size, volume, repeats))));
}

pub fn cursor() -> (u32, u32) {
    CURSOR.with(|x| x.borrow().to_owned())
}

pub fn set_cursor(row: u32, column: u32) {
    CURSOR.with(|x| x.swap(&RefCell::new((row, column))));
}

pub fn scroll() -> u32 {
    SCROLL.with(|x| x.borrow().to_owned())
}

pub fn set_scroll(row: u32) {
    SCROLL.with(|x| x.swap(&RefCell::new(row)))
}

pub fn backspace() -> (u32, u32) {
    BACKSPACE.with(|x| x.borrow().to_owned())
}

pub fn set_backspace(at: (u32, u32)) {
    BACKSPACE.with(|x| x.swap(&RefCell::new(at)));
}

pub fn reset() -> bool {
    RESET.with(|x| x.borrow().to_owned())
}

pub fn set_reset() {
    RESET.with(|x| x.swap(&RefCell::new(true)));
}

pub fn quit() -> bool {
    QUIT.with(|x| x.borrow().to_owned())
}

pub fn set_quit() {
    QUIT.with(|x| x.swap(&RefCell::new(true)));
}

pub fn test_map(version: u8) -> Vec<u8> {
    let mut v = vec![0; 0x800];
    v[0] = version;
    // Initial PC at $0400
    v[6] = 0x4;
    // Object table as 0x200
    v[0x0A] = 0x02;
    // Static mark at $0400
    v[0x0E] = 0x04;
    // Global variables at $0100
    v[0x0C] = 0x01;

    v
}

pub fn set_variable(map: &mut [u8], variable: u8, value: u16) {
    let address = 0x100 + ((variable - 16) * 2) as usize;
    map[address] = (value >> 8) as u8;
    map[address + 1] = value as u8;
}

pub fn mock_zmachine(map: Vec<u8>) -> ZMachine {
    let m = Memory::new(map);
    let manager = Manager::mock();
    assert!(manager.is_ok());
    let z = ZMachine::new(m, Config::default(), Some(manager.unwrap()), "test");
    assert!(z.is_ok());
    z.unwrap()
}

pub fn mock_state(map: Vec<u8>) -> State {
    let m = Memory::new(map);
    let s = State::new(m);
    assert!(s.is_ok());
    s.unwrap()
}

pub fn operand(operand_type: OperandType, value: u16) -> Operand {
    Operand::new(operand_type, value)
}

pub fn mock_instruction(
    address: usize,
    operands: Vec<Operand>,
    opcode: Opcode,
    next_address: usize,
) -> Instruction {
    Instruction::new(address, opcode, operands, None, None, next_address)
}

pub fn branch(byte_address: usize, condition: bool, branch_address: usize) -> Branch {
    Branch::new(byte_address, condition, branch_address)
}

pub fn mock_branch_instruction(
    address: usize,
    operands: Vec<Operand>,
    opcode: Opcode,
    next_address: usize,
    branch: Branch,
) -> Instruction {
    Instruction::new(address, opcode, operands, None, Some(branch), next_address)
}

pub fn store(byte_address: usize, variable: u8) -> StoreResult {
    StoreResult::new(byte_address, variable)
}

pub fn mock_store_instruction(
    address: usize,
    operands: Vec<Operand>,
    opcode: Opcode,
    next_address: usize,
    result: StoreResult,
) -> Instruction {
    Instruction::new(address, opcode, operands, Some(result), None, next_address)
}

pub fn mock_branch_store_instruction(
    address: usize,
    operands: Vec<Operand>,
    opcode: Opcode,
    next_address: usize,
    branch: Branch,
    result: StoreResult,
) -> Instruction {
    Instruction::new(
        address,
        opcode,
        operands,
        Some(result),
        Some(branch),
        next_address,
    )
}
pub fn mock_branch(condition: bool, branch_address: usize, next_address: usize) -> Instruction {
    Instruction::new(
        0,
        Opcode::new(5, 1, 1, OpcodeForm::Var, OperandCount::_VAR),
        vec![],
        None,
        Some(Branch::new(0, condition, branch_address)),
        next_address,
    )
}

pub fn mock_store_result(result: Option<u8>, next_address: usize) -> Instruction {
    let r = result.map(|x| StoreResult::new(0, x));
    Instruction::new(
        0,
        Opcode::new(5, 1, 1, OpcodeForm::Var, OperandCount::_VAR),
        vec![],
        r,
        None,
        next_address,
    )
}

pub fn mock_frame(
    zmachine: &mut ZMachine,
    address: usize,
    result: Option<u8>,
    return_address: usize,
) {
    let r = result.map(|x| StoreResult::new(0, x));
    assert!(zmachine
        .call_routine(address, &vec![], r, return_address)
        .is_ok());
}

pub fn mock_routine(map: &mut [u8], address: usize, local_variables: &[u16]) {
    // Arguments
    map[address] = local_variables.len() as u8;
    for (i, w) in local_variables.iter().enumerate() {
        if map[0] < 5 {
            map[address + 1 + (i * 2)] = (*w >> 8) as u8;
            map[address + 2 + (i * 2)] = *w as u8;
        }
    }
}

pub fn mock_dictionary(map: &mut [u8]) {
    // Create a simple dictionary with 4 words
    // hello
    // inventory
    // look
    // sailor
    // Set the dictionary address to 0x300
    map[0x08] = 0x03;

    map[0x300] = 3;
    map[0x301] = b'.';
    map[0x302] = b',';
    map[0x303] = b'"';

    // Entry length is 9 bytes
    map[0x304] = 0x9;
    // There are 4 entries
    map[0x306] = 4;

    if map[0] == 3 {
        // Text buffer is at 0x380 and can hold up to 10 characters
        map[0x380] = 11;
        // Parse buffer is at 0x3A0 and can hold up to 2 entries
        map[0x3A0] = 2;
        // hello
        //   C     A     11       11    14    5
        // 0 01100 01010 10001  1 10001 10100 00101
        // 3151 C685
        map[0x307] = 0x31;
        map[0x308] = 0x51;
        map[0x309] = 0xC6;
        map[0x30a] = 0x85;
        // 5 bytes of other content
        // inventory
        //   E     13    1B       A     13    19
        // 0 01110 10011 11011  1 01010 10011 11001
        // 3A7B AA79
        map[0x310] = 0x3A;
        map[0x311] = 0x7B;
        map[0x312] = 0xAA;
        map[0x313] = 0x79;
        // 5 bytes of other content
        // look
        //   11    14    14       10    5     5
        // 0 10001 10100 10100  1 01010 00101 00101
        // 4D24 A8A5
        map[0x319] = 0x4D;
        map[0x31A] = 0x24;
        map[0x31B] = 0xA8;
        map[0x31C] = 0xA5;
        // 5 bytes of other content
        // sailor
        //   18    6     E        11    14    17
        // 0 11000 00110 01110  1 10001 10100 10111
        // 60CE C697
        map[0x322] = 0x60;
        map[0x323] = 0xCE;
        map[0x324] = 0xC6;
        map[0x325] = 0x97;
    } else {
        // Text buffer is at 0x380 and can hold up to 10 characters
        if map[0] == 4 {
            map[0x380] = 11;
        } else {
            map[0x380] = 10;
        }
        // Parse buffer is at 0x3A0 and can hold up to 2 entries
        map[0x3A0] = 2;
        // hello
        //   C     A     11       11    14    5        5     5     5
        // 0 01100 01010 10001  0 10001 10100 00101  1 00101 00101 00101
        // 3151 4685 94A5
        map[0x307] = 0x31;
        map[0x308] = 0x51;
        map[0x309] = 0x46;
        map[0x30a] = 0x85;
        map[0x30b] = 0x94;
        map[0x30c] = 0xA5;
        // 3 bytes of other content
        // inventory
        //   E     13    1B       4     13    19       14    17    1E
        // 0 01110 10011 11011  0 01010 10011 11001  1 10100 10111 11110
        // 3A7B 2A79 D2FE
        map[0x310] = 0x3A;
        map[0x311] = 0x7B;
        map[0x312] = 0x2A;
        map[0x313] = 0x79;
        map[0x314] = 0xD2;
        map[0x315] = 0xFE;
        // 5 bytes of other content
        // look
        //   11    14    14       10    5     5
        // 0 10001 10100 10100  0 01010 00101 00101
        // 4D24 48A5 94A5;
        map[0x319] = 0x4D;
        map[0x31A] = 0x24;
        map[0x31B] = 0x48;
        map[0x31C] = 0xA5;
        map[0x31D] = 0x94;
        map[0x31E] = 0xA5;
        // 5 bytes of other content
        // sailor
        //   18    6     E        11    14    17
        // 0 11000 00110 01110  0 10001 10100 10111
        // 60CE 4697 94A5
        map[0x322] = 0x60;
        map[0x323] = 0xCE;
        map[0x324] = 0x46;
        map[0x325] = 0x97;
        map[0x326] = 0x94;
        map[0x327] = 0xA5;
    }
}

pub fn mock_custom_dictionary(map: &mut [u8], address: usize) {
    // Create a custom dictionary with 3 words
    // xyzzy
    // plover
    // moon

    map[address] = 3;
    map[address + 1] = b'.';
    map[address + 2] = b',';
    map[address + 3] = b'"';

    // Entry length is 9 bytes
    map[address + 4] = 0x9;
    // There are 3 entries, unsorted
    map[address + 5] = 0xFF;
    map[address + 6] = 0xFD;

    // xyzzy
    //   1D    1E    1F       1F    1E    5        5     5     5
    // 0 11101 11110 11111  0 11111 11110 00101  1 00101 00101 00101
    // 77DF 7FC5 94A5
    map[address + 7] = 0x77;
    map[address + 8] = 0xDF;
    map[address + 9] = 0x7F;
    map[address + 10] = 0xC5;
    map[address + 11] = 0x94;
    map[address + 12] = 0xA5;
    // 3 bytes of other content
    // plover
    //   15    11    14       1B    A     17       5     5     5
    // 0 10101 10001 10100  0 11011 01010 10111  1 00101 00101 00101
    // 5634 6D57 94A5
    map[address + 16] = 0x56;
    map[address + 17] = 0x34;
    map[address + 18] = 0x6D;
    map[address + 19] = 0x57;
    map[address + 20] = 0x94;
    map[address + 21] = 0xA5;
    // 5 bytes of other content
    // moon
    //   12    14    14       13    5     5
    // 0 10010 10100 10100  0 10011 00101 00101
    // 4A94 54A5 4CA5;
    map[address + 25] = 0x4A;
    map[address + 26] = 0x94;
    map[address + 27] = 0x4C;
    map[address + 28] = 0xA5;
    map[address + 29] = 0x94;
    map[address + 30] = 0xA5;
}

pub fn assert_ok<T>(result: Result<T, RuntimeError>) -> T {
    assert!(result.is_ok());
    result.unwrap()
}

pub fn assert_eq_ok<T: std::fmt::Debug + std::cmp::PartialEq>(
    s: Result<T, RuntimeError>,
    value: T,
) {
    assert!(s.is_ok());
    assert_eq!(s.unwrap(), value);
}

pub fn assert_print(str: &str) {
    assert_eq!(print(), str);
}

pub fn mock_object(
    map: &mut [u8],
    object: usize,
    short_name: Vec<u16>,
    (parent, sibling, child): (u16, u16, u16),
) {
    let version = map[0];
    let object_table = ((map[0x0a] as usize) << 8) + map[0x0b] as usize;
    let object_address = if version < 4 {
        object_table + 62 + ((object - 1) * 9)
    } else {
        object_table + 126 + ((object - 1) * 14)
    };

    // Property tables will be placed at 0x300
    let property_table_address = 0x300 + ((object - 1) * 20);
    // Set parent/sibling/child
    // Set the property table address
    if version < 4 {
        map[object_address + 4] = parent as u8;
        map[object_address + 5] = sibling as u8;
        map[object_address + 6] = child as u8;
        map[object_address + 7] = (property_table_address >> 8) as u8;
        map[object_address + 8] = property_table_address as u8;
    } else {
        map[object_address + 6] = (parent >> 8) as u8;
        map[object_address + 7] = parent as u8;
        map[object_address + 8] = (sibling >> 8) as u8;
        map[object_address + 9] = sibling as u8;
        map[object_address + 10] = (child >> 8) as u8;
        map[object_address + 11] = child as u8;
        map[object_address + 12] = (property_table_address >> 8) as u8;
        map[object_address + 13] = property_table_address as u8;
    }

    let l = short_name.len();
    map[property_table_address] = l as u8;

    for (i, w) in short_name.iter().enumerate() {
        let a = property_table_address + 1 + (i * 2);
        map[a] = (*w >> 8) as u8;
        map[a + 1] = *w as u8;
    }
}

pub fn mock_attributes(map: &mut [u8], object: usize, attributes: &[u8]) {
    let version = map[0];
    let object_table = ((map[0x0a] as usize) << 8) + map[0x0b] as usize;
    let object_address = if version < 4 {
        object_table + 62 + ((object - 1) * 9)
    } else {
        object_table + 126 + ((object - 1) * 14)
    };

    for (i, b) in attributes.iter().enumerate() {
        map[object_address + i] = *b;
    }
}

pub fn mock_default_properties(map: &mut [u8]) {
    let version = map[0];
    let words = if version < 4 { 31 } else { 63 };

    let object_table = ((map[0x0a] as usize) << 8) + map[0x0b] as usize;
    for i in 0..words {
        let address = object_table + (i * 2);
        map[address] = (i as u8) % 0x10;
        map[address + 1] = i as u8;
    }
}

pub fn mock_properties(map: &mut [u8], object: usize, properties: &[(u8, &Vec<u8>)]) {
    let property_table_address = 0x300 + ((object - 1) * 20);
    let hl = map[property_table_address] as usize;

    let mut address = property_table_address + 1 + (hl * 2);
    for (number, data) in properties {
        match (map[0], data.len()) {
            // V3
            (3, _) => {
                let size = ((data.len() - 1) * 32) as u8 + *number;
                map[address] = size;
                for (i, b) in data.iter().enumerate() {
                    map[address + 1 + i] = *b;
                }
                address = address + 1 + data.len();
            }
            // V4+, 1 byte
            (_, 1) => {
                map[address] = *number;
                map[address + 1] = data[0];
                address = address + 1 + data.len();
            }
            // V4+, 2 bytes
            (_, 2) => {
                map[address] = 0x40 | *number;
                map[address + 1] = data[0];
                map[address + 2] = data[1];
                address = address + 1 + data.len();
            }
            // V4+, > 2 bytes
            (_, _) => {
                map[address] = 0x80 | *number;
                map[address + 1] = 0x80 | (data.len() as u8 & 0x3F);
                for (i, b) in data.iter().enumerate() {
                    map[address + 1 + i] = *b;
                }
                address = address + 2 + data.len();
            }
        }
    }
}
