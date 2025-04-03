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
}

pub enum Registers {
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
    FL_POS,
    FL_ZRO,
    FL_NEG,
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
}
