use clap::Parser;
use std::fs;
use std::iter::{Enumerate, Peekable};
use std::slice::Iter;

mod cli;
mod cpu_state;

use cli::Args;
use cpu_state::*;

type AsmBuffer<'a> = Peekable<Enumerate<Iter<'a, u8>>>;

fn main() {
    let args = Args::parse();
    let file_path = args.asm_bin_path;
    let output_file = args.output_file;
    println!("Selected file: {}", file_path);

    let should_sim = args.sim;

    let file_buffer = fs::read(file_path).expect("Unable to open file");
    let mut buf_iter: AsmBuffer = file_buffer.iter().enumerate().peekable();

    // Final assembled string of the file - mutated over the course of the loop
    let mut assembled_file_str = "bits 16\n\n".to_string();
    // Initialize empty registers
    let mut cpu_state = CpuState::new();

    // Loop through the buffer
    while let Some((i, byte)) = buf_iter.next() {
        let byte = *byte;

        let first_four_bits = (byte >> 4) & 0b1111_u8;
        let first_six_bits = (byte >> 2) & 0b111111_u8;
        let first_seven_bits = (byte >> 1) & 0b1111111_u8;
        let first_full_byte = byte;

        // Checking the first four bits
        match first_four_bits {
            0b1011 => {
                println!("Found an immediate-to-register instruction at index {}", i);
                let w_field = (byte >> 3) & 0b1_u8;
                let reg_field = byte & 0b111;

                match w_field {
                    0b0 => {
                        let data = buf_iter.next().unwrap().1;
                        let reg = decode_register_field(reg_field, false);

                        assembled_file_str.push_str(&format!("mov {}, {}\n", reg, data));

                        if should_sim {
                            let current_reg_value = cpu_state.get_register_value(reg);
                            let new_reg_value = *data;
                            cpu_state.set_new_register_value(reg, new_reg_value as u16);

                            assembled_file_str.push_str(
                                format!(
                                    "; {}: 0x{:02x} -> 0x{:02x}\n",
                                    reg, current_reg_value, new_reg_value
                                )
                                .as_str(),
                            );
                        }
                    }

                    0b1 => {
                        let data_1 = buf_iter.next().unwrap().1;
                        let data_2 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*data_1, *data_2]);
                        let reg = decode_register_field(reg_field, true);

                        assembled_file_str.push_str(&format!("mov {}, {}\n", reg, displacement));
                    }
                    _ => {
                        panic!("Unhandled W field at index {}", i);
                    }
                }
            }

            _ => {}
        }

        // Checking the first six bits
        match first_six_bits {
            0b100010 => {
                println!(
                    "Found a register/memory-to/from-register instruction at index {}",
                    i
                );
                let d_field = (byte >> 1) & 0b1_u8;
                let w_field = byte & 0b1;

                let reg_is_dest = match d_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled D field at index {}", i),
                };

                let is_wide = match w_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled W field at index {}", i),
                };

                let byte_2 = *buf_iter.next().unwrap().1;
                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let reg_field = (byte_2 >> 3) & 0b111_u8;
                let rm_field = byte_2 & 0b111;

                let reg = decode_register_field(reg_field, is_wide);

                match mod_field {
                    0b11 => {
                        println!("Register-to-register mode found at index {}", i);
                        let rm = decode_rm_field_at_mod_11(rm_field, is_wide);

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", reg, rm));
                        } else {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", rm, reg));
                        }
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);
                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", operand, reg));
                        }
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);
                        let displacement = *buf_iter.next().unwrap().1 as i8; // byte3 is the 8bit displacement
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("mov {}, {}\n", operand, reg));
                        }
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            if reg_is_dest {
                                assembled_file_str
                                    .push_str(&format!("mov {}, [{}]\n", reg, address));
                            } else {
                                assembled_file_str
                                    .push_str(&format!("mov {:04X}, {}\n", address, reg));
                            }
                        } else {
                            let rm = decode_rm_field_at_mod_00(rm_field);
                            let operand = format!("[{}]", rm);

                            if reg_is_dest {
                                assembled_file_str.push_str(&format!("mov {}, {}\n", reg, operand));
                            } else {
                                assembled_file_str.push_str(&format!("mov {}, {}\n", operand, reg));
                            }
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                }
            }

            0b000000 => {
                println!(
                    "Found an ADD Reg/memory with register to either instruction at index {}",
                    i
                );
                let d_field = (byte >> 1) & 0b1_u8;
                let w_field = byte & 0b1;

                let reg_is_dest = match d_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled D field at index {}", i),
                };

                let is_wide = match w_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled W field at index {}", i),
                };

                let byte_2 = *buf_iter.next().unwrap().1;

                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let reg_field = (byte_2 >> 3) & 0b111_u8;
                let rm_field = byte_2 & 0b111;

                let reg = decode_register_field(reg_field, is_wide);

                match mod_field {
                    0b11 => {
                        println!("Register mode found at index {}", i);
                        let rm = decode_rm_field_at_mod_11(rm_field, is_wide);

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("add {}, {}\n", reg, rm));
                        } else {
                            assembled_file_str.push_str(&format!("add {}, {}\n", rm, reg));
                        }
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);
                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("add {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("add {}, {}\n", operand, reg));
                        }
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);
                        let displacement = *buf_iter.next().unwrap().1 as i8; // byte3 is the 8bit displacement
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("add {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("add {}, {}\n", operand, reg));
                        }
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            if reg_is_dest {
                                assembled_file_str
                                    .push_str(&format!("add {}, [{}]\n", reg, address));
                            } else {
                                assembled_file_str
                                    .push_str(&format!("add {:04X}, {}\n", address, reg));
                            }
                        } else {
                            let rm = decode_rm_field_at_mod_00(rm_field);
                            let operand = format!("[{}]", rm);

                            if reg_is_dest {
                                assembled_file_str.push_str(&format!("add {}, {}\n", reg, operand));
                            } else {
                                assembled_file_str.push_str(&format!("add {}, {}\n", operand, reg));
                            }
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                }
            }

            0b100000 => {
                println!(
                    "Found an (ADD/SUB/CMP) immediate to register/ memory instruction at index {}",
                    i
                );
                let s_field = (byte >> 1) & 0b1_u8;
                let w_field = byte & 0b1;

                let is_wide = match w_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled W field at index {}", i),
                };

                let is_signed = match s_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled S field at index {}", i),
                };

                let byte_2 = *buf_iter.next().unwrap().1;

                let reg_field = (byte_2 >> 3) & 0b111_u8;
                let ix_code = match reg_field {
                    0b000 => "add",
                    0b101 => "sub",
                    0b111 => "cmp",
                    _ => "Unknown",
                };

                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let rm_field = byte_2 & 0b111;

                // NOTE: Here, MOD determines the addressing mode as usual. W determines the size of the
                // immediate operand and S determines whether the immediate operand is signed or
                // unsigned

                match mod_field {
                    0b11 => {
                        println!("Register mode found at index {}", i);
                        let rm = decode_rm_field_at_mod_11(rm_field, is_wide);

                        let immediate = match is_wide {
                            true => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                    format!("{}", displacement)
                                } else {
                                    let displacement = u16::from_le_bytes([*data_1, *data_2]);
                                    format!("{}", displacement)
                                }
                            }

                            false => {
                                let data = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i8::from_le_bytes([*data]);
                                    format!("{}", displacement)
                                } else {
                                    let displacement = u8::from_le_bytes([*data]);
                                    format!("{}", displacement)
                                }
                            }
                        };

                        assembled_file_str
                            .push_str(&format!("{} {}, {}\n", ix_code, rm, immediate));
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);

                        let displacement = *buf_iter.next().unwrap().1 as i8;
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        let immediate = match is_wide {
                            true => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                } else {
                                    let displacement = u16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                }
                            }

                            false => {
                                let data = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i8::from_le_bytes([*data]);
                                    format!("byte {}", displacement)
                                } else {
                                    let displacement = u8::from_le_bytes([*data]);
                                    format!("byte {}", displacement)
                                }
                            }
                        };

                        assembled_file_str
                            .push_str(&format!("{} {}, {}\n", ix_code, operand, immediate));
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);

                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        let immediate = match is_wide {
                            true => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                } else {
                                    let displacement = u16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                }
                            }

                            false => {
                                let data = buf_iter.next().unwrap().1;
                                if let true = is_signed {
                                    let displacement = i8::from_le_bytes([*data]);
                                    format!("byte {}", displacement)
                                } else {
                                    let displacement = u8::from_le_bytes([*data]);
                                    format!("byte {}", displacement)
                                }
                            }
                        };

                        assembled_file_str
                            .push_str(&format!("{} {}, {}\n", ix_code, operand, immediate));
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            let immediate = match is_wide {
                                true => {
                                    let data_1 = buf_iter.next().unwrap().1;
                                    let data_2 = buf_iter.next().unwrap().1;
                                    if let true = is_signed {
                                        let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                        format!("word {}", displacement)
                                    } else {
                                        let displacement = u16::from_le_bytes([*data_1, *data_2]);
                                        format!("word {}", displacement)
                                    }
                                }

                                false => {
                                    let data = buf_iter.next().unwrap().1;
                                    if let true = is_signed {
                                        let displacement = i8::from_le_bytes([*data]);
                                        format!("byte {}", displacement)
                                    } else {
                                        let displacement = u8::from_le_bytes([*data]);
                                        format!("byte {}", displacement)
                                    }
                                }
                            };

                            assembled_file_str
                                .push_str(&format!("{} {:04X}, {}\n", ix_code, address, immediate));
                        } else {
                            let rm = decode_rm_field_at_mod_00(rm_field);
                            let operand = format!("[{}]", rm);

                            let immediate = match is_wide {
                                true => {
                                    let data_1 = buf_iter.next().unwrap().1;
                                    let data_2 = buf_iter.next().unwrap().1;
                                    if let true = is_signed {
                                        let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                        format!("word {}", displacement)
                                    } else {
                                        let displacement = u16::from_le_bytes([*data_1, *data_2]);
                                        format!("word {}", displacement)
                                    }
                                }

                                false => {
                                    let data = buf_iter.next().unwrap().1;
                                    if let true = is_signed {
                                        let displacement = i8::from_le_bytes([*data]);
                                        format!("byte {}", displacement)
                                    } else {
                                        let displacement = u8::from_le_bytes([*data]);
                                        format!("byte {}", displacement)
                                    }
                                }
                            };

                            assembled_file_str
                                .push_str(&format!("{} {}, {} \n", ix_code, operand, immediate));
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                }
            }

            0b001010 => {
                println!(
                    "Found a SUB Reg/memory with register to either instruction at index {}",
                    i
                );

                let d_field = (byte >> 1) & 0b1_u8;
                let w_field = byte & 0b1;

                let reg_is_dest = match d_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled D field at index {}", i),
                };

                let is_wide = match w_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled W field at index {}", i),
                };

                let byte_2 = *buf_iter.next().unwrap().1;

                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let reg_field = (byte_2 >> 3) & 0b111_u8;
                let rm_field = byte_2 & 0b111;

                let reg = decode_register_field(reg_field, is_wide);

                match mod_field {
                    0b11 => {
                        println!("Register mode found at index {}", i);
                        let rm = decode_rm_field_at_mod_11(rm_field, is_wide);

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", reg, rm));
                        } else {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", rm, reg));
                        }
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);
                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", operand, reg));
                        }
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);
                        let displacement = *buf_iter.next().unwrap().1 as i8; // byte3 is the 8bit displacement
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("sub {}, {}\n", operand, reg));
                        }
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            if reg_is_dest {
                                assembled_file_str
                                    .push_str(&format!("sub {}, [{}]\n", reg, address));
                            } else {
                                assembled_file_str
                                    .push_str(&format!("sub {:04X}, {}\n", address, reg));
                            }
                        } else {
                            let rm = decode_rm_field_at_mod_00(rm_field);
                            let operand = format!("[{}]", rm);

                            if reg_is_dest {
                                assembled_file_str.push_str(&format!("sub {}, {}\n", reg, operand));
                            } else {
                                assembled_file_str.push_str(&format!("sub {}, {}\n", operand, reg));
                            }
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                }
            }

            0b001110 => {
                println!(
                    "Found a CMP Reg/memory and register instruction at index {}",
                    i
                );

                let d_field = (byte >> 1) & 0b1_u8;
                let w_field = byte & 0b1;

                let reg_is_dest = match d_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled D field at index {}", i),
                };

                let is_wide = match w_field {
                    0b0 => false,
                    0b1 => true,
                    _ => panic!("Unhandled W field at index {}", i),
                };

                let byte_2 = *buf_iter.next().unwrap().1;

                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let reg_field = (byte_2 >> 3) & 0b111_u8;
                let rm_field = byte_2 & 0b111;

                let reg = decode_register_field(reg_field, is_wide);

                match mod_field {
                    0b11 => {
                        println!("Register mode found at index {}", i);
                        let rm = decode_rm_field_at_mod_11(rm_field, is_wide);

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", reg, rm));
                        } else {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", rm, reg));
                        }
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);
                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", operand, reg));
                        }
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);
                        let displacement = *buf_iter.next().unwrap().1 as i8; // byte3 is the 8bit displacement
                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        if reg_is_dest {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", reg, operand));
                        } else {
                            assembled_file_str.push_str(&format!("cmp {}, {}\n", operand, reg));
                        }
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            if reg_is_dest {
                                assembled_file_str
                                    .push_str(&format!("cmp {}, [{}]\n", reg, address));
                            } else {
                                assembled_file_str
                                    .push_str(&format!("cmp {:04X}, {}\n", address, reg));
                            }
                        } else {
                            let rm = decode_rm_field_at_mod_00(rm_field);
                            let operand = format!("[{}]", rm);

                            if reg_is_dest {
                                assembled_file_str.push_str(&format!("cmp {}, {}\n", reg, operand));
                            } else {
                                assembled_file_str.push_str(&format!("cmp {}, {}\n", operand, reg));
                            }
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                }
            }

            _ => {}
        }

        // Checking the first seven bits
        match first_seven_bits {
            0b0011110 => {
                println!(
                    "Found a CMP immediate-to-accumulator instruction at index {}",
                    i
                );

                let w_field = byte & 0b1;

                let accumulator_reg = match w_field {
                    0b0 => "al",
                    0b1 => "ax",
                    _ => "Unknown",
                };

                // This instruction never has a wide immediate operand (at least according to the
                // documentation)
                let immediate = *buf_iter.next().unwrap().1;

                assembled_file_str.push_str(&format!("cmp {}, {}\n", accumulator_reg, immediate));
            }

            0b0000010 => {
                println!(
                    "Found an ADD immediate-to-accumulator instruction at index {}",
                    i
                );

                let w_field = byte & 0b1;

                let accumulator_reg = match w_field {
                    0b0 => "al",
                    0b1 => "ax",
                    _ => "Unknown",
                };

                let immediate = match w_field {
                    0b0 => {
                        let data = buf_iter.next().unwrap().1;
                        format!("{}", data)
                    }

                    0b1 => {
                        let data_1 = buf_iter.next().unwrap().1;
                        let data_2 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*data_1, *data_2]);
                        format!("{}", displacement)
                    }

                    _ => {
                        panic!("Unhandled W field at index {}", i);
                    }
                };

                assembled_file_str.push_str(&format!("mov {}, {}\n", accumulator_reg, immediate));
            }

            0b0010110 => {
                println!(
                    "Found a SUB immediate-to-accumulator instruction at index {}",
                    i
                );

                let w_field = byte & 0b1;

                let accumulator_reg = match w_field {
                    0b0 => "al",
                    0b1 => "ax",
                    _ => "Unknown",
                };

                let immediate = match w_field {
                    0b0 => {
                        let data = buf_iter.next().unwrap().1;
                        format!("{}", data)
                    }

                    0b1 => {
                        let data_1 = buf_iter.next().unwrap().1;
                        let data_2 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*data_1, *data_2]);
                        format!("{}", displacement)
                    }

                    _ => {
                        panic!("Unhandled W field at index {}", i);
                    }
                };

                assembled_file_str.push_str(&format!("sub {}, {}\n", accumulator_reg, immediate));
            }

            0b1100011 => {
                println!(
                    "Found an immediate-to-register/memory instruction at index {}",
                    i
                );
                let w_field = byte & 0b1;
                let byte_2 = *buf_iter.next().unwrap().1;

                let reg_field = (byte_2 >> 3) & 0b111_u8;
                assert_eq!(reg_field, 0b000_u8); // the REG field should always be 0b000 for this instruction

                let mod_field = (byte_2 >> 6) & 0b11_u8;
                let rm_field = byte_2 & 0b111;

                match mod_field {
                    0b11 => {
                        println!("Register mode found at index {}", i);

                        let rm = decode_rm_field_at_mod_11(rm_field, w_field == 0b1);
                        let immediate = match w_field {
                            0b0 => {
                                let data = buf_iter.next().unwrap().1;
                                format!("{}", data)
                            }

                            0b1 => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                format!("{}", displacement)
                            }

                            _ => {
                                panic!("Unhandled W field at index {}", i);
                            }
                        };

                        assembled_file_str.push_str(&format!("mov {}, {}\n", rm, immediate));
                    }

                    0b10 => {
                        println!("Memory mode (16bit displacement) found at index {}", i);

                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);

                        let byte_3 = buf_iter.next().unwrap().1;
                        let byte_4 = buf_iter.next().unwrap().1;
                        let displacement = i16::from_le_bytes([*byte_3, *byte_4]);

                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        let immediate = match w_field {
                            0b0 => {
                                let data = buf_iter.next().unwrap().1;
                                format!("byte {}", data)
                            }

                            0b1 => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                format!("word {}", displacement)
                            }

                            _ => {
                                panic!("Unhandled W field at index {}", i);
                            }
                        };

                        assembled_file_str.push_str(&format!("mov {}, {}\n", operand, immediate));
                    }

                    0b01 => {
                        println!("Memory mode (8bit displacement) found at index {}", i);

                        let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                        let displacement = *buf_iter.next().unwrap().1 as i8;
                        let operand = match displacement.is_negative() {
                            true => format!("[{}{}]", rm, displacement),
                            false => format!("[{}+{}]", rm, displacement),
                        };

                        let immediate = match w_field {
                            0b0 => {
                                let data = buf_iter.next().unwrap().1;
                                format!("byte {}", data)
                            }

                            0b1 => {
                                let data_1 = buf_iter.next().unwrap().1;
                                let data_2 = buf_iter.next().unwrap().1;
                                let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                format!("word {}", displacement)
                            }

                            _ => {
                                panic!("Unhandled W field at index {}", i);
                            }
                        };

                        assembled_file_str.push_str(&format!("mov {}, {}\n", operand, immediate));
                    }

                    0b00 => {
                        println!("Memory mode (no displacement)* found at index {}", i);

                        let rm = decode_rm_field_at_mod_00(rm_field);

                        if rm_field == 0b110 {
                            let byte_3 = buf_iter.next().unwrap().1;
                            let byte_4 = buf_iter.next().unwrap().1;
                            let address = i16::from_le_bytes([*byte_3, *byte_4]);

                            let immediate = match w_field {
                                0b0 => {
                                    let data = buf_iter.next().unwrap().1;
                                    format!("byte {}", data)
                                }

                                0b1 => {
                                    let data_1 = buf_iter.next().unwrap().1;
                                    let data_2 = buf_iter.next().unwrap().1;
                                    let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                }

                                _ => {
                                    panic!("Unhandled W field at index {}", i);
                                }
                            };

                            assembled_file_str
                                .push_str(&format!("mov {:04X}, {}\n", address, immediate));
                        } else {
                            let operand = format!("[{}]", rm);

                            let immediate = match w_field {
                                0b0 => {
                                    let data = buf_iter.next().unwrap().1;
                                    format!("byte {}", data)
                                }

                                0b1 => {
                                    let data_1 = buf_iter.next().unwrap().1;
                                    let data_2 = buf_iter.next().unwrap().1;
                                    let displacement = i16::from_le_bytes([*data_1, *data_2]);
                                    format!("word {}", displacement)
                                }

                                _ => {
                                    panic!("Unhandled W field at index {}", i);
                                }
                            };

                            assembled_file_str
                                .push_str(&format!("mov {}, {}\n", operand, immediate));
                        }
                    }

                    _ => {
                        panic!("Unhandled mod field at index {}", i);
                    }
                };
            }

            0b1010000 => {
                println!("Found a memory-to-accumulator instruction at index {}", i);

                let w_field = byte & 0b1;
                let accumulator_reg = match w_field {
                    0b0 => "al",
                    0b1 => "ax",
                    _ => "Unknown",
                };

                let byte_2 = buf_iter.next().unwrap().1;
                let byte_3 = buf_iter.next().unwrap().1;

                let memory_location = i16::from_le_bytes([*byte_2, *byte_3]);

                assembled_file_str
                    .push_str(&format!("mov {}, [{}]\n", accumulator_reg, memory_location));
            }

            0b1010001 => {
                println!("Found an accumulator-to-memory instruction at index {}", i);

                let w_field = byte & 0b1;
                let accumulator_reg = match w_field {
                    0b0 => "al",
                    0b1 => "ax",
                    _ => "Unknown",
                };

                let byte_2 = buf_iter.next().unwrap().1;
                let byte_3 = buf_iter.next().unwrap().1;

                let memory_location = i16::from_le_bytes([*byte_2, *byte_3]);

                assembled_file_str
                    .push_str(&format!("mov [{}], {}\n", memory_location, accumulator_reg));
            }

            _ => {}
        }

        // Checking the full byte (Conditional jump instructions)
        match first_full_byte {
            0b01110100 => {
                println!("Found a JE/JZ instruction at index {}", i);

                let next_byte = buf_iter.next().unwrap().1;

                let displacement = i8::from_le_bytes([*next_byte]);
                assembled_file_str.push_str(&format!("je {}\n", displacement));
            }

            0b01111100 => {
                println!("Found a JL/JNGE instruction at index {}", i);

                let next_byte = buf_iter.next().unwrap().1;

                let displacement = i8::from_le_bytes([*next_byte]);
                assembled_file_str.push_str(&format!("jl {}\n", displacement));
            }

            0b01111110 => {
                println!("Found a JLE/JNG instruction at index {}", i);

                let next_byte = buf_iter.next().unwrap().1;

                let displacement = i8::from_le_bytes([*next_byte]);
                assembled_file_str.push_str(&format!("jle {}\n", displacement));
            }

            0b01110010 => {
                println!("Found a JB/JNAE instruction at index {}", i);

                let next_byte = buf_iter.next().unwrap().1;

                let displacement = i8::from_le_bytes([*next_byte]);
                assembled_file_str.push_str(&format!("jb {}\n", displacement));
            }

            0b01110110 => {
                println!("Found a JBE/JNA instruction at index {}", i);

                let next_byte = buf_iter.next().unwrap().1;

                let displacement = i8::from_le_bytes([*next_byte]);
                assembled_file_str.push_str(&format!("jbe {}\n", displacement));
            }

            _ => {
                println!("FFB - Unknown instruction found at index {}", i);
            }
        }
    }

    println!("{}", assembled_file_str);
    println!("File processed!");

    if let Some(path) = output_file {
        fs::write(&path, assembled_file_str).expect("Unable to write file");
        println!("File written to {}", path);
    }

    if should_sim {
        // Print the register state
        println!("Final state:");
        cpu_state.print_register_state()
    }
}

fn decode_rm_field_at_mod_11<'a>(rm_field: u8, w_field: bool) -> &'a str {
    match w_field {
        true => match rm_field {
            0b000 => "ax",
            0b001 => "cx",
            0b010 => "dx",
            0b011 => "bx",
            0b100 => "sp",
            0b101 => "bp",
            0b110 => "si",
            0b111 => "di",
            _ => "Unknown",
        },
        false => match rm_field {
            0b000 => "al",
            0b001 => "cl",
            0b010 => "dl",
            0b011 => "bl",
            0b100 => "ah",
            0b101 => "ch",
            0b110 => "dh",
            0b111 => "bh",
            _ => "Unknown",
        },
    }
}

fn decode_rm_field_at_mod_10_and_mod_01<'a>(rm_field: u8) -> &'a str {
    match rm_field {
        0b000 => "bx+si",
        0b001 => "bx+di",
        0b010 => "bp+si",
        0b011 => "bp+di",
        0b100 => "si",
        0b101 => "di",
        0b110 => "bp",
        0b111 => "bx",
        _ => "Unknown",
    }
}

fn decode_rm_field_at_mod_00(rm_field: u8) -> &'static str {
    match rm_field {
        0b000 => "bx+si",
        0b001 => "bx+di",
        0b010 => "bp+si",
        0b011 => "bp+di",
        0b100 => "si",
        0b101 => "di",
        // 0b110 => "DIRECT ADDRESSING", *We handle this explicitly in the calling code for this function*
        0b111 => "bx",
        _ => "Unknown",
    }
}

fn decode_register_field<'a>(reg_field: u8, w_field: bool) -> &'a str {
    match w_field {
        true => match reg_field {
            0b000 => "ax",
            0b001 => "cx",
            0b010 => "dx",
            0b011 => "bx",
            0b100 => "sp",
            0b101 => "bp",
            0b110 => "si",
            0b111 => "di",
            _ => "Unknown",
        },
        false => match reg_field {
            0b000 => "al",
            0b001 => "cl",
            0b010 => "dl",
            0b011 => "bl",
            0b100 => "ah",
            0b101 => "ch",
            0b110 => "dh",
            0b111 => "bh",
            _ => "Unknown",
        },
    }
}
