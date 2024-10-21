use chrono::{DateTime, Local};
use eframe::egui::{self, vec2, Color32, FontId, Hyperlink, RichText};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use serde::{Deserialize, Serialize};

use crate::{
    assembler::assemble,
    colors::MfColors,
    icons::material_design_icons::{
        MDI_ALERT, MDI_CLOCK_FAST, MDI_CLOSE_OCTAGON, MDI_CONTENT_COPY, MDI_EXPORT,
        MDI_HELP_CIRCLE_OUTLINE, MDI_IMPORT, MDI_OCTAGON, MDI_PACKAGE_VARIANT_CLOSED_REMOVE,
        MDI_PLAY, MDI_RESTORE, MDI_STEP_FORWARD, MDI_STOP,
    },
    init, AssemblerError, ExecutionInfo, ExecutionState, MemoryData, Opcode, Operand, Register,
};

const DEFAULT_PROGRAM: &str = "loop:
    LDX string
    OUT
    INC IX
    LDD count
    DEC ACC
    STO count
    CMP #0
    JPN loop
    END

count:  #12

string:
        &48
        &65
        &6c
        &6c
        &6f
        &2c
        &20
        &77
        &6f
        &72
        &6c
        &64
";

#[derive(Serialize, Deserialize)]
struct AppContext {
    source_code: String,
    memory: [[MemoryData; 16]; 16],
    program_load_location: u16,
    pc: u16,
    cir: (Opcode, Operand),
    ix: u16,
    mdr: MemoryData,
    mar: u16,
    acc: u16,
    carry: bool,
    zero: bool,
    overflow: bool,
    sign: bool,
    output: String,
    input: String,
    execution_state: ExecutionState,
    highlight_pc_location: bool,
    pc_highlight_color: [u8; 3],
    assembler_error: Option<AssemblerError>,
    show_assembler_error_window: bool,
    show_assembler_info_window: bool,
    value_as_hex: bool,

    ins_executed: u64,

    clock_speed: u16,
    execution_info: Option<ExecutionInfo>,
    last_step_time: DateTime<Local>,
}

impl egui_dock::TabViewer for AppContext {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab.as_str() {
            "Source Editor" => self.source_editor(ui),
            "Console" => self.console(ui),
            "Registers" => self.registers(ui),
            "Memory" => self.memory(ui),
            _ => {
                ui.label("There is nothing here...\nYou see this because of a bug. Please report this to Micfong.");
            }
        }
    }
}

impl AppContext {
    fn source_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
                    ui.label("Load program to");
                    ui.add(
                        if self.value_as_hex {
                            egui::DragValue::new(&mut self.program_load_location)
                                .speed(1.0)
                                .range(0..=255)
                                .hexadecimal(2, false, true)
                        } else {
                            egui::DragValue::new(&mut self.program_load_location).speed(1.0).range(0..=255)
                        }
                    );
                    ui.colored_label(MfColors::GRAY_700, MDI_HELP_CIRCLE_OUTLINE)
                        .on_hover_text("Program needs to be loaded in the memory before execution. This is the memory address where the first line of your compiled program will be loaded. Usually, this is kept consistent with where the program is executed from.");
                    ui.separator();
                    if ui.button("Assemble and load").clicked() {
                        match assemble(&self.source_code) {
                            Ok(program) => {
                                if program.len() > 256 - self.program_load_location as usize {
                                    self.assembler_error = Some(AssemblerError::ProgramTooLong {
                                        program_size: program.len(),
                                        memory_available: 256 - self.program_load_location as usize,
                                    });
                                    self.show_assembler_error_window = true;
                                } else {
                                    for (i, &instruction) in program.iter().enumerate() {
                                        self.memory[(self.program_load_location as usize + i) / 16][(self.program_load_location as usize + i) % 16] =
                                            match instruction.0 {
                                                Opcode::Data(v) => {
                                                    MemoryData::Value(v)
                                                }
                                                _ => {
                                                    if let Operand::Address(v) = instruction.1 {
                                                        MemoryData::Instruction(instruction.0, Operand::Address(v + self.program_load_location))
                                                    } else {
                                                        MemoryData::Instruction(instruction.0, instruction.1)
                                                    }
                                                }
                                            }
                                    }
                                }
                            }
                            Err(e) => {
                                self.assembler_error = Some(e);
                                self.show_assembler_error_window = true;
                            }
                        }
                    };
                });
        ui.add(
            egui::TextEdit::multiline(&mut self.source_code)
                .code_editor()
                .desired_rows(10)
                .desired_width(f32::INFINITY),
        );
    }

    fn console(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .code_editor()
                .desired_width(f32::INFINITY)
                .interactive(false),
        );
        ui.add_enabled_ui(
            (self.execution_state == ExecutionState::ExecutingAwaitingInput)
                || (self.execution_state == ExecutionState::SteppingAwaitingInput),
            |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .char_limit(1)
                        .desired_width(f32::INFINITY)
                        .hint_text("Console input"),
                );
                ui.add_enabled_ui(!self.input.is_empty(), |ui| {
                    if ui.button("Send").clicked() {
                        match self.execution_state {
                            ExecutionState::ExecutingAwaitingInput => {
                                self.execution_state = ExecutionState::Executing;
                            }
                            ExecutionState::SteppingAwaitingInput => {
                                self.execution_state = ExecutionState::Stopped
                            }
                            _ => unreachable!(),
                        }
                        self.acc = self.input.chars().next().unwrap() as u16;
                        self.input.clear();
                    }
                });
            },
        );
    }

    fn registers(&mut self, ui: &mut egui::Ui) {
        ui.colored_label(
            MfColors::GRAY_700,
            "For CIR: Hover on opcode to see full instruction.",
        );
        ui.horizontal(|ui| self.show_register_grid(ui));
    }

    fn show_register_grid(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        egui::Grid::new("register_grid")
            .num_columns(2)
            .spacing(vec2(0.0, 2.0))
            .show(ui, |ui| {
                ui.label("PC");
                ui.add(if self.value_as_hex {
                    egui::DragValue::new(&mut self.pc)
                        .speed(1.0)
                        .range(0..=255)
                        .hexadecimal(2, false, true)
                } else {
                    egui::DragValue::new(&mut self.pc).speed(1.0).range(0..=255)
                });
                ui.end_row();

                ui.label("CIR");
                ui.label(self.cir.0.to_string())
                    .on_hover_text(format!("{} {}", self.cir.0, self.cir.1));
                ui.end_row();

                ui.label("IX");
                ui.add(if self.value_as_hex {
                    egui::DragValue::new(&mut self.ix)
                        .speed(1.0)
                        .range(0..=255)
                        .hexadecimal(2, false, true)
                } else {
                    egui::DragValue::new(&mut self.ix).speed(1.0).range(0..=255)
                });
                ui.end_row();

                ui.label("MDR");
                match &mut self.mdr {
                    MemoryData::Instruction(opcode, operand) => {
                        ui.label(opcode.to_string())
                            .on_hover_text(format!("{} {}", opcode, operand));
                    }
                    MemoryData::Value(v) => {
                        ui.add(if self.value_as_hex {
                            egui::DragValue::new(v)
                                .speed(1.0)
                                .range(0..=0xffff)
                                .hexadecimal(4, false, true)
                        } else {
                            egui::DragValue::new(v).speed(1.0).range(0..=0xffff)
                        });
                    }
                }
                ui.end_row();

                ui.label("MAR");
                ui.add(if self.value_as_hex {
                    egui::DragValue::new(&mut self.mar)
                        .speed(1.0)
                        .range(0..=255)
                        .hexadecimal(2, false, true)
                } else {
                    egui::DragValue::new(&mut self.mar)
                        .speed(1.0)
                        .range(0..=255)
                });
                ui.end_row();

                ui.label("ACC");
                ui.add(if self.value_as_hex {
                    egui::DragValue::new(&mut self.acc)
                        .speed(1.0)
                        .range(0..=0xffff)
                        .hexadecimal(4, false, true)
                } else {
                    egui::DragValue::new(&mut self.acc)
                        .speed(1.0)
                        .range(0..=0xffff)
                });
                ui.end_row();
            });
        ui.separator();
        ui.vertical(|ui| {
            ui.toggle_value(&mut self.carry, "Carry flag");
            ui.toggle_value(&mut self.zero, "Zero flag");
            ui.toggle_value(&mut self.overflow, "Overflow flag");
            ui.toggle_value(&mut self.sign, "Sign flag");
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.highlight_pc_location,
                    "Highlight PC index in memory",
                );
                ui.color_edit_button_srgb(&mut self.pc_highlight_color);
            });
            ui.horizontal(|ui| {
                ui.label("Show values in");
                ui.radio_value(&mut self.value_as_hex, true, "hex");
                ui.radio_value(&mut self.value_as_hex, false, "dec");
            });
        })
    }

    fn memory(&mut self, ui: &mut egui::Ui) {
        ui.colored_label(MfColors::GRAY_700, "Hover on any cell to see details.");
        egui::Grid::new("memroy_grid")
            .num_columns(17)
            .spacing(vec2(2.0, 2.0))
            .show(ui, |ui| {
                ui.style_mut().spacing.interact_size = vec2(30.0, 18.0);
                ui.label("");
                for i in 0..16 {
                    ui.label(format!("{:02X}", i));
                }
                ui.end_row();
                for i in 0..16 {
                    ui.label(format!("{:02X}", i));
                    for j in 0..16 {
                        match &mut self.memory[i][j] {
                            MemoryData::Instruction(opcode, operand) => {
                                if self.highlight_pc_location && i * 16 + j == self.pc as usize {
                                    ui.colored_label(
                                        Color32::from_rgb(
                                            self.pc_highlight_color[0],
                                            self.pc_highlight_color[1],
                                            self.pc_highlight_color[2],
                                        ),
                                        opcode.to_string(),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(format!("{} {}", opcode, operand));
                                        ui.separator();
                                        ui.label(format!(
                                            "Address: {:X}{:X}₁₆ = {}₁₀",
                                            i,
                                            j,
                                            i * 16 + j
                                        ));
                                    });
                                } else {
                                    ui.label(opcode.to_string()).on_hover_ui(|ui| {
                                        ui.label(format!("{} {}", opcode, operand));
                                        ui.separator();
                                        ui.label(format!(
                                            "Address: {:X}{:X}₁₆ = {}₁₀",
                                            i,
                                            j,
                                            i * 16 + j
                                        ));
                                    });
                                }
                            }
                            MemoryData::Value(v) => {
                                let original_color =
                                    ui.style().visuals.widgets.inactive.fg_stroke.color;
                                if self.highlight_pc_location && i * 16 + j == self.pc as usize {
                                    ui.style_mut().visuals.widgets.inactive.fg_stroke.color =
                                        Color32::from_rgb(
                                            self.pc_highlight_color[0],
                                            self.pc_highlight_color[1],
                                            self.pc_highlight_color[2],
                                        );
                                }
                                ui.add(if self.value_as_hex {
                                    egui::DragValue::new(v)
                                        .speed(1.0)
                                        .range(0..=0xffff)
                                        .hexadecimal(4, false, true)
                                } else {
                                    egui::DragValue::new(v).speed(1.0).range(0..=0xffff)
                                })
                                .on_hover_ui(|ui| {
                                    ui.label(format!("Value: {:04X}₁₆ = {}₁₀", v, v));
                                    ui.separator();
                                    ui.label(format!(
                                        "Address: {:X}{:X}₁₆ = {}₁₀",
                                        i,
                                        j,
                                        i * 16 + j
                                    ));
                                });
                                ui.style_mut().visuals.widgets.inactive.fg_stroke.color =
                                    original_color;
                            }
                        }
                    }
                    ui.end_row();
                }
            });
    }

    fn step(&mut self) {
        if self.ins_executed >= 1000 {
            self.execution_info = Some(ExecutionInfo::TooManySteps {
                steps: self.ins_executed,
            });
            self.show_assembler_info_window = true;
        }
        self.mar = self.pc;
        self.mdr = self.memory.as_flattened()[self.pc as usize];
        let current_instruction = match &self.memory.as_flattened()[self.pc as usize] {
            MemoryData::Instruction(opcode, operand) => (*opcode, *operand),
            MemoryData::Value(v) => {
                if *v == 0 {
                    self.execution_info = Some(ExecutionInfo::ExecutionTerminated {
                        ins_address: self.pc,
                    });
                    self.execution_state = ExecutionState::Stopped;
                } else {
                    self.execution_info = Some(ExecutionInfo::ExecutionAbortedValueMet {
                        ins_address: self.pc,
                        value: *v,
                    });
                    self.execution_state = ExecutionState::Stopped;
                }
                self.show_assembler_info_window = true;
                return;
            }
        };
        self.cir = current_instruction;
        let cur_ins_add = self.pc;
        self.pc += 1;
        match current_instruction.0 {
            Opcode::Ldm => {
                self.acc = match current_instruction.1 {
                    Operand::Immediate(v) => v,
                    _ => unreachable!(),
                };
            }
            Opcode::Ldd => {
                self.acc = match current_instruction.1 {
                    Operand::Address(a) => {
                        self.mar = a;
                        match self.memory.as_flattened()[a as usize] {
                            MemoryData::Value(v) => {
                                self.mdr = MemoryData::Value(v);
                                v
                            }
                            MemoryData::Instruction(_, _) => {
                                self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                    ins_address: cur_ins_add,
                                    requested_address: a,
                                });
                                self.execution_state = ExecutionState::Stopped;
                                self.show_assembler_info_window = true;
                                return;
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Ldi => {
                self.acc = match current_instruction.1 {
                    Operand::Address(a) => {
                        self.mar = a;
                        match self.memory.as_flattened()[a as usize] {
                            MemoryData::Value(v) => {
                                self.mdr = MemoryData::Value(v);
                                self.mar = v;
                                if v > 255 {
                                    self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                                        ins_address: cur_ins_add,
                                        requested_address: v,
                                    });
                                    self.execution_state = ExecutionState::Stopped;
                                    self.show_assembler_info_window = true;
                                    return;
                                }
                                match self.memory.as_flattened()[v as usize] {
                                    MemoryData::Value(v) => v,
                                    MemoryData::Instruction(_, _) => {
                                        self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                            ins_address: cur_ins_add,
                                            requested_address: v,
                                        });
                                        self.execution_state = ExecutionState::Stopped;
                                        self.show_assembler_info_window = true;
                                        return;
                                    }
                                }
                            }
                            MemoryData::Instruction(_, _) => {
                                self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                    ins_address: cur_ins_add,
                                    requested_address: a,
                                });
                                self.execution_state = ExecutionState::Stopped;
                                self.show_assembler_info_window = true;
                                return;
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Ldx => {
                self.acc = match current_instruction.1 {
                    Operand::Address(a) => {
                        self.mar = a + self.ix;
                        if a + self.ix > 255 {
                            self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                                ins_address: cur_ins_add,
                                requested_address: a + self.ix,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                            return;
                        }
                        match self.memory.as_flattened()[(a + self.ix) as usize] {
                            MemoryData::Value(v) => {
                                self.mdr = MemoryData::Value(v);
                                v
                            }
                            MemoryData::Instruction(_, _) => {
                                self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                    ins_address: cur_ins_add,
                                    requested_address: a + self.ix,
                                });
                                self.execution_state = ExecutionState::Stopped;
                                self.show_assembler_info_window = true;
                                return;
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Ldr => {
                self.ix = match current_instruction.1 {
                    Operand::Immediate(v) => v,
                    _ => unreachable!(),
                };
            }
            Opcode::Mov => match current_instruction.1 {
                Operand::Register(r) => match r {
                    Register::Ix => {
                        self.ix = self.acc;
                    }
                    Register::Acc => (),
                },
                _ => unreachable!(),
            },
            Opcode::Sto => match current_instruction.1 {
                Operand::Address(a) => {
                    self.mar = a;
                    self.mdr = MemoryData::Value(self.acc);
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.memory.as_flattened_mut()[a as usize] = MemoryData::Value(self.acc);
                }
                _ => unreachable!(),
            },
            Opcode::Add => match current_instruction.1 {
                Operand::Immediate(v) => {
                    let (result, overflow) = self.acc.overflowing_add(v);
                    self.acc = result;
                    self.carry = overflow;
                    self.zero = result == 0;
                    self.overflow = overflow;
                    self.sign = result & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            let (result, overflow) = self.acc.overflowing_add(v);
                            self.acc = result;
                            self.carry = overflow;
                            self.zero = result == 0;
                            self.overflow = overflow;
                            self.sign = result & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unreachable!(),
            },
            Opcode::Sub => match current_instruction.1 {
                Operand::Immediate(v) => {
                    let (result, overflow) = self.acc.overflowing_sub(v);
                    self.acc = result;
                    self.carry = overflow;
                    self.zero = result == 0;
                    self.overflow = overflow;
                    self.sign = result & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            let (result, overflow) = self.acc.overflowing_sub(v);
                            self.acc = result;
                            self.carry = overflow;
                            self.zero = result == 0;
                            self.overflow = overflow;
                            self.sign = result & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unreachable!(),
            },
            Opcode::Inc => match current_instruction.1 {
                Operand::Register(r) => match r {
                    // Add 1 to the destination register, while preserving CF
                    Register::Ix => {
                        let (result, overflow) = self.ix.overflowing_add(1);
                        self.ix = result;
                        self.zero = result == 0;
                        self.overflow = overflow;
                        self.sign = result & 0x8000 != 0;
                    }
                    Register::Acc => {
                        let (result, overflow) = self.acc.overflowing_add(1);
                        self.acc = result;
                        self.zero = result == 0;
                        self.overflow = overflow;
                        self.sign = result & 0x8000 != 0;
                    }
                },
                _ => unreachable!(),
            },
            Opcode::Dec => match current_instruction.1 {
                Operand::Register(r) => match r {
                    // Subtract 1 from the destination register, while preserving CF
                    Register::Ix => {
                        let (result, overflow) = self.ix.overflowing_sub(1);
                        self.ix = result;
                        self.zero = result == 0;
                        self.overflow = overflow;
                        self.sign = result & 0x8000 != 0;
                    }
                    Register::Acc => {
                        let (result, overflow) = self.acc.overflowing_sub(1);
                        self.acc = result;
                        self.zero = result == 0;
                        self.overflow = overflow;
                        self.sign = result & 0x8000 != 0;
                    }
                },
                _ => unreachable!(),
            },
            Opcode::Jmp => match current_instruction.1 {
                Operand::Address(a) => {
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.pc = a;
                }
                _ => unreachable!(),
            },
            Opcode::Cmp => match current_instruction.1 {
                Operand::Immediate(v) => {
                    let (result, overflow) = self.acc.overflowing_sub(v);
                    self.carry = overflow;
                    self.zero = result == 0;
                    self.overflow = overflow;
                    self.sign = result & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            let (result, overflow) = self.acc.overflowing_sub(v);
                            self.carry = overflow;
                            self.zero = result == 0;
                            self.overflow = overflow;
                            self.sign = result & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unreachable!(),
            },
            Opcode::Cmi => match current_instruction.1 {
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.mar = v;
                            if v > 255 {
                                self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                                    ins_address: cur_ins_add,
                                    requested_address: v,
                                });
                                self.execution_state = ExecutionState::Stopped;
                                self.show_assembler_info_window = true;
                                return;
                            }
                            self.mdr = self.memory.as_flattened()[v as usize];
                            match self.mdr {
                                MemoryData::Value(v) => {
                                    let (result, overflow) = self.acc.overflowing_sub(v);
                                    self.carry = overflow;
                                    self.zero = result == 0;
                                    self.overflow = overflow;
                                    self.sign = result & 0x8000 != 0;
                                }
                                MemoryData::Instruction(_, _) => {
                                    self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                        ins_address: cur_ins_add,
                                        requested_address: v,
                                    });
                                    self.execution_state = ExecutionState::Stopped;
                                    self.show_assembler_info_window = true;
                                }
                            }
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unreachable!(),
            },
            Opcode::Jpe => match current_instruction.1 {
                Operand::Address(a) => {
                    if self.zero {
                        if a > 255 {
                            self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                            return;
                        }
                        self.pc = a;
                    }
                }
                _ => unreachable!(),
            },
            Opcode::Jpn => match current_instruction.1 {
                Operand::Address(a) => {
                    if !self.zero {
                        if a > 255 {
                            self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                            return;
                        }
                        self.pc = a;
                    }
                }
                _ => unreachable!(),
            },
            Opcode::In => {
                if self.execution_state == ExecutionState::Executing {
                    self.execution_state = ExecutionState::ExecutingAwaitingInput;
                } else {
                    self.execution_state = ExecutionState::SteppingAwaitingInput;
                }
            }
            Opcode::Out => {
                if let Some(c) = std::char::from_u32(self.acc as u32) {
                    self.output.push(c);
                } else {
                    self.output.push('�');
                }
            }
            Opcode::End => {
                self.execution_info = Some(ExecutionInfo::ExecutionTerminated {
                    ins_address: cur_ins_add,
                });
                self.execution_state = ExecutionState::Stopped;
                self.show_assembler_info_window = true;
            }
            Opcode::And => match current_instruction.1 {
                Operand::Immediate(v) => {
                    self.acc &= v;
                    self.zero = self.acc == 0;
                    self.sign = self.acc & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.acc &= v;
                            self.zero = self.acc == 0;
                            self.sign = self.acc & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Opcode::Xor => match current_instruction.1 {
                Operand::Immediate(v) => {
                    self.acc ^= v;
                    self.zero = self.acc == 0;
                    self.sign = self.acc & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.acc ^= v;
                            self.zero = self.acc == 0;
                            self.sign = self.acc & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Opcode::Or => match current_instruction.1 {
                Operand::Immediate(v) => {
                    self.acc |= v;
                    self.zero = self.acc == 0;
                    self.sign = self.acc & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.acc |= v;
                            self.zero = self.acc == 0;
                            self.sign = self.acc & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Opcode::Lsl => match current_instruction.1 {
                Operand::Immediate(v) => {
                    self.acc <<= v;
                    self.zero = self.acc == 0;
                    self.sign = self.acc & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.acc <<= v;
                            self.zero = self.acc == 0;
                            self.sign = self.acc & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Opcode::Lsr => match current_instruction.1 {
                Operand::Immediate(v) => {
                    self.acc >>= v;
                    self.zero = self.acc == 0;
                    self.sign = self.acc & 0x8000 != 0;
                }
                Operand::Address(a) => {
                    self.mar = a;
                    if a > 255 {
                        self.execution_info = Some(ExecutionInfo::AddressNotInMemory {
                            ins_address: cur_ins_add,
                            requested_address: a,
                        });
                        self.execution_state = ExecutionState::Stopped;
                        self.show_assembler_info_window = true;
                        return;
                    }
                    self.mdr = self.memory.as_flattened()[a as usize];
                    match self.mdr {
                        MemoryData::Value(v) => {
                            self.acc >>= v;
                            self.zero = self.acc == 0;
                            self.sign = self.acc & 0x8000 != 0;
                        }
                        MemoryData::Instruction(_, _) => {
                            self.execution_info = Some(ExecutionInfo::InvalidLoad {
                                ins_address: cur_ins_add,
                                requested_address: a,
                            });
                            self.execution_state = ExecutionState::Stopped;
                            self.show_assembler_info_window = true;
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Opcode::Data(_) => unreachable!(),
        }
    }
}

pub struct CaieAsmApp {
    tree: DockState<String>,
    context: AppContext,
    export_string: String,
    import_string: String,
    show_export_window: bool,
    show_import_window: bool,
    import_failed: bool,
}

impl Default for CaieAsmApp {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["Source Editor".to_owned()]);
        let [a, b] = tree.main_surface_mut().split_right(
            NodeIndex::root(),
            0.3,
            vec!["Registers".to_owned()],
        );
        let [_, _] = tree
            .main_surface_mut()
            .split_below(a, 0.7, vec!["Console".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(b, 0.3, vec!["Memory".to_owned()]);

        Self {
            tree,
            context: AppContext {
                source_code: DEFAULT_PROGRAM.to_string(),
                memory: [[MemoryData::Value(0); 16]; 16],
                program_load_location: 0,
                pc: 0,
                cir: (Opcode::End, Operand::Empty),
                ix: 0,
                mdr: MemoryData::Value(0),
                mar: 0,
                acc: 0,
                carry: false,
                zero: false,
                overflow: false,
                sign: false,
                output: String::new(),
                input: String::new(),
                execution_state: ExecutionState::Stopped,
                highlight_pc_location: true,
                pc_highlight_color: [236, 111, 39],
                assembler_error: None,
                show_assembler_error_window: false,
                show_assembler_info_window: false,
                value_as_hex: true,
                clock_speed: 4,
                execution_info: None,
                last_step_time: Local::now(),
                ins_executed: 0,
            },
            export_string: String::new(),
            import_string: String::new(),
            show_export_window: false,
            show_import_window: false,
            import_failed: false,
        }
    }
}

impl CaieAsmApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        init::setup_custom_fonts(&cc.egui_ctx);
        init::setup_custom_styles(&cc.egui_ctx);
        Default::default()
    }
}

impl eframe::App for CaieAsmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.context.execution_state == ExecutionState::Executing {
            if self.context.clock_speed == 0 {
                self.context.last_step_time = Local::now();
                self.context.step();
                self.context.ins_executed += 1;
            } else {
                let now = Local::now();
                let elapsed = now - self.context.last_step_time;
                if elapsed.num_milliseconds() as f32 >= 1000. / (self.context.clock_speed) as f32 {
                    self.context.step();
                    self.context.last_step_time = now;
                    self.context.ins_executed += 1;
                }
            }
            ctx.request_repaint();
        }

        if self.context.execution_state == ExecutionState::Stopped {
            self.context.ins_executed = 0;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button(MDI_EXPORT.to_owned() + " Export").clicked() {
                    self.show_export_window = true;
                    self.export_string = serde_json::to_string(&self.context).unwrap();
                }
                if ui.button(MDI_IMPORT.to_owned() + " Import").clicked() {
                    self.show_import_window = true;
                }
                ui.separator();
                ui.menu_button(MDI_CLOCK_FAST.to_owned() + " Clock speed", |ui| {
                    ui.radio_value(&mut self.context.clock_speed, 1, "1 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 2, "2 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 4, "4 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 8, "8 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 16, "16 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 32, "32 Hz");
                    ui.radio_value(&mut self.context.clock_speed, 0, "Unlimited");
                });
                ui.separator();

                if self.context.execution_state != ExecutionState::Stopped {
                    if ui.button(MDI_STOP.to_owned() + " Terminate").clicked() {
                        self.context.execution_state = ExecutionState::Stopped;
                    }
                } else if ui.button(MDI_PLAY.to_owned() + " Execute").clicked() {
                    self.context.execution_state = ExecutionState::Executing;
                }
                if ui.button(MDI_STEP_FORWARD.to_owned() + " Step").clicked() {
                    self.context.step();
                }
                ui.separator();
                if ui
                    .button(MDI_RESTORE.to_owned() + " Reset registers and memory")
                    .clicked()
                {
                    self.context.execution_state = ExecutionState::Stopped;
                    self.context.pc = 0;
                    self.context.cir = (Opcode::End, Operand::Empty);
                    self.context.ix = 0;
                    self.context.mdr = MemoryData::Value(0);
                    self.context.mar = 0;
                    self.context.acc = 0;
                    self.context.carry = false;
                    self.context.zero = false;
                    self.context.overflow = false;
                    self.context.sign = false;
                    self.context.output.clear();
                    self.context.memory = [[MemoryData::Value(0); 16]; 16];
                }

                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add(Hyperlink::from_label_and_url(
                            RichText::new("Micfong").color(MfColors::BLUE_300),
                            "https://micfong.space/",
                        ));
                        ui.label("CAIE Assembly Emulator by");
                    });
                    ui.separator();
                })
            });
        });

        egui::CentralPanel::default().show(ctx, |_ui| {
            ctx.style_mut(|style| {
                style.interaction.tooltip_delay = 0.0;
            });

            DockArea::new(&mut self.tree)
                .style(Style::from_egui(ctx.style().as_ref()))
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_close_buttons(false)
                .show(ctx, &mut self.context);
        });

        if self.context.show_assembler_info_window && self.context.execution_info.is_some() {
            let (title, icon, color, summary, content) = match self.context.execution_info.as_ref().unwrap() {
                ExecutionInfo::ExecutionTerminated { ins_address } => (
                    "Execution terminated",
                    MDI_OCTAGON,
                    MfColors::WHITE,
                    "Execution terminated.",
                    format!(
                        "Execution terminated at address {:X}₁₆ = {}₁₀, because the END instruction (value 0) was encountered.",
                        ins_address, ins_address
                    ),
                ),
                ExecutionInfo::ExecutionAbortedValueMet { ins_address, value } => (
                    "Aborted",
                    MDI_CLOSE_OCTAGON,
                    MfColors::RED_500,
                    "Execution aborted.", 
                    format!(
                        "Execution aborted at address {:X}₁₆ = {}₁₀, because the value {} was encountered, which is not an instruction.",
                        ins_address, ins_address, value
                    )
                ),
                ExecutionInfo::TooManySteps { steps } => (
                    "Warning",
                    MDI_ALERT,
                    MfColors::YELLOW_500,
                    "Many instructions executed.", 
                    format!(
                        "{} instructions have been executed. You may have mistakenly written a program that runs indefinitely.\n\nNote that this emulator runs entirely in your browser, so you really can't break anything.",
                        steps
                    )
                ),
                ExecutionInfo::AddressNotInMemory { ins_address, requested_address } => (
                    "Aborted",
                    MDI_CLOSE_OCTAGON,
                    MfColors::RED_500,
                    "Invalid address.", 
                    format!(
                        "Execution aborted at address {:X}₁₆ = {}₁₀, because the program attempted to access memory at address {:X}₁₆ = {}₁₀, which is not in the memory.",
                        ins_address, ins_address, requested_address, requested_address
                    )
                ),
                ExecutionInfo::InvalidLoad { ins_address, requested_address } => (
                    "Aborted",
                    MDI_CLOSE_OCTAGON,
                    MfColors::RED_500,
                    "Invalid load.", 
                    format!(
                        "Execution aborted at address {:X}₁₆ = {}₁₀, because the program attempted to load memory at address {:X}₁₆ = {}₁₀ to the ACC, which is an instruction.",
                        ins_address, ins_address, requested_address, requested_address
                    )
                ),
            };
            egui::Window::new(title)
                .open(&mut self.context.show_assembler_info_window)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(icon)
                                    .color(color)
                                    .font(FontId::proportional(32.0)),
                            );
                            ui.label(summary);
                            ui.separator();
                        },
                    );
                    ui.label(content);
                });
        }

        egui::Window::new("Assembler error")
            .open(&mut self.context.show_assembler_error_window)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                if let Some(e) = &self.context.assembler_error {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(MDI_PACKAGE_VARIANT_CLOSED_REMOVE)
                                    .color(MfColors::RED_500)
                                    .font(FontId::proportional(32.0)),
                            );
                            ui.label("The assembler reported an error.");
                            ui.separator();
                        },
                    );
                    ui.label(e.to_string());
                } else {
                    ui.label("The assembler is alright – no error found.");
                    ui.label("This is a bug. Please report this to Micfong.");
                }
            });

        egui::Window::new("Export")
            .open(&mut self.show_export_window)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(
                    "This is a sharable string that contains the current state of the emulator.",
                );
                ui.horizontal(|ui| {
                    if ui.button(MDI_RESTORE.to_owned() + " Refresh").clicked() {
                        self.export_string = serde_json::to_string(&self.context).unwrap();
                    }
                    if ui.button(MDI_CONTENT_COPY.to_owned() + " Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = self.export_string.clone());
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.export_string)
                                .code_editor()
                                .interactive(false),
                        );
                    });
            });

        egui::Window::new("Import")
            .open(&mut self.show_import_window)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Paste the exported text here to restore the state.");
                if self.import_failed {
                    ui.colored_label(
                        MfColors::RED_500,
                        "Failed to import. Errorneous JSON string.",
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.import_string).code_editor());
                    });
                if ui.button(MDI_IMPORT.to_owned() + " Import").clicked() {
                    match serde_json::from_str(&self.import_string) {
                        Ok(c) => {
                            self.context = c;
                        }
                        Err(_) => {
                            self.import_failed = true;
                        }
                    }
                }
            });
    }
}
