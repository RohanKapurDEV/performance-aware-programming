#[derive(Debug)]
pub struct Register {
    value: u16, // 8086 uses 16-bit registers
}

impl Register {
    pub fn new() -> Self {
        Register { value: 0 } // we initialize all registers to 0
    }
}

#[derive(Debug)]
pub struct CpuState {
    pub ax: Register,
    pub bx: Register,
    pub cx: Register,
    pub dx: Register,

    pub si: Register,
    pub di: Register,
    pub bp: Register,
    pub sp: Register,

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
}
