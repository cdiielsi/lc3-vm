pub struct LC3VirtualMachine {
    memory: [u16; 1 << 16], /* 65536 locations */
    registers: [u16; 10],
}

impl LC3VirtualMachine {
    pub fn new() -> Self {
        Self {
            memory: [0; 1 << 16],
            registers: [0; 10],
        }
    }

    fn branch(&mut self, flag: Flags, pc_offset: u16) {
        if self.flag_is_on(flag) {
            self.registers[Registers::PC] += pc_offset;
        }
    }

    fn flag_is_on(&self, flag: Flags) -> bool {
        match flag {
            Flags::FL_POS => {
                println!("holis {}", self.registers[Registers::COND]);
                self.registers[Registers::COND] & 0b1 == 1
            }
            Flags::FL_ZRO => self.registers[Registers::COND] & 0b10 == 2,
            Flags::FL_NEG => self.registers[Registers::COND] & 0b100 == 4,
        }
    }
}

pub enum Registers {
    R0 = 0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    PC, /* program counter */
    COND,
}

enum Flags {
    Pos,
    Zro,
    Neg,
}

enum Instructions {
    OpBR,   /* branch */
    OpADD,  /* add  */
    OpLD,   /* load */
    OpST,   /* store */
    OpJSR,  /* jump register */
    OpAND,  /* bitwise and */
    OpLDR,  /* load register */
    OpSTR,  /* store register */
    OpRTI,  /* unused */
    OpNOT,  /* bitwise not */
    OpLDI,  /* load indirect */
    OpSTI,  /* store indirect */
    OpJMP,  /* jump */
    OpRES,  /* reserved (unused) */
    OpLEA,  /* load effective address */
    OpTRAP, /* execute trap */
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_and_index_mut_with_registers() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        vm.registers[Registers::R0 as usize] = 16;
        assert_eq!(vm.registers[Registers::R0 as usize], 16);
    }

    #[test]
    /// At the moment all registers are initialized with zero, so when executing a conditional branch PC
    /// should stay equal to zero
    fn branch_instruction_no_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.branch(Flags::FL_NEG, 16);
        assert_eq!(vm.registers[Registers::PC], 0);
        vm.branch(Flags::FL_POS, 16);
        assert_eq!(vm.registers[Registers::PC], 0);
        vm.branch(Flags::FL_ZRO, 16);
        assert_eq!(vm.registers[Registers::PC], 0);
    }

    #[test]
    /// At the moment all registers are initialized with zero, so when changing the flags values PC
    /// should add up the offset.
    fn branch_instruction_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Registers::COND] = 1;
        vm.branch(Flags::FL_POS, 16);
        assert_eq!(vm.registers[Registers::PC], 16);
        vm.registers[Registers::COND] = 2;
        vm.branch(Flags::FL_ZRO, 16);
        assert_eq!(vm.registers[Registers::PC], 32);
        vm.registers[Registers::COND] = 4;
        vm.branch(Flags::FL_NEG, 16);
        assert_eq!(vm.registers[Registers::PC], 48);
    }
}
