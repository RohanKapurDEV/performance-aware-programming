use std::fs;

fn main() {
    let file_path = "./listing_38";
    let file_buffer = fs::read(file_path).expect("Unable to open file");

    let mut assembled_file_str = "bits 16\n\n".to_string();

    let mut buf_iter = file_buffer.iter().enumerate().peekable();

    // Loop through the buffer
    while let Some((i, byte)) = buf_iter.next() {
        let opcode = (byte >> 2) & 0b111111;
        let remainder = byte & 0b11;

        match opcode {
            // MOV instruction
            0b00_100010 => {
                println!("MOV instruction found at index {}", i);
                // Currently, we only want to handle situations where the W flag is 1 (word operations)

                match remainder {
                    0b01 => {
                        println!("D flag is 0 and W flag is 1");
                        let (_, byte) = buf_iter.next().unwrap();

                        let (reg, rm) = parse_second_mov_byte(byte, true);
                        assembled_file_str.push_str(&format!("mov {}, {}\n", rm, reg));
                    }
                    0b11 => {
                        println!("D flag is 1 and W flag is 1");
                        let (_, byte) = buf_iter.next().unwrap();

                        let (reg, rm) = parse_second_mov_byte(byte, true);
                        assembled_file_str.push_str(&format!("mov {}, {}\n", reg, rm));
                    }
                    0b10 => {
                        println!("D flag is 1 and W flag is 0");
                        let (_, byte) = buf_iter.next().unwrap();

                        let (reg, rm) = parse_second_mov_byte(byte, false);
                        assembled_file_str.push_str(&format!("mov {}, {}\n", reg, rm));
                    }
                    0b00 => {
                        println!("D flag is 0 and W flag is 0");
                        let (_, byte) = buf_iter.next().unwrap();

                        let (reg, rm) = parse_second_mov_byte(byte, false);
                        assembled_file_str.push_str(&format!("mov {}, {}\n", rm, reg));
                    }

                    _ => {
                        println!("Unhandled bit pattern for remainder");
                    }
                }
            }

            _ => {
                println!("Unknown opcode at index {}", i);
            }
        }
    }

    println!("{}", assembled_file_str);
    println!("File processed!")
}

/// Returns (REG, R/M) fields
fn parse_second_mov_byte(byte: &u8, w_field: bool) -> (&str, &str) {
    let mod_field = (byte >> 6) & 0b11;
    let reg_field = (byte >> 3) & 0b111;
    let rm_field = byte & 0b111;

    if let 0b11 = mod_field {
        println!("Register addressing mode");

        match w_field {
            true => {
                let reg = match reg_field {
                    0b000 => "ax",
                    0b001 => "cx",
                    0b010 => "dx",
                    0b011 => "bx",
                    0b100 => "sp",
                    0b101 => "bp",
                    0b110 => "si",
                    0b111 => "di",
                    _ => "Unknown",
                };

                let rm = match rm_field {
                    0b000 => "ax",
                    0b001 => "cx",
                    0b010 => "dx",
                    0b011 => "bx",
                    0b100 => "sp",
                    0b101 => "bp",
                    0b110 => "si",
                    0b111 => "di",
                    _ => "Unknown",
                };

                return (reg, rm);
            }

            false => {
                let reg = match reg_field {
                    0b000 => "al",
                    0b001 => "cl",
                    0b010 => "dl",
                    0b011 => "bl",
                    0b100 => "ah",
                    0b101 => "ch",
                    0b110 => "dh",
                    0b111 => "bh",
                    _ => "Unknown",
                };

                let rm = match rm_field {
                    0b000 => "al",
                    0b001 => "cl",
                    0b010 => "dl",
                    0b011 => "bl",
                    0b100 => "ah",
                    0b101 => "ch",
                    0b110 => "dh",
                    0b111 => "bh",
                    _ => "Unknown",
                };

                return (reg, rm);
            }
        }
    } else {
        println!("Unhandled addressing mode - for now")
    }

    return ("Unknown", "Unknown");
}
