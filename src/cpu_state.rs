#[derive(Debug)]
pub struct Register {
    value: u16, // 8086 uses 16-bit registers
}

impl Register {
    pub fn new() -> Self {
        Register { value: 0 } // we initialize all registers to 0
    }

    // Get full 16-bit value
    pub fn get(&self) -> u16 {
        self.value
    }

    // Set full 16-bit value
    pub fn set(&mut self, value: u16) {
        self.value = value;
    }

    // Get low 8-bit byte
    pub fn get_low(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    // Get high 8-bit byte
    pub fn get_high(&self) -> u8 {
        ((self.value >> 8) & 0xFF) as u8
    }

    // Set low 8-bit byte
    pub fn set_low(&mut self, value: u8) {
        self.value = (self.value & 0xFF00) | (value as u16);
    }

    // Set high 8-bit byte
    pub fn set_high(&mut self, value: u8) {
        self.value = (self.value & 0x00FF) | ((value as u16) << 8);
    }
}

#[derive(Debug)]
pub struct CpuState {
    // General purpose registers
    pub ax: Register,
    pub bx: Register,
    pub cx: Register,
    pub dx: Register,

    // Index registers
    pub si: Register,
    pub di: Register,
    pub bp: Register,
    pub sp: Register,

    // Instruction pointer
    pub ip: Register,
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            ax: Register::new(),
            bx: Register::new(),
            cx: Register::new(),
            dx: Register::new(),

            si: Register::new(),
            di: Register::new(),
            bp: Register::new(),
            sp: Register::new(),

            ip: Register::new(),
        }
    }

    /// Get value of the register
    pub fn get_register_value(&self, register: &str) -> u16 {
        match register {
            "al" => self.ax.get_low() as u16,
            "ah" => self.ax.get_high() as u16,
            "ax" => self.ax.get(),

            "bl" => self.bx.get_low() as u16,
            "bh" => self.bx.get_high() as u16,
            "bx" => self.bx.get(),

            "cl" => self.cx.get_low() as u16,
            "ch" => self.cx.get_high() as u16,
            "cx" => self.cx.get(),

            "dl" => self.dx.get_low() as u16,
            "dh" => self.dx.get_high() as u16,
            "dx" => self.dx.get(),

            "si" => self.si.get(),
            "di" => self.di.get(),
            "bp" => self.bp.get(),
            "sp" => self.sp.get(),

            "ip" => self.ip.get(),

            _ => panic!("Unknown register: {}", register),
        }
    }

    /// Set new value for the register
    pub fn set_new_register_value(&mut self, register: &str, value: u16) {
        match register {
            "al" => self.ax.set_low(value as u8),
            "ah" => self.ax.set_high(value as u8),
            "ax" => self.ax.set(value),

            "bl" => self.bx.set_low(value as u8),
            "bh" => self.bx.set_high(value as u8),
            "bx" => self.bx.set(value),

            "cl" => self.cx.set_low(value as u8),
            "ch" => self.cx.set_high(value as u8),
            "cx" => self.cx.set(value),

            "dl" => self.dx.set_low(value as u8),
            "dh" => self.dx.set_high(value as u8),
            "dx" => self.dx.set(value),

            "si" => self.si.set(value),
            "di" => self.di.set(value),
            "bp" => self.bp.set(value),
            "sp" => self.sp.set(value),

            "ip" => self.ip.set(value),

            _ => panic!("Unknown register: {}", register),
        }
    }

    pub fn print_register_state(&self) {
        println!(format!("ax: {:#X}", self.ax.get()));
        println!(format!("bx: {:#X}", self.bx.get()));
        println!(format!("cx: {:#X}", self.cx.get()));
        println!(format!("dx: {:#X}", self.dx.get()));
        println!(format!("si: {:#X}", self.si.get()));
        println!(format!("di: {:#X}", self.di.get()));
        println!(format!("bp: {:#X}", self.bp.get()));
        println!(format!("sp: {:#X}", self.sp.get()));
        println!(format!("ip: {:#X}", self.ip.get()));
    }
}
