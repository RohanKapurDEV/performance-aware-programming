use clap::Parser;
use std::{
    fs,
    iter::{Enumerate, Peekable},
    slice::Iter,
};

mod cli;

use cli::Args;

fn main() {
    let args = Args::parse();
    let file_path = args.asm_bin_path;
    let outout_file = args.output_file;
    println!("Selected file: {}", file_path);

    let file_buffer = fs::read(file_path).expect("Unable to open file");
    let mut buf_iter: Peekable<Enumerate<Iter<u8>>> = file_buffer.iter().enumerate().peekable();

    // Final assembled string of the file - mutated over the course of the loop
    let mut assembled_file_str = "bits 16\n\n".to_string();

    // Loop through the buffer
    while let Some((i, byte)) = buf_iter.next() {
        let first_four_bits = (byte >> 4) & 0b1111; // [immediate-to-register]
        let first_six_bits = (byte >> 2) & 0b111111; // [register/memory-to/from-register]
        let first_seven_bits = (byte >> 1) & 0b1111111; // [immediate-to-register/memory, memory-to-accumulator, accumulator-to-memory]

        // Checking the first four bits
        if let 0b1011 = first_four_bits {
            println!("Found an immediate-to-register instruction at index {}", i);
            let w_field = (byte >> 3) & 0b1;
            let reg_field = byte & 0b111;

            match w_field {
                0b0 => {
                    let data = buf_iter.next().unwrap().1;
                    let reg = decode_register_field(reg_field, false);

                    assembled_file_str.push_str(&format!("mov {}, {}\n", reg, data));
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

            continue;
        }

        // Checking the first six bits
        if let 0b100010 = first_six_bits {
            println!(
                "Found a register/memory-to/from-register instruction at index {}",
                i
            );
            let d_field = (byte >> 1) & 0b1;
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

            let byte_2 = buf_iter.next().unwrap().1;
            let mod_field = (byte_2 >> 6) & 0b11;
            let reg_field = (byte_2 >> 3) & 0b111;
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
                    let displacement = buf_iter.next().unwrap().1; // byte3 is the 8bit displacement
                    let rm = decode_rm_field_at_mod_10_and_mod_01(rm_field);
                    let operand = format!("[{}+{}]", rm, displacement);

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
                            assembled_file_str.push_str(&format!("mov {}, [{}]\n", reg, address));
                        } else {
                            assembled_file_str.push_str(&format!("mov {:04X}, {}\n", address, reg));
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

            continue;
        }

        // Checking the first seven bits
        if let 0b1100011 = first_seven_bits {
            println!(
                "Found an immediate-to-register/memory instruction at index {}",
                i
            );
            let w_field = byte & 0b1;
            let byte_2 = buf_iter.next().unwrap().1;

            let reg_field = (byte_2 >> 3) & 0b111;
            assert!(reg_field == 0b000); // the REG field should always be 0b000 for this instruction

            let mod_field = (byte_2 >> 6) & 0b11;
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

                    let operand = format!("[{}+{}]", rm, displacement);

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
                    let displacement = buf_iter.next().unwrap().1;
                    let operand = format!("[{}+{}]", rm, displacement);

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

                        assembled_file_str.push_str(&format!("mov {}, {}\n", operand, immediate));
                    }
                }

                _ => {
                    panic!("Unhandled mod field at index {}", i);
                }
            };
        }

        if let 0b1010000 = first_seven_bits {
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

        if let 0b1010001 = first_seven_bits {
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
    }

    println!("{}", assembled_file_str);
    println!("File processed!");

    if let Some(path) = outout_file {
        fs::write(&path, assembled_file_str).expect("Unable to write file");
        println!("File written to {}", path);
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
