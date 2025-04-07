use std::result;

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

    /// Checks if a determined flag is on.
    fn flag_is_on(&self, flag: Flags) -> bool {
        match flag {
            Flags::Pos => self.registers[Registers::COND as usize] & 0b1 == 1,
            Flags::Zro => self.registers[Registers::COND as usize] & 0b10 == 2,
            Flags::Neg => self.registers[Registers::COND as usize] & 0b100 == 4,
        }
    }

    fn update_flags(&mut self, result_from_operation: u16) {
        if result_from_operation == 0 {
            self.registers[Registers::COND as usize] = 2; // Flag Zro 0b10
        } else if result_from_operation & 0x8000 == 0 {
            self.registers[Registers::COND as usize] = 1; // Flag Pos 0b1
        } else {
            self.registers[Registers::COND as usize] = 4; // Flag Neg 0b100
        }
    }

    /// Extends sign for 9 bit numbers
    fn extend_sign(&mut self, number: u16, imm_size: usize) -> u16 {
        let extend_mask = 0xFFFF << imm_size;
        if number >> (imm_size - 1) & 1 == 1 {
            return number | extend_mask;
        }
        number
    }

    /// Branch instruction adds a 9 bit offset to the PC if the indicated flag is on.
    fn branch(&mut self, flag: Flags, pc_offset: u16) {
        if self.flag_is_on(flag) {
            let offset = self.extend_sign(pc_offset, 9);
            self.registers[Registers::PC as usize] =
                self.registers[Registers::PC as usize].wrapping_add(offset);
        }
    }

    /// Add istruction has two modes:
    /// Mode 0 => adds the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => adds the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// Add alters the flags depending on the result of the operation.
    fn add(&mut self, dst: Registers, src1: Registers, mode: u16, second_operand: u16) {
        let mut result = 0;
        if mode == 0 {
            result =
                self.registers[src1 as usize].wrapping_add(self.registers[second_operand as usize]);
        } else if mode == 1 {
            result =
                self.registers[src1 as usize].wrapping_add(self.extend_sign(second_operand, 5));
        }
        self.registers[dst as usize] = result;
        self.update_flags(result);
    }

    /// Load instruction loads into dst register the content in the memory address pc + pc_offset (9 bit immediate).
    /// Load alters flags depending the content loaded into the register.
    fn load(&mut self, dst: Registers, pc_offset: u16) {
        let mem_adress = self.registers[Registers::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9)) as usize;
        self.registers[dst as usize] = self.memory[mem_adress];
        self.update_flags(self.memory[mem_adress]);
    }

    /// Store instruction loads into the memory address pc + pc_offset (9 bit immediate) the content in src register.
    /// Store doesn't alter flags.
    fn store(&mut self, src: Registers, pc_offset: u16) {
        let mem_adress = self.registers[Registers::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9)) as usize;
        self.memory[mem_adress] = self.registers[src as usize];
    }

    /// Jump Register stores the PC in R7 and then diverges in two modes:
    /// if long_flag == 1 the PC is updated to PC + operand (an 11 bit immediate value).
    /// if long_flag == 0 the PC takes the value stored in the register indicated by operand.
    fn jump_register(&mut self, long_flag: u16, operand: u16) {
        self.registers[Registers::R7 as usize] = self.registers[Registers::PC as usize];
        if long_flag == 1 {
            // JSR
            self.registers[Registers::PC as usize] =
                self.registers[Registers::PC as usize].wrapping_add(self.extend_sign(operand, 11));
        } else {
            // JSRR
            self.registers[Registers::PC as usize] = self.registers[operand as usize];
        }
    }

    /// And istruction has two modes:
    /// Mode 0 => bitwise and between the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => bitwise and between the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// And alters the flags depending on the result of the operation.
    fn and(&mut self, dst: Registers, src1: Registers, mode: u16, second_operand: u16) {
        let mut result = 0;
        if mode == 0 {
            result = self.registers[src1 as usize] & self.registers[second_operand as usize];
        } else if mode == 1 {
            result = self.registers[src1 as usize] & self.extend_sign(second_operand, 5);
        }
        self.registers[dst as usize] = result;
        self.update_flags(result);
    }

    /// Load register instruction loads into dst register the content in the memory addres obtained by adding the
    /// content of src register and offset (6 bit immediate).
    /// Load register alters flags depending the content loaded into the dst register.
    fn load_register(&mut self, dst: Registers, src: Registers, offset: u16) {
        let data_in_memory = self.memory
            [self.registers[src as usize].wrapping_add(self.extend_sign(offset, 6)) as usize];
        self.registers[dst as usize] = data_in_memory;
        self.update_flags(data_in_memory);
    }

    /// Store register instruction stores in memory the content in the src register.
    /// Te memory address is calculated by adding the offset to the content in the dst register.
    fn store_register(&mut self, src: Registers, dst: Registers, offset: u16) {
        let memory_address = self.registers[dst as usize].wrapping_add(self.extend_sign(offset, 6));
        self.memory[memory_address as usize] = self.registers[src as usize];
    }

    fn not(&mut self, dst: Registers, src: Registers) {
        let result = !self.registers[src as usize];
        self.registers[dst as usize] = result;
        self.update_flags(result);
    }

    /// Load Indirect instruction loads into dst register the content in the memory address found in memory at pc + pc_offset (9 bit immediate).
    /// Load Indirect alters flags depending the content loaded into the register.
    fn load_indirect(&mut self, dst: Registers, pc_offset: u16) {
        let mem_adress = self.memory[self.registers[Registers::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9))
            as usize];
        self.registers[dst as usize] = self.memory[mem_adress as usize];
        self.update_flags(self.memory[mem_adress as usize]);
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
    /// When initializing the vm all registers are initialized with zero, so when executing a conditional branch PC
    /// should stay equal to zero.
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
    fn branch_instruction_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Registers::COND as usize] = 1; // Set only Pos flag in 1.
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 16);
        vm.registers[Registers::COND as usize] = 2; // Set only Zro flag in 1.
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 32);
        vm.registers[Registers::COND as usize] = 4; // Set only Neg flag in 1.
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Registers::PC as usize], 48);
        vm.branch(Flags::Neg, 0xFFFF);
        assert_eq!(vm.registers[Registers::PC as usize], 47);
    }

    #[test]
    fn add_instruction_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        vm.registers[Registers::R0 as usize] = 32;
        vm.registers[Registers::R5 as usize] = 5;
        vm.add(Registers::R4, Registers::R5, 0, Registers::R0 as u16);
        assert_eq!(vm.registers[Registers::R4 as usize], 37);
        assert_eq!(vm.registers[Registers::R0 as usize], 32);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        vm.registers[Registers::R1 as usize] = 1;
        vm.registers[Registers::R2 as usize] = 65530;
        vm.add(Registers::R2, Registers::R2, 0, Registers::R1 as u16);
        assert_eq!(vm.registers[Registers::R2 as usize], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        vm.add(Registers::R2, Registers::R2, 0, Registers::R5 as u16);
        assert_eq!(vm.registers[Registers::R2 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag.
    }

    #[test]
    fn add_instruction_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // Immediate mode add positive number.
        vm.add(Registers::R5, Registers::R0, 1, 5);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 
        // Register mode.
        vm.registers[Registers::R4 as usize] = 65535; // 65535 in u16 is 0xFFFF which is equal to -1 in two'2 complement notation.
        // Immediate mode add negative number.
        vm.add(Registers::R7, Registers::R4, 1, 1);
        assert_eq!(vm.registers[Registers::R7 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag.

        vm.registers[Registers::R2 as usize] = 65530;
        vm.add(Registers::R2, Registers::R2, 1, 1);
        assert_eq!(vm.registers[Registers::R2 as usize], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.load(Registers::R0, 15);
        assert_eq!(vm.registers[Registers::R0 as usize], 52);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Registers::PC as usize] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load(Registers::R1, 0b111111000);
        assert_eq!(vm.registers[Registers::R1 as usize], 50000);
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag.
        vm.load(Registers::R0, 0);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Registers::R0 as usize] = 52;
        assert_eq!(vm.memory[15], 0);
        vm.store(Registers::R0, 15);
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Registers::COND as usize], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Registers::PC as usize] = 2;
        vm.registers[Registers::R1 as usize] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(vm.memory[65530], 0);
        vm.store(Registers::R1, 0b111111000);
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Registers::COND as usize], 0); // Check flags. 
    }

    #[test]
    fn jump_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // JSR with positive offset
        vm.registers[Registers::PC as usize] = 3;
        vm.jump_register(1, 4);
        assert_eq!(vm.registers[Registers::PC as usize], 7);
        assert_eq!(vm.registers[Registers::R7 as usize], 3);
        // JSR with negative offset
        // PC is equal to 7 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.jump_register(1, 0b11111111000);
        assert_eq!(vm.registers[Registers::PC as usize], 65535);
        assert_eq!(vm.registers[Registers::R7 as usize], 7);
        // JSRR
        vm.registers[Registers::R6 as usize] = 365;
        vm.jump_register(0, Registers::R6 as u16);
        assert_eq!(vm.registers[Registers::PC as usize], 365);
        assert_eq!(vm.registers[Registers::R7 as usize], 65535);
    }

    #[test]
    fn bitwise_and_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Registers::R0 as usize] = 33;
        vm.registers[Registers::R5 as usize] = 5;
        vm.and(Registers::R4, Registers::R5, 0, Registers::R0 as u16);
        assert_eq!(vm.registers[Registers::R4 as usize], 1);
        assert_eq!(vm.registers[Registers::R0 as usize], 33);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        vm.registers[Registers::R2 as usize] = 65535;
        vm.registers[Registers::R3 as usize] = 65520;
        vm.and(Registers::R2, Registers::R2, 0, Registers::R3 as u16);
        assert_eq!(vm.registers[Registers::R2 as usize], 0xFFF0);
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        vm.and(Registers::R6, Registers::R2, 0, Registers::R1 as u16);
        assert_eq!(vm.registers[Registers::R6 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn bitwise_and_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.and(Registers::R5, Registers::R0, 1, 5);
        assert_eq!(vm.registers[Registers::R5 as usize], 0);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag. 

        // 20 is 1 0100 in binary, which is equal to -12 in two's complement notation for 9 bits.
        vm.registers[Registers::R5 as usize] = 5;
        vm.and(Registers::R7, Registers::R5, 1, 20);
        assert_eq!(vm.registers[Registers::R7 as usize], 4);
        assert_eq!(vm.registers[Registers::R5 as usize], 5);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag.

        // Register mode to check neg flag.
        vm.registers[Registers::R2 as usize] = 65535;
        vm.and(Registers::R2, Registers::R2, 1, 16);
        assert_eq!(vm.registers[Registers::R2 as usize], 0xFFF0); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.registers[Registers::R1 as usize] = 7;
        vm.load_register(Registers::R0, Registers::R1, 8);
        assert_eq!(vm.registers[Registers::R0 as usize], 52);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Registers::R2 as usize] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        vm.load_register(Registers::R1, Registers::R2, 0b111000);
        assert_eq!(vm.registers[Registers::R1 as usize], 50000);
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag. (R2 is equal to 2 from previous assertion set up)
        vm.load_register(Registers::R0, Registers::R2, 0);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Registers::R0 as usize] = 52;
        vm.registers[Registers::R1 as usize] = 7;
        assert_eq!(vm.memory[15], 0);
        vm.store_register(Registers::R0, Registers::R1, 8);
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Registers::COND as usize], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Registers::R2 as usize] = 2;
        vm.registers[Registers::R1 as usize] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        assert_eq!(vm.memory[65530], 0);
        vm.store_register(Registers::R1, Registers::R2, 0b111000);
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Registers::COND as usize], 0); // Check flags. 
    }

    #[test]
    fn bitwise_not() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Registers::R5 as usize] = 1;
        vm.not(Registers::R4, Registers::R5);
        assert_eq!(vm.registers[Registers::R4 as usize], 0xFFFE); // 0xFFFE is -2 in two'2 complement notation.
        assert_eq!(vm.registers[Registers::R5 as usize], 1);
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        vm.registers[Registers::R3 as usize] = 65520; //0xFFF0 
        vm.not(Registers::R2, Registers::R3);
        assert_eq!(vm.registers[Registers::R2 as usize], 15); //0x000F 
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        vm.registers[Registers::R2 as usize] = 0xFFFF;
        vm.not(Registers::R6, Registers::R2);
        assert_eq!(vm.registers[Registers::R6 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag.
    }

    #[test]
    fn load_indirect() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.memory[52] = 10;
        vm.registers[Registers::R1 as usize] = 7;
        vm.load_indirect(Registers::R0, 15);
        assert_eq!(vm.registers[Registers::R0 as usize], 10);
        assert_eq!(vm.registers[Registers::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Registers::PC as usize] = 2;
        vm.memory[65530] = 50000;
        vm.memory[50000] = 55555;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load_indirect(Registers::R1, 0b111111000);
        assert_eq!(vm.registers[Registers::R1 as usize], 55555);
        assert_eq!(vm.registers[Registers::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag. (PC is equal to 2 from previous assertion set up and address 2 in memory stores 0 for default)
        vm.load_indirect(Registers::R0, 0);
        assert_eq!(vm.registers[Registers::R0 as usize], 0);
        assert_eq!(vm.registers[Registers::COND as usize], 2); // Check Zro flag. 
    }
}
