use std::ops::{Index, IndexMut};

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

enum Registers {
    R0,
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

impl<T> Index<Registers> for [T] {
    type Output = T;

    fn index(&self, register: Registers) -> &Self::Output {
        match register {
            Registers::R0 => &self[0],
            Registers::R1 => &self[1],
            Registers::R2 => &self[2],
            Registers::R3 => &self[3],
            Registers::R4 => &self[4],
            Registers::R5 => &self[5],
            Registers::R6 => &self[6],
            Registers::R7 => &self[7],
            Registers::PC => &self[8],
            Registers::COND => &self[9],
        }
    }
}

impl<T> IndexMut<Registers> for [T] {
    fn index_mut(&mut self, register: Registers) -> &mut Self::Output {
        match register {
            Registers::R0 => &mut self[0],
            Registers::R1 => &mut self[1],
            Registers::R2 => &mut self[2],
            Registers::R3 => &mut self[3],
            Registers::R4 => &mut self[4],
            Registers::R5 => &mut self[5],
            Registers::R6 => &mut self[6],
            Registers::R7 => &mut self[7],
            Registers::PC => &mut self[8],
            Registers::COND => &mut self[9],
        }
    }
}

enum Flags {
    FL_POS, /* 1 << 0 == 1 */
    FL_ZRO, /* 1 << 1 == 2 */
    FL_NEG, /* 1 << 2 == 3 */
}

enum Instructions {
    OP_BR,   /* branch */
    OP_ADD,  /* add  */
    OP_LD,   /* load */
    OP_ST,   /* store */
    OP_JSR,  /* jump register */
    OP_AND,  /* bitwise and */
    OP_LDR,  /* load register */
    OP_STR,  /* store register */
    OP_RTI,  /* unused */
    OP_NOT,  /* bitwise not */
    OP_LDI,  /* load indirect */
    OP_STI,  /* store indirect */
    OP_JMP,  /* jump */
    OP_RES,  /* reserved (unused) */
    OP_LEA,  /* load effective address */
    OP_TRAP, /* execute trap */
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_and_index_mut_with_registers() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        assert_eq!(vm.registers[Registers::R0], 0);
        vm.registers[Registers::R0] = 16;
        assert_eq!(vm.registers[Registers::R0], 16);
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
