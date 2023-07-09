use crate::error::*;
use crate::zmachine::ZMachine;

use super::*;

mod processor_0op;
mod processor_1op;
mod processor_2op;
mod processor_ext;
mod processor_var;

fn operand_value(zmachine: &mut ZMachine, operand: &Operand) -> Result<u16, RuntimeError> {
    match operand.operand_type() {
        OperandType::SmallConstant | OperandType::LargeConstant => Ok(operand.value()),
        OperandType::Variable => zmachine.variable(operand.value() as u8),
    }
}

fn operand_values(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
) -> Result<Vec<u16>, RuntimeError> {
    let mut v = Vec::new();
    let mut l = "Operand values: ".to_string();
    for o in instruction.operands() {
        let value = operand_value(zmachine, o)?;
        match o.operand_type {
            OperandType::SmallConstant => l.push_str(&format!(" #{:02x}", value as u8)),
            _ => l.push_str(&format!(" #{:04x}", value)),
        }
        v.push(value)
    }
    if !v.is_empty() {
        info!(target: "app::instruction", "{}", l);
    }
    Ok(v)
}

fn branch(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    condition: bool,
) -> Result<usize, RuntimeError> {
    match instruction.branch() {
        Some(b) => {
            if condition == b.condition() {
                match b.branch_address {
                    0 => zmachine.return_routine(0), // return false
                    1 => zmachine.return_routine(1), // return true,
                    _ => Ok(b.branch_address()),
                }
            } else {
                Ok(instruction.next_address())
            }
        }
        None => Ok(instruction.next_address()),
    }
}

fn store_result(
    zmachine: &mut ZMachine,
    instruction: &Instruction,
    value: u16,
) -> Result<(), RuntimeError> {
    match instruction.store() {
        Some(s) => zmachine.set_variable(s.variable, value),
        None => Ok(()),
    }
}

fn call_fn(
    zmachine: &mut ZMachine,
    address: usize,
    return_addr: usize,
    arguments: &Vec<u16>,
    result: Option<StoreResult>,
) -> Result<usize, RuntimeError> {
    match address {
        0 | 1 => {
            if let Some(r) = result {
                zmachine.set_variable(r.variable, address as u16)?
            }

            Ok(return_addr)
        }
        _ => zmachine.call_routine(address, arguments, result, return_addr),
    }
}

pub fn dispatch(zmachine: &mut ZMachine, instruction: &Instruction) -> Result<usize, RuntimeError> {
    info!(target: "app::instruction", "dispatch: {}", instruction);
    match instruction.opcode().form() {
        OpcodeForm::Ext => match (zmachine.version(), instruction.opcode().instruction()) {
            (5, 0x00) | (7, 0x00) | (8, 0x00) => processor_ext::save(zmachine, instruction),
            (5, 0x01) | (7, 0x01) | (8, 0x01) => processor_ext::restore(zmachine, instruction),
            (5, 0x02) | (7, 0x02) | (8, 0x02) => processor_ext::log_shift(zmachine, instruction),
            (5, 0x03) | (7, 0x03) | (8, 0x03) => processor_ext::art_shift(zmachine, instruction),
            (5, 0x04) | (7, 0x04) | (8, 0x04) => processor_ext::set_font(zmachine, instruction),
            //         0x05 => processor_ext::draw_picture(context, instruction),
            //         0x06 => processor_ext::picture_data(context, instruction),
            //         0x07 => processor_ext::erase_picture(context, instruction),
            //         0x08 => processor_ext::set_margins(context, instruction),
            (5, 0x09) | (7, 0x09) | (8, 0x09) => processor_ext::save_undo(zmachine, instruction),
            (5, 0x0a) | (7, 0x0a) | (8, 0x0a) => processor_ext::restore_undo(zmachine, instruction),
            //         (5, 0x0b) | (7, 0x0b) | (8, 0x0b) => processor_ext::print_unicode(context, instruction),
            //         (5, 0x0c) | (7, 0x0c) | (8, 0x0c) => processor_ext::check_unicode(context, instruction),
            //         (5, 0x0d) | (7, 0x0d) | (8, 0x0d) => processor_ext::set_true_colour(context, instruction),
            //         0x10 => processor_ext::move_window(context, instruction),
            //         0x11 => processor_ext::window_size(context, instruction),
            //         0x12 => processor_ext::window_style(context, instruction),
            //         0x13 => processor_ext::get_wind_prop(context, instruction),
            //         0x14 => processor_ext::scroll_window(context, instruction),
            //         0x15 => processor_ext::pop_stack(context, instruction),
            //         0x16 => processor_ext::read_mouse(context, instruction),
            //         0x17 => processor_ext::mouse_window(context, instruction),
            //         0x18 => processor_ext::push_stack(context, instruction),
            //         0x19 => processor_ext::put_wind_prop(context, instruction),
            //         0x1a => processor_ext::print_form(context, instruction),
            //         0x1b => processor_ext::make_menu(context, instruction),
            //         0x1c => processor_ext::picture_table(context, instruction),
            //         0x1d => processor_ext::buffer_screen(context, instruction),
            (_, _) => Err(RuntimeError::new(
                ErrorCode::UnimplementedInstruction,
                format!("Unimplemented EXT instruction: {}", instruction.opcode()),
            )),
        },
        _ => match instruction.opcode().operand_count() {
            OperandCount::_0OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x0) => processor_0op::rtrue(zmachine, instruction),
                (_, 0x1) => processor_0op::rfalse(zmachine, instruction),
                (_, 0x2) => processor_0op::print(zmachine, instruction),
                (_, 0x3) => processor_0op::print_ret(zmachine, instruction),
                (_, 0x4) => processor_0op::nop(zmachine, instruction),
                (3, 0x5) | (4, 0x5) => processor_0op::save(zmachine, instruction),
                (3, 0x6) | (4, 0x6) => processor_0op::restore(zmachine, instruction),
                (_, 0x7) => processor_0op::restart(zmachine, instruction),
                (_, 0x8) => processor_0op::ret_popped(zmachine, instruction),
                (3, 0x9) | (4, 0x9) => processor_0op::pop(zmachine, instruction),
                (_, 0x9) => processor_0op::catch(zmachine, instruction),
                (_, 0xa) => processor_0op::quit(zmachine, instruction),
                (_, 0xb) => processor_0op::new_line(zmachine, instruction),
                (3, 0xc) => processor_0op::show_status(zmachine, instruction),
                (_, 0xd) => processor_0op::verify(zmachine, instruction),
                (_, 0xf) => processor_0op::piracy(zmachine, instruction),
                (_, _) => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_1OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x0) => processor_1op::jz(zmachine, instruction),
                (_, 0x1) => processor_1op::get_sibling(zmachine, instruction),
                (_, 0x2) => processor_1op::get_child(zmachine, instruction),
                (_, 0x3) => processor_1op::get_parent(zmachine, instruction),
                (_, 0x4) => processor_1op::get_prop_len(zmachine, instruction),
                (_, 0x5) => processor_1op::inc(zmachine, instruction),
                (_, 0x6) => processor_1op::dec(zmachine, instruction),
                (_, 0x7) => processor_1op::print_addr(zmachine, instruction),
                (4, 0x8) | (5, 0x8) | (7, 0x8) | (8, 0x8) => {
                    processor_1op::call_1s(zmachine, instruction)
                }
                (_, 0x9) => processor_1op::remove_obj(zmachine, instruction),
                (_, 0xa) => processor_1op::print_obj(zmachine, instruction),
                (_, 0xb) => processor_1op::ret(zmachine, instruction),
                (_, 0xc) => processor_1op::jump(zmachine, instruction),
                (_, 0xd) => processor_1op::print_paddr(zmachine, instruction),
                (_, 0xe) => processor_1op::load(zmachine, instruction),
                (3, 0xf) | (4, 0xf) => processor_1op::not(zmachine, instruction),
                (_, 0xf) => processor_1op::call_1n(zmachine, instruction),
                (_, _) => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_2OP => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x01) => processor_2op::je(zmachine, instruction),
                (_, 0x02) => processor_2op::jl(zmachine, instruction),
                (_, 0x03) => processor_2op::jg(zmachine, instruction),
                (_, 0x04) => processor_2op::dec_chk(zmachine, instruction),
                (_, 0x05) => processor_2op::inc_chk(zmachine, instruction),
                (_, 0x06) => processor_2op::jin(zmachine, instruction),
                (_, 0x07) => processor_2op::test(zmachine, instruction),
                (_, 0x08) => processor_2op::or(zmachine, instruction),
                (_, 0x09) => processor_2op::and(zmachine, instruction),
                (_, 0x0a) => processor_2op::test_attr(zmachine, instruction),
                (_, 0x0b) => processor_2op::set_attr(zmachine, instruction),
                (_, 0x0c) => processor_2op::clear_attr(zmachine, instruction),
                (_, 0x0d) => processor_2op::store(zmachine, instruction),
                (_, 0x0e) => processor_2op::insert_obj(zmachine, instruction),
                (_, 0x0f) => processor_2op::loadw(zmachine, instruction),
                (_, 0x10) => processor_2op::loadb(zmachine, instruction),
                (_, 0x11) => processor_2op::get_prop(zmachine, instruction),
                (_, 0x12) => processor_2op::get_prop_addr(zmachine, instruction),
                (_, 0x13) => processor_2op::get_next_prop(zmachine, instruction),
                (_, 0x14) => processor_2op::add(zmachine, instruction),
                (_, 0x15) => processor_2op::sub(zmachine, instruction),
                (_, 0x16) => processor_2op::mul(zmachine, instruction),
                (_, 0x17) => processor_2op::div(zmachine, instruction),
                (_, 0x18) => processor_2op::modulus(zmachine, instruction),
                (4, 0x19) | (5, 0x19) | (7, 0x19) | (8, 0x19) => {
                    processor_2op::call_2s(zmachine, instruction)
                }
                (5, 0x1a) | (7, 0x1a) | (8, 0x1a) => processor_2op::call_2n(zmachine, instruction),
                (5, 0x1b) | (7, 0x1b) | (8, 0x1b) => {
                    processor_2op::set_colour(zmachine, instruction)
                }
                (5, 0x1c) | (7, 0x1c) | (8, 0x1c) => processor_2op::throw(zmachine, instruction),
                _ => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
            OperandCount::_VAR => match (zmachine.version(), instruction.opcode().instruction()) {
                (_, 0x00) => processor_var::call_vs(zmachine, instruction),
                (_, 0x01) => processor_var::storew(zmachine, instruction),
                (_, 0x02) => processor_var::storeb(zmachine, instruction),
                (_, 0x03) => processor_var::put_prop(zmachine, instruction),
                (_, 0x04) => processor_var::read(zmachine, instruction),
                (_, 0x05) => processor_var::print_char(zmachine, instruction),
                (_, 0x06) => processor_var::print_num(zmachine, instruction),
                (_, 0x07) => processor_var::random(zmachine, instruction),
                (_, 0x08) => processor_var::push(zmachine, instruction),
                (_, 0x09) => processor_var::pull(zmachine, instruction),
                (_, 0x0a) => processor_var::split_window(zmachine, instruction),
                (_, 0x0b) => processor_var::set_window(zmachine, instruction),
                (4, 0x0c) | (5, 0x0c) | (7, 0x0c) | (8, 0x0c) => {
                    processor_var::call_vs2(zmachine, instruction)
                }
                (4, 0x0d) | (5, 0x0d) | (7, 0x0d) | (8, 0x0d) => {
                    processor_var::erase_window(zmachine, instruction)
                }
                (4, 0x0e) | (5, 0x0e) | (7, 0x0e) | (8, 0x0e) => {
                    processor_var::erase_line(zmachine, instruction)
                }
                (4, 0x0f) | (5, 0x0f) | (7, 0x0f) | (8, 0x0f) => {
                    processor_var::set_cursor(zmachine, instruction)
                }
                (4, 0x10) | (5, 0x10) | (7, 0x10) | (8, 0x10) => {
                    processor_var::get_cursor(zmachine, instruction)
                }
                (4, 0x11) | (5, 0x11) | (7, 0x11) | (8, 0x11) => {
                    processor_var::set_text_style(zmachine, instruction)
                }
                (4, 0x12) | (5, 0x12) | (7, 0x12) | (8, 0x12) => {
                    processor_var::buffer_mode(zmachine, instruction)
                }
                (_, 0x13) => processor_var::output_stream(zmachine, instruction),
                (_, 0x14) => processor_var::input_stream(zmachine, instruction),
                (_, 0x15) => processor_var::sound_effect(zmachine, instruction),
                (4, 0x16) | (5, 0x16) | (7, 0x16) | (8, 0x16) => {
                    processor_var::read_char(zmachine, instruction)
                }
                (4, 0x17) | (5, 0x17) | (7, 0x17) | (8, 0x17) => {
                    processor_var::scan_table(zmachine, instruction)
                }
                (5, 0x18) | (7, 0x18) | (8, 0x18) => processor_var::not(zmachine, instruction),
                (5, 0x19) | (7, 0x19) | (8, 0x19) => processor_var::call_vn(zmachine, instruction),
                (5, 0x1a) | (7, 0x1a) | (8, 0x1a) => processor_var::call_vn2(zmachine, instruction),
                (5, 0x1b) | (7, 0x1b) | (8, 0x1b) => processor_var::tokenise(zmachine, instruction),
                (5, 0x1c) | (7, 0x1c) | (8, 0x1c) => {
                    processor_var::encode_text(zmachine, instruction)
                }
                (5, 0x1d) | (7, 0x1d) | (8, 0x1d) => {
                    processor_var::copy_table(zmachine, instruction)
                }
                (5, 0x1e) | (7, 0x1e) | (8, 0x1e) => {
                    processor_var::print_table(zmachine, instruction)
                }
                (5, 0x1f) | (7, 0x1f) | (8, 0x1f) => {
                    processor_var::check_arg_count(zmachine, instruction)
                }
                (_, _) => Err(RuntimeError::new(
                    ErrorCode::UnimplementedInstruction,
                    format!("Unimplemented instruction: {}", instruction.opcode()),
                )),
            },
        },
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{config::Config, sound::Manager, zmachine::state::memory::Memory};
    use std::{cell::RefCell, collections::VecDeque};

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

    // **NOTE**: Tests for dispatch() are delegated to the processor_* tests

    #[test]
    fn test_operand_value() {
        // Set up a simple memory map with global var 0x80 set
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0x789A);
        let mut zmachine = mock_zmachine(v);

        let o_small_constant = Operand::new(OperandType::SmallConstant, 0x12);
        let o_large_constant = Operand::new(OperandType::LargeConstant, 0x3456);
        let o_variable = Operand::new(OperandType::Variable, 0x80);
        assert_eq_ok(operand_value(&mut zmachine, &o_small_constant), 0x12);
        assert_eq_ok(operand_value(&mut zmachine, &o_large_constant), 0x3456);
        assert_eq_ok(operand_value(&mut zmachine, &o_variable), 0x789A);
    }

    #[test]
    fn test_operand_values() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0x789A);
        set_variable(&mut v, 0x81, 0x1357);
        let mut zmachine = mock_zmachine(v);
        let o_small_constant = Operand::new(OperandType::SmallConstant, 0x12);
        let o_large_constant = Operand::new(OperandType::LargeConstant, 0x3456);
        let o_variable1 = Operand::new(OperandType::Variable, 0x80);
        let o_variable2 = Operand::new(OperandType::Variable, 0x81);
        let i = mock_instruction(
            0x480,
            vec![o_variable1, o_large_constant, o_small_constant, o_variable2],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0,
        );

        let operands = operand_values(&mut zmachine, &i);
        assert!(operands.is_ok());
        let o = operands.unwrap();
        assert_eq!(o[0], 0x789A);
        assert_eq!(o[1], 0x3456);
        assert_eq!(o[2], 0x12);
        assert_eq!(o[3], 0x1357);

        let i = mock_instruction(
            0x480,
            vec![],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0,
        );
        let operands = operand_values(&mut zmachine, &i);
        assert!(operands.is_ok());
        assert!(operands.unwrap().is_empty());
    }

    #[test]
    fn test_branch_rfalse() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(true, 0, 0x502);
        let a = processor::branch(&mut zmachine, &i, true);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x480);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0);
    }

    #[test]
    fn test_branch_rfalse_no_branch() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(false, 0, 0x502);
        let a = processor::branch(&mut zmachine, &i, true);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x502);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0xFF);
    }

    #[test]
    fn test_branch_rtrue() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(true, 1, 0x502);
        let a = processor::branch(&mut zmachine, &i, true);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x480);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 1);
    }

    #[test]
    fn test_branch_rtrue_no_branch() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        mock_frame(&mut zmachine, 0x500, Some(0x80), 0x480);
        let i = mock_branch(false, 1, 0x502);
        let a = processor::branch(&mut zmachine, &i, true);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x502);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0xFF);
    }

    #[test]
    fn test_branch() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        let i = mock_branch(true, 0x500, 0x482);
        let a = processor::branch(&mut zmachine, &i, true);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x500);
    }

    #[test]
    fn test_branch_no_branch() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        let i = mock_branch(true, 0x500, 0x482);
        let a = processor::branch(&mut zmachine, &i, false);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x482);
    }

    #[test]
    fn test_branch_not_a_branch_instruction() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(
            0x480,
            vec![],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0x482,
        );
        let a = processor::branch(&mut zmachine, &i, false);
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x482);
    }

    #[test]
    fn test_store_result() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_store_result(Some(0x80), 0x482);
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0x12);
    }

    #[test]
    fn test_store_result_no_result() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_store_result(None, 0x482);
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0xFF);
    }

    #[test]
    fn test_store_result_not_a_store_result_instruction() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let i = mock_instruction(
            0x480,
            vec![],
            Opcode::new(5, 0, 0, OpcodeForm::Var, OperandCount::_VAR),
            0x482,
        );
        let a = store_result(&mut zmachine, &i, 0x12);
        assert!(a.is_ok());
    }

    #[test]
    fn test_call_fn_rfalse() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let a = call_fn(
            &mut zmachine,
            0,
            0x482,
            &vec![],
            Some(StoreResult::new(0, 0x80)),
        );
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x482);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0x00);
    }

    #[test]
    fn test_call_fn_rtrue() {
        let mut v = test_map(5);
        set_variable(&mut v, 0x80, 0xFF);
        let mut zmachine = mock_zmachine(v);
        let a = call_fn(
            &mut zmachine,
            1,
            0x482,
            &vec![],
            Some(StoreResult::new(0, 0x80)),
        );
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x482);
        let v = zmachine.variable(0x80);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), 0x01);
    }

    #[test]
    fn test_call_fn() {
        let v = test_map(5);
        let mut zmachine = mock_zmachine(v);
        assert_eq!(zmachine.frame_count(), 1);
        let a = call_fn(
            &mut zmachine,
            0x500,
            0x482,
            &vec![],
            Some(StoreResult::new(0, 0x80)),
        );
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), 0x501);
        assert_eq!(zmachine.frame_count(), 2);
    }
}
