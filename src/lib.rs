mod app;
mod assembler;
mod colors;
pub mod icons;
mod init;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

pub use app::CaieAsmApp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum MemoryData {
    Instruction(Opcode, Operand),
    Value(u16),
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Opcode {
    Ldm,
    Ldd,
    Ldi,
    Ldx,
    Ldr,
    Mov,
    Sto,
    Add,
    Sub,
    Inc,
    Dec,
    Jmp,
    Cmp,
    Cmi,
    Jpe,
    Jpn,
    In,
    Out,
    End,
    And,
    Xor,
    Or,
    Lsl,
    Lsr,
    Data(u16),
}

impl TryFrom<&str> for Opcode {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "LDM" => Ok(Opcode::Ldm),
            "LDD" => Ok(Opcode::Ldd),
            "LDI" => Ok(Opcode::Ldi),
            "LDX" => Ok(Opcode::Ldx),
            "LDR" => Ok(Opcode::Ldr),
            "MOV" => Ok(Opcode::Mov),
            "STO" => Ok(Opcode::Sto),
            "ADD" => Ok(Opcode::Add),
            "SUB" => Ok(Opcode::Sub),
            "INC" => Ok(Opcode::Inc),
            "DEC" => Ok(Opcode::Dec),
            "JMP" => Ok(Opcode::Jmp),
            "CMP" => Ok(Opcode::Cmp),
            "CMI" => Ok(Opcode::Cmi),
            "JPE" => Ok(Opcode::Jpe),
            "JPN" => Ok(Opcode::Jpn),
            "IN" => Ok(Opcode::In),
            "OUT" => Ok(Opcode::Out),
            "END" => Ok(Opcode::End),
            "AND" => Ok(Opcode::And),
            "XOR" => Ok(Opcode::Xor),
            "OR" => Ok(Opcode::Or),
            "LSL" => Ok(Opcode::Lsl),
            "LSR" => Ok(Opcode::Lsr),
            _ => {
                if let Ok(Operand::Immediate(v)) = Operand::str_to_operand(s, &HashMap::new()) {
                    Ok(Opcode::Data(v))
                } else {
                    Err(())
                }
            }
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Operand {
    Register(Register),
    Address(u16),
    Immediate(u16),
    Empty,
}

impl Operand {
    #[allow(clippy::result_unit_err)]
    pub fn str_to_operand(s: &str, symbol_table: &HashMap<&str, usize>) -> Result<Self, ()> {
        if s == "IX" {
            return Ok(Operand::Register(Register::Ix));
        } else if s == "ACC" {
            return Ok(Operand::Register(Register::Acc));
        } else if let Some(s) = s.strip_prefix('&') {
            if let Ok(value) = u16::from_str_radix(s, 16) {
                return Ok(Operand::Immediate(value));
            } else {
                return Err(());
            }
        } else if let Some(s) = s.strip_prefix("B") {
            if let Ok(value) = u16::from_str_radix(s, 2) {
                return Ok(Operand::Immediate(value));
            } else {
                return Err(());
            }
        } else if let Some(s) = s.strip_prefix("#") {
            if let Ok(value) = s.parse::<u16>() {
                return Ok(Operand::Immediate(value));
            } else {
                return Err(());
            }
        } else if let Ok(value) = s.parse::<u16>() {
            return Ok(Operand::Address(value));
        } else if let Some(value) = symbol_table.get(s) {
            return Ok(Operand::Address(*value as u16));
        }
        Err(())
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Register {
    Ix,
    Acc,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    Executing,
    ExecutingAwaitingInput,
    SteppingAwaitingInput,
    Stopped,
}

#[derive(Serialize, Deserialize)]
pub enum ExecutionInfo {
    ExecutionTerminated {
        ins_address: u16,
    },
    ExecutionAbortedValueMet {
        ins_address: u16,
        value: u16,
    },
    TooManySteps {
        steps: u64,
    },
    AddressNotInMemory {
        ins_address: u16,
        requested_address: u16,
    },
    InvalidLoad {
        ins_address: u16,
        requested_address: u16,
    },
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum AssemblerError {
    #[error("too many operands on line {line_index}: found {operands_found} operands")]
    TooManyOperands {
        line_index: usize,
        operands_found: usize,
    },
    #[error("unknown opcode on line {line_index}: {opcode}")]
    UnknownOpcode { line_index: usize, opcode: String },
    #[error("malformed operand on line {line_index}: {operand}")]
    MalformedOperand { line_index: usize, operand: String },
    #[error(
        "redunant operand on line {line_index}: {opcode} does not need an operand, but {operand} is given"
    )]
    RedundantOperand {
        line_index: usize,
        opcode: Opcode,
        operand: Operand,
    },
    #[error(
        "incorrect operand on line {line_index}: {opcode} expects an operand of type {operand_type_expected}, but {operand_given} is given"
    )]
    IncorrectOperand {
        line_index: usize,
        opcode: Opcode,
        operand_given: Operand,
        operand_type_expected: String,
    },
    #[error("missing operand on line {line_index}: {opcode} expects an operand")]
    MissingOperand { line_index: usize, opcode: Opcode },
    #[error("program too long: program size is {program_size}, but only {memory_available} unit of memory space is available")]
    ProgramTooLong {
        program_size: usize,
        memory_available: usize,
    },
}

impl Debug for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Ldm => write!(f, "LDM"),
            Opcode::Ldd => write!(f, "LDD"),
            Opcode::Ldi => write!(f, "LDI"),
            Opcode::Ldx => write!(f, "LDX"),
            Opcode::Ldr => write!(f, "LDR"),
            Opcode::Mov => write!(f, "MOV"),
            Opcode::Sto => write!(f, "STO"),
            Opcode::Add => write!(f, "ADD"),
            Opcode::Sub => write!(f, "SUB"),
            Opcode::Inc => write!(f, "INC"),
            Opcode::Dec => write!(f, "DEC"),
            Opcode::Jmp => write!(f, "JMP"),
            Opcode::Cmp => write!(f, "CMP"),
            Opcode::Cmi => write!(f, "CMI"),
            Opcode::Jpe => write!(f, "JPE"),
            Opcode::Jpn => write!(f, "JPN"),
            Opcode::In => write!(f, "IN"),
            Opcode::Out => write!(f, "OUT"),
            Opcode::End => write!(f, "END"),
            Opcode::And => write!(f, "AND"),
            Opcode::Xor => write!(f, "XOR"),
            Opcode::Or => write!(f, "OR"),
            Opcode::Lsl => write!(f, "LSL"),
            Opcode::Lsr => write!(f, "LSR"),
            Opcode::Data(v) => write!(f, "{}", v),
        }
    }
}

impl Debug for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Register(arg0) => f.debug_tuple("Register").field(arg0).finish(),
            Self::Address(arg0) => f.debug_tuple("Address").field(arg0).finish(),
            Self::Immediate(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::Empty => f.write_str(""),
        }
    }
}

impl Debug for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::Ix => write!(f, "IX"),
            Register::Acc => write!(f, "ACC"),
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
