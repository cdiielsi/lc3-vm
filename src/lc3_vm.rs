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

    /// Branch instruction adds a 9 bit offset to the PC if the indicated flag is on.
    fn branch(&mut self, flag: Flags, pc_offset: u16) {
        if self.flag_is_on(flag) {
            let offset = self.extend_sign(pc_offset, 9);
            self.registers[Registers::PC as usize] =
                self.registers[Registers::PC as usize].wrapping_add(offset);
        }
    }

    /// Checks if a determined flag is on.
    fn flag_is_on(&self, flag: Flags) -> bool {
        match flag {
            Flags::Pos => self.registers[Registers::COND as usize] & 0b1 == 1,
            Flags::Zro => self.registers[Registers::COND as usize] & 0b10 == 2,
            Flags::Neg => self.registers[Registers::COND as usize] & 0b100 == 4,
        }
    }

    /// Extends sign for 9 bit numbers
    fn extend_sign(&mut self, number: u16, imm_size: usize) -> u16 {
        let mut check_mask = 0;
        let mut extend_mask = 0;
        if imm_size == 5 {
            check_mask = 0x0010; // check_mask = 0000 0000 0001 0000; 
            extend_mask = 0xFFF0;
        } else if imm_size == 9 {
            check_mask = 0x0100; // check_mask = 0000 0001 0000 0000;
            extend_mask = 0xFFF0;
        }

        if number & check_mask == check_mask {
            return number | extend_mask;
        }
        number
    }

    /// Add istruction has two modes:
    /// Mode 0 => adds the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => adds the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    fn add(&mut self, dst: usize, src1: usize, mode: u16, second_operand: u16) {
        if mode == 0 {
            self.registers[dst as usize] =
                self.registers[src1 as usize].wrapping_add(self.registers[second_operand as usize])
        } else if mode == 1 {
            self.registers[dst as usize] =
                self.registers[src1 as usize].wrapping_add(self.extend_sign(second_operand, 5))
        }
    }
}

enum Registers {
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
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 0);
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 0);
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 0);
    }

    #[test]
    /// At the moment all registers are initialized with zero, so when changing the flags values PC
    /// should add up the offset.
    fn branch_instruction_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Registers::COND as usize] = 1;
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 16);
        vm.registers[Registers::COND as usize] = 2;
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 32);
        vm.registers[Registers::COND as usize] = 4;
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 48);
        vm.branch(Flags::Neg, 0xFFFF);
        assert_eq!(vm.registers[Registers::PC as usize], 47);
    }

    #[test]
    fn adding_two_modes() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // Immediate mode add positive number
        vm.add(Registers::R5 as usize, Registers::R0 as usize, 1, 5);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        vm.registers[Registers::R0 as usize] = 32;
        // Register mode
        vm.add(
            Registers::R4 as usize,
            Registers::R5 as usize,
            0,
            Registers::R0 as u16,
        );
        assert_eq!(vm.registers[Registers::R4 as usize], 37);
        assert_eq!(vm.registers[Registers::R0 as usize], 32);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        // Immediate mode add negative number.
        // 20 is 1 0100 in binary, which is equal to -12 in two's complement notation.
        vm.add(Registers::R7 as usize, Registers::R4 as usize, 1, 20);
        assert_eq!(vm.registers[Registers::R7 as usize], 25);
        assert_eq!(vm.registers[Registers::R4 as usize], 37);
        assert_eq!(vm.registers[Registers::R0 as usize], 32);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
    }
}
