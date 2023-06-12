use std::fmt;

pub mod decoder;

pub enum OpcodeForm {
    Short,
    Long,
    Var,
    Ext,
}

#[derive(Debug)]
pub enum OperandType {
    LargeConstant,
    SmallConstant,
    Variable,
    Omitted,
}

struct Branch {
    condition: bool,
    dest: usize,
}

pub struct Instruction {
    address: usize,
    bytes: Vec<u8>,
    opcode: u8,
    ext_opcode: Option<u8>,
    opcode_form: OpcodeForm,
    opcode_name: String,
    operand_types: Vec<OperandType>,
    operands: Vec<u16>,
    store: Option<u8>,
    branch: Option<Branch>,
    pub next_pc: usize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${:05x}:", self.address)?;
        for b in &self.bytes {
            write!(f, " {:02x}", b)?;
        }
        for i in 0..9-self.bytes.len() {
            write!(f, "   ")?;
        }
        write!(f, "  {:15} ", self.opcode_name.to_uppercase())?;
        for i in 0..self.operand_types.len() {
            if i > 0 {
                write!(f, ",")?;
            }
            match self.operand_types[i] {
                OperandType::LargeConstant => write!(f, "#{:04x}", self.operands[i])?,
                OperandType::SmallConstant => write!(f, "#{:02x}", self.operands[i])?,
                OperandType::Variable => {
                    if self.operands[i] == 0 {
                        write!(f, "(SP)+")?
                    } else if self.operands[i] < 16 {
                        write!(f, "L{:02x}", self.operands[i] - 1)?
                    } else {
                        write!(f, "G{:02x}", self.operands[i] - 16)?
                    }
                }
                OperandType::Omitted => {}
            }
        }

        match &self.store {
            Some(s) => {
                write!(f, " -> ")?;
                if *s == 0 {
                    write!(f, "-(SP)")?
                } else if *s < 16 {
                    write!(f, "L{:02x}", *s - 1)?
                } else {
                    write!(f, "G{:02x}", *s - 16)?
                }
            }
            None => {}
        }
        match &self.branch {
            Some(b) => write!(f, " [{}] ${:05x}", b.condition.to_string().to_uppercase(), b.dest)?,
            None => {}
        }

        write!(f, "")
    }
}
