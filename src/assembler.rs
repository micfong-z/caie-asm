use std::collections::HashMap;

use crate::{AssemblerError, Opcode, Operand};

pub fn assemble(source: &str) -> Result<Vec<(Opcode, Operand)>, AssemblerError> {
    let lines = source.lines();
    let mut symbol_table: HashMap<&str, usize> = HashMap::new();
    let mut memory_offset = 0;
    let mut result = Vec::new();

    // First pass: create symbol table
    for line in lines.clone() {
        let line = line.split("//").next().unwrap_or("").trim();
        let mut parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        if parts[0].ends_with(':') {
            symbol_table.insert(parts[0].trim_end_matches(":"), memory_offset);
            parts.remove(0);
        }
        if !parts.is_empty() {
            memory_offset += 1
        }
    }

    // Second pass: assemble
    for (line_index, line) in lines.enumerate() {
        let line = line.split("//").next().unwrap_or("").trim();
        let mut parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        if parts[0].ends_with(':') {
            parts.remove(0);
        }
        if parts.is_empty() {
            continue;
        }
        if parts.len() > 2 {
            return Err(AssemblerError::TooManyOperands {
                line_index: line_index + 1,
                operands_found: parts.len() - 1,
            });
        }
        let opcode = match Opcode::try_from(parts[0]) {
            Ok(opcode) => opcode,
            Err(_) => {
                return Err(AssemblerError::UnknownOpcode {
                    line_index: line_index + 1,
                    opcode: parts[0].to_string(),
                })
            }
        };
        if parts.len() == 1 {
            match opcode {
                Opcode::In => {
                    result.push((opcode, Operand::Empty));
                }
                Opcode::Out => {
                    result.push((opcode, Operand::Empty));
                }
                Opcode::End => {
                    result.push((opcode, Operand::Empty));
                }
                Opcode::Data(v) => {
                    result.push((opcode, Operand::Immediate(v)));
                }
                _ => {
                    return Err(AssemblerError::MissingOperand {
                        line_index: line_index + 1,
                        opcode,
                    })
                }
            }
        } else {
            let operand = match Operand::str_to_operand(parts[1], &symbol_table) {
                Ok(operand) => operand,
                Err(_) => {
                    return Err(AssemblerError::MalformedOperand {
                        line_index: line_index + 1,
                        operand: parts[1].to_string(),
                    })
                }
            };
            match opcode {
                Opcode::Ldm => {
                    if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Number".to_string(),
                        });
                    }
                }
                Opcode::Ldd => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Ldi => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Ldx => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Ldr => {
                    if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Number".to_string(),
                        });
                    }
                }
                Opcode::Mov => {
                    if let Operand::Register(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Register".to_string(),
                        });
                    }
                }
                Opcode::Sto => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Add => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Sub => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Inc => {
                    if let Operand::Register(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Register".to_string(),
                        });
                    }
                }
                Opcode::Dec => {
                    if let Operand::Register(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Register".to_string(),
                        });
                    }
                }
                Opcode::Jmp => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Cmp => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Cmi => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Jpe => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::Jpn => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address".to_string(),
                        });
                    }
                }
                Opcode::And => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Xor => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Or => {
                    if let Operand::Address(_) = operand {
                        result.push((opcode, operand));
                    } else if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Address/Number".to_string(),
                        });
                    }
                }
                Opcode::Lsl => {
                    if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Number".to_string(),
                        });
                    }
                }
                Opcode::Lsr => {
                    if let Operand::Immediate(_) = operand {
                        result.push((opcode, operand));
                    } else {
                        return Err(AssemblerError::IncorrectOperand {
                            line_index: line_index + 1,
                            opcode,
                            operand_given: operand,
                            operand_type_expected: "Number".to_string(),
                        });
                    }
                }
                _ => {
                    return Err(AssemblerError::RedundantOperand {
                        line_index: line_index + 1,
                        opcode,
                        operand,
                    });
                }
            }
        }
    }
    Ok(result)
}
