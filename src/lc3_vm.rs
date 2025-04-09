use std::io::prelude::*;
use std::{char, io};

pub struct LC3VirtualMachine {
    pub memory: [u16; 1 << 16], /* 65536 locations */
    pub registers: [u16; 10],
    pub running:u8,
}

impl LC3VirtualMachine {
    pub fn new() -> Self {
        Self {
            memory: [0; 1 << 16],
            registers: [0; 10],
            running : 1,
        }
    }

    fn decode_instruction(&self, instrucction_16: u16) -> DecodedInstruction {
        DecodedInstruction {
            op_code: instrucction_16 >> 12,
            dst: Register::from_u16((instrucction_16 >> 9) & 0x7),
            src: Register::from_u16((instrucction_16 >> 6) & 0x7),
            alu_operand2: instrucction_16 & 0x1F,
            imm6: instrucction_16 & 0x3F,
            imm9: instrucction_16 & 0x1FF,
            imm11: instrucction_16 & 0x7FF,
            base_for_jump: (instrucction_16 >> 6) & 0x7,
            mode_alu: (instrucction_16 >> 5) & 0x1,
            flags: (instrucction_16 >> 9) & 0x7,
            mode_jump: (instrucction_16 >> 11) & 0x1,
            trapvect8: instrucction_16 & 0xFF,
        }
    }

    pub fn execute_instruction(&mut self, instrucction_16: u16) {
        if self.running == 0 {return;}
        let decoded_instruction = self.decode_instruction(instrucction_16);
        match Instruction::from_u16(decoded_instruction.op_code) {
            Instruction::OpBR =>
            /* branch */
            {
                self.branch(
                    Flags::from_u16(decoded_instruction.flags),
                    decoded_instruction.imm9,
                )
            }
            Instruction::OpADD =>
            /* add  */
            {
                self.add(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.mode_alu,
                    decoded_instruction.alu_operand2,
                )
            }

            Instruction::OpLD =>
            /* load */
            {
                self.load(decoded_instruction.dst, decoded_instruction.imm9)
            }
            Instruction::OpST =>
            /* store */
            {
                self.store(decoded_instruction.dst, decoded_instruction.imm9)
            }
            Instruction::OpJSR =>
            /* jump register */
            {
                let mut operand = decoded_instruction.imm11;
                if decoded_instruction.mode_jump == 0 {
                    operand = decoded_instruction.base_for_jump;
                }
                self.jump_register(decoded_instruction.mode_jump, operand)
            }
            Instruction::OpAND =>
            /* bitwise and */
            {
                self.and(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.mode_alu,
                    decoded_instruction.alu_operand2,
                )
            }
            Instruction::OpLDR => self.load_register(
                decoded_instruction.dst,
                decoded_instruction.src,
                decoded_instruction.imm6,
            ), /* load register */
            Instruction::OpSTR => self.store_register(
                decoded_instruction.dst,
                decoded_instruction.src,
                decoded_instruction.imm6,
            ), /* store register */
            Instruction::OpRTI => todo!(), /* unused */
            Instruction::OpNOT => self.not(decoded_instruction.dst, decoded_instruction.src), /* bitwise not */
            Instruction::OpLDI => {
                self.load_indirect(decoded_instruction.dst, decoded_instruction.imm9)
            } /* load indirect */
            Instruction::OpSTI => {
                self.store_indirect(decoded_instruction.dst, decoded_instruction.imm9)
            } /* store indirect */
            Instruction::OpJMP => self.jump(Register::from_u16(decoded_instruction.base_for_jump)), /* jump */
            Instruction::OpRES => todo!(), /* reserved (unused) */
            Instruction::OpLEA => {
                self.load_effective_address(decoded_instruction.dst, decoded_instruction.imm9)
            } /* load effective address */
            Instruction::OpTRAP => todo!(), /* execute trap */
        }
    }

    /// Checks if a determined flag is on.
    fn flag_is_on(&self, flag: Flags) -> bool {
        match flag {
            Flags::Pos => self.registers[Register::COND as usize] & 0b1 == 1,
            Flags::Zro => self.registers[Register::COND as usize] & 0b10 == 2,
            Flags::Neg => self.registers[Register::COND as usize] & 0b100 == 4,
        }
    }

    fn update_flags(&mut self, result_from_operation: u16) {
        if result_from_operation == 0 {
            self.registers[Register::COND as usize] = 2; // Flag Zro 0b10
        } else if result_from_operation & 0x8000 == 0 {
            self.registers[Register::COND as usize] = 1; // Flag Pos 0b1
        } else {
            self.registers[Register::COND as usize] = 4; // Flag Neg 0b100
        }
    }

    /// Extends sign for 9 bit numbers
    fn extend_sign(&mut self, number: u16, imm_size: usize) -> u16 {
        let extend_mask = 0xFFFF << imm_size;
        if (number >> (imm_size - 1)) & 1 == 1 {
            return number | extend_mask;
        }
        number
    }

    /// Branch instruction adds a 9 bit offset to the PC if the indicated flag is on.
    fn branch(&mut self, flag: Flags, pc_offset: u16) {
        if self.flag_is_on(flag) {
            let offset = self.extend_sign(pc_offset, 9);
            self.registers[Register::PC as usize] =
                self.registers[Register::PC as usize].wrapping_add(offset);
        }
    }

    /// Add istruction has two modes:
    /// Mode 0 => adds the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => adds the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// Add alters the flags depending on the result of the operation.
    fn add(&mut self, dst: Register, src1: Register, mode: u16, second_operand: u16) {
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
    fn load(&mut self, dst: Register, pc_offset: u16) {
        let mem_adress = self.registers[Register::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9)) as usize;
        self.registers[dst as usize] = self.memory[mem_adress];
        self.update_flags(self.memory[mem_adress]);
    }

    /// Store instruction loads into the memory address pc + pc_offset (9 bit immediate) the content in src register.
    /// Store doesn't alter flags.
    fn store(&mut self, src: Register, pc_offset: u16) {
        let mem_adress = self.registers[Register::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9)) as usize;
        self.memory[mem_adress] = self.registers[src as usize];
    }

    /// Jump Register stores the PC in R7 and then diverges in two modes:
    /// if long_flag == 1 the PC is updated to PC + operand (an 11 bit immediate value).
    /// if long_flag == 0 the PC takes the value stored in the register indicated by operand.
    fn jump_register(&mut self, long_flag: u16, operand: u16) {
        self.registers[Register::R7 as usize] = self.registers[Register::PC as usize];
        if long_flag == 1 {
            // JSR
            self.registers[Register::PC as usize] =
                self.registers[Register::PC as usize].wrapping_add(self.extend_sign(operand, 11));
        } else {
            // JSRR
            self.registers[Register::PC as usize] = self.registers[operand as usize];
        }
    }

    /// And istruction has two modes:
    /// Mode 0 => bitwise and between the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => bitwise and between the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// And alters the flags depending on the result of the operation.
    fn and(&mut self, dst: Register, src1: Register, mode: u16, second_operand: u16) {
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
    fn load_register(&mut self, dst: Register, src: Register, offset: u16) {
        let data_in_memory = self.memory
            [self.registers[src as usize].wrapping_add(self.extend_sign(offset, 6)) as usize];
        self.registers[dst as usize] = data_in_memory;
        self.update_flags(data_in_memory);
    }

    /// Store register instruction stores in memory the content in the src register.
    /// The memory address to store the value is calculated by adding the offset to the content in the dst register.
    fn store_register(&mut self, src: Register, dst: Register, offset: u16) {
        let memory_address = self.registers[dst as usize].wrapping_add(self.extend_sign(offset, 6));
        self.memory[memory_address as usize] = self.registers[src as usize];
    }

    /// Not instruction computes a bitwise not operation on the data in src register and stores the result in dst register.
    fn not(&mut self, dst: Register, src: Register) {
        let result = !self.registers[src as usize];
        self.registers[dst as usize] = result;
        self.update_flags(result);
    }

    /// Load Indirect instruction loads into dst register the content in the memory address found in memory at pc + pc_offset (9 bit immediate).
    /// Load Indirect alters flags depending the content loaded into the register.
    fn load_indirect(&mut self, dst: Register, pc_offset: u16) {
        let mem_adress = self.memory[self.registers[Register::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9))
            as usize];
        self.registers[dst as usize] = self.memory[mem_adress as usize];
        self.update_flags(self.memory[mem_adress as usize]);
    }

    /// Store Indirect instruction stores in memory the content in the src register.
    /// The memory address to store de value is obtained from the memory position in address pc + pc_offset (9 bit immediate).
    fn store_indirect(&mut self, src: Register, pc_offset: u16) {
        let memory_address = self.memory[self.registers[Register::PC as usize]
            .wrapping_add(self.extend_sign(pc_offset, 9))
            as usize];
        self.memory[memory_address as usize] = self.registers[src as usize];
    }

    /// Jump instruction sets PC register with the value of the indicated register in the arguments.
    fn jump(&mut self, base_register: Register) {
        self.registers[Register::PC as usize] = self.registers[base_register as usize];
    }

    /// Load effective adress loads dst register with the adress stored in the PC plus an offset.
    fn load_effective_address(&mut self, dst: Register, pc_offset: u16) {
        let effective_adress =
            self.registers[Register::PC as usize].wrapping_add(self.extend_sign(pc_offset, 9));
        self.registers[dst as usize] = effective_adress;
        self.update_flags(effective_adress);
    }

    /// Reads input character from stdin.
    fn getchar(&self) -> Result<u8, std::io::Error> {
        let mut buffer = [0; 1];
        let _ = io::stdin().read(&mut buffer)?;
        
        Ok(buffer[0])
    }

    /// Writes character in stdout.
    fn putchar(&self, char_to_write: char) -> io::Result<()> {
        io::stdout().write(&[char_to_write as u8])?;
        Ok(())
    }

    /// Writes in stdout string stored in memory address in src register. Each address stores one char.
    pub fn trap_puts(&mut self, src: Register) -> io::Result<()> {
        let mut character_address_in_memory = self.registers[src as usize] as usize;
        while self.memory[character_address_in_memory] != 0 {
            let char_to_write =
                char::from_u32(self.memory[character_address_in_memory] as u32).unwrap(); //TODO: Handle this as error
            self.putchar(char_to_write)?;
            character_address_in_memory += 1;
        }
        io::stdout().flush().expect("Stdout error");
        Ok(())
    }

    /// Stores input character in dst register.
    pub fn trap_getc(&mut self, dst: Register) -> io::Result<()> {
        let read_byte = self.getchar().unwrap();
        self.registers[dst as usize] = read_byte as u16;
        Ok(())
    }

    /// Writes in stdout the char in store in src.
    pub fn trap_out(&mut self, src: Register) -> io::Result<()> {
        let char_to_write = char::from_u32(self.registers[src as usize] as u32).unwrap(); //TODO: Handle this as error
        self.putchar(char_to_write)?;
        io::stdout().flush().expect("Stdout error");
        Ok(())
    }

    /// Reads a character written in stdin, then writes it in stdout and stores it in dst register.
    pub fn trap_in(&mut self, dst: Register) -> io::Result<()> {
        println!("Enter a character: ");
        let read_char = self.getchar()?;
        let char_to_write = char::from_u32(read_char as u32).unwrap(); //TODO: Handle this as error
        self.putchar(char_to_write)?;
        io::stdout().flush().expect("Stdout error");
        self.registers[dst as usize] = char_to_write as u16;
        self.update_flags(char_to_write as u16);
        Ok(())
    }

    /// Writes in stdout the stored in memory address in src register. Each address stores 4 chars in little endian format.
    pub fn trap_putsp(&mut self, src: Register) -> io::Result<()> {
        let mut character_address_in_memory = self.registers[src as usize] as usize;
        while self.memory[character_address_in_memory] != 0 {
            let chars_to_write = self.process_chars_for_writting(self.memory[character_address_in_memory]);
            for char in chars_to_write{
                self.putchar(char)?;
            }
            character_address_in_memory += 1;
        }
        io::stdout().flush().expect("Stdout error");
        Ok(())
    }

    /// Turns two chars read as little endian format into big endian format.
    fn process_chars_for_writting(&self, chars:u16)->[char;2]{
        [char::from_u32((chars & 0xFF) as u32).unwrap(),char::from_u32((chars>>8 & 0xFF) as u32).unwrap()]
    }

    pub fn trap_halt(&mut self){
        io::stdout().flush().expect("Stdout error");
        self.running = 0;
    }
}

pub enum Register {
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

impl Register {
    fn from_u16(value: u16) -> Self {
        match value {
            0 => Self::R0,
            1 => Self::R1,
            2 => Self::R2,
            3 => Self::R3,
            4 => Self::R4,
            5 => Self::R5,
            6 => Self::R6,
            7 => Self::R7,
            8 => Self::PC,
            9 => Self::COND,
            _ => {
                todo!() //Invalid Register
            }
        }
    }
}

enum Flags {
    Pos,
    Zro,
    Neg,
}

impl Flags {
    fn from_u16(value: u16) -> Self {
        match value {
            0 => todo!(), //Invalid Flag
            1 => Self::Pos,
            2 => Self::Zro,
            3 => todo!(), //Invalid Flag
            4 => Self::Neg,
            _ => {
                todo!() //Invalid Flag
            }
        }
    }
}
pub enum Instruction {
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

impl Instruction {
    fn from_u16(value: u16) -> Self {
        match value {
            0 => Self::OpBR,    /* branch */
            1 => Self::OpADD,   /* add  */
            2 => Self::OpLD,    /* load */
            3 => Self::OpST,    /* store */
            4 => Self::OpJSR,   /* jump register */
            5 => Self::OpAND,   /* bitwise and */
            6 => Self::OpLDR,   /* load register */
            7 => Self::OpSTR,   /* store register */
            8 => Self::OpRTI,   /* unused */
            9 => Self::OpNOT,   /* bitwise not */
            10 => Self::OpLDI,  /* load indirect */
            11 => Self::OpSTI,  /* store indirect */
            12 => Self::OpJMP,  /* jump */
            13 => Self::OpRES,  /* reserved (unused) */
            14 => Self::OpLEA,  /* load effective address */
            15 => Self::OpTRAP, /* execute trap */
            _ => {
                todo!() //Invalid OpCode
            }
        }
    }
}

struct DecodedInstruction {
    op_code: u16,
    dst: Register,
    src: Register,
    alu_operand2: u16, //It can be either an imm of 5 bits or a register number
    imm6: u16,
    imm9: u16,
    imm11: u16,
    base_for_jump: u16,
    mode_alu: u16,
    flags: u16,
    mode_jump: u16,
    trapvect8: u16,
}

pub enum TrapCode {
    TrapGetc = 0x20,  /* get character from keyboard, not echoed onto the terminal */
    TrapOut = 0x21,   /* output a character */
    TrapPuts = 0x22,  /* output a word string */
    TrapIn = 0x23,    /* get character from keyboard, echoed onto the terminal */
    TrapPutsp = 0x24, /* output a byte string */
    TrapHalt = 0x25,  /* halt the program */
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_and_index_mut_with_registers() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        vm.registers[Register::R0 as usize] = 16;
        assert_eq!(vm.registers[Register::R0 as usize], 16);
    }

    #[test]
    /// When initializing the vm all registers are initialized with zero, so when executing a conditional branch PC
    /// should stay equal to zero.
    fn branch_instruction_no_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Register::PC as usize], 0);
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Register::PC as usize], 0);
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Register::PC as usize], 0);
    }

    #[test]
    fn branch_instruction_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::COND as usize] = 1; // Set only Pos flag in 1.
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Register::PC as usize], 16);
        vm.registers[Register::COND as usize] = 2; // Set only Zro flag in 1.
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Register::PC as usize], 32);
        vm.registers[Register::COND as usize] = 4; // Set only Neg flag in 1.
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Register::PC as usize], 48);
        vm.branch(Flags::Neg, 0xFFFF);
        assert_eq!(vm.registers[Register::PC as usize], 47);
    }

    #[test]
    fn add_instruction_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        vm.registers[Register::R0 as usize] = 32;
        vm.registers[Register::R5 as usize] = 5;
        vm.add(Register::R4, Register::R5, 0, Register::R0 as u16);
        assert_eq!(vm.registers[Register::R4 as usize], 37);
        assert_eq!(vm.registers[Register::R0 as usize], 32);
        assert_eq!(vm.registers[Register::R5 as usize], 5);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        vm.registers[Register::R1 as usize] = 1;
        vm.registers[Register::R2 as usize] = 65530;
        vm.add(Register::R2, Register::R2, 0, Register::R1 as u16);
        assert_eq!(vm.registers[Register::R2 as usize], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        vm.add(Register::R2, Register::R2, 0, Register::R5 as u16);
        assert_eq!(vm.registers[Register::R2 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag.
    }

    #[test]
    fn add_instruction_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // Immediate mode add positive number.
        vm.add(Register::R5, Register::R0, 1, 5);
        assert_eq!(vm.registers[Register::R5 as usize], 5);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 
        // Register mode.
        vm.registers[Register::R4 as usize] = 65535; // 65535 in u16 is 0xFFFF which is equal to -1 in two'2 complement notation.
        // Immediate mode add negative number.
        vm.add(Register::R7, Register::R4, 1, 1);
        assert_eq!(vm.registers[Register::R7 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag.

        vm.registers[Register::R2 as usize] = 65530;
        vm.add(Register::R2, Register::R2, 1, 1);
        assert_eq!(vm.registers[Register::R2 as usize], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.load(Register::R0, 15);
        assert_eq!(vm.registers[Register::R0 as usize], 52);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC as usize] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load(Register::R1, 0b111111000);
        assert_eq!(vm.registers[Register::R1 as usize], 50000);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag.
        vm.load(Register::R0, 0);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Register::R0 as usize] = 52;
        assert_eq!(vm.memory[15], 0);
        vm.store(Register::R0, 15);
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::PC as usize] = 2;
        vm.registers[Register::R1 as usize] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(vm.memory[65530], 0);
        vm.store(Register::R1, 0b111111000);
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 
    }

    #[test]
    fn jump_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // JSR with positive offset
        vm.registers[Register::PC as usize] = 3;
        vm.jump_register(1, 4);
        assert_eq!(vm.registers[Register::PC as usize], 7);
        assert_eq!(vm.registers[Register::R7 as usize], 3);
        // JSR with negative offset
        // PC is equal to 7 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.jump_register(1, 0b11111111000);
        assert_eq!(vm.registers[Register::PC as usize], 65535);
        assert_eq!(vm.registers[Register::R7 as usize], 7);
        // JSRR
        vm.registers[Register::R6 as usize] = 365;
        vm.jump_register(0, Register::R6 as u16);
        assert_eq!(vm.registers[Register::PC as usize], 365);
        assert_eq!(vm.registers[Register::R7 as usize], 65535);
    }

    #[test]
    fn bitwise_and_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R0 as usize] = 33;
        vm.registers[Register::R5 as usize] = 5;
        vm.and(Register::R4, Register::R5, 0, Register::R0 as u16);
        assert_eq!(vm.registers[Register::R4 as usize], 1);
        assert_eq!(vm.registers[Register::R0 as usize], 33);
        assert_eq!(vm.registers[Register::R5 as usize], 5);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        vm.registers[Register::R2 as usize] = 65535;
        vm.registers[Register::R3 as usize] = 65520;
        vm.and(Register::R2, Register::R2, 0, Register::R3 as u16);
        assert_eq!(vm.registers[Register::R2 as usize], 0xFFF0);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        vm.and(Register::R6, Register::R2, 0, Register::R1 as u16);
        assert_eq!(vm.registers[Register::R6 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn bitwise_and_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.and(Register::R5, Register::R0, 1, 5);
        assert_eq!(vm.registers[Register::R5 as usize], 0);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 

        // 20 is 1 0100 in binary, which is equal to -12 in two's complement notation for 9 bits.
        vm.registers[Register::R5 as usize] = 5;
        vm.and(Register::R7, Register::R5, 1, 20);
        assert_eq!(vm.registers[Register::R7 as usize], 4);
        assert_eq!(vm.registers[Register::R5 as usize], 5);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag.

        // Register mode to check neg flag.
        vm.registers[Register::R2 as usize] = 65535;
        vm.and(Register::R2, Register::R2, 1, 16);
        assert_eq!(vm.registers[Register::R2 as usize], 0xFFF0); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.registers[Register::R1 as usize] = 7;
        vm.load_register(Register::R0, Register::R1, 8);
        assert_eq!(vm.registers[Register::R0 as usize], 52);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::R2 as usize] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        vm.load_register(Register::R1, Register::R2, 0b111000);
        assert_eq!(vm.registers[Register::R1 as usize], 50000);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag. (R2 is equal to 2 from previous assertion set up)
        vm.load_register(Register::R0, Register::R2, 0);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Register::R0 as usize] = 52;
        vm.registers[Register::R1 as usize] = 7;
        assert_eq!(vm.memory[15], 0);
        vm.store_register(Register::R0, Register::R1, 8);
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::R2 as usize] = 2;
        vm.registers[Register::R1 as usize] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        assert_eq!(vm.memory[65530], 0);
        vm.store_register(Register::R1, Register::R2, 0b111000);
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 
    }

    #[test]
    fn bitwise_not() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R5 as usize] = 1;
        vm.not(Register::R4, Register::R5);
        assert_eq!(vm.registers[Register::R4 as usize], 0xFFFE); // 0xFFFE is -2 in two'2 complement notation.
        assert_eq!(vm.registers[Register::R5 as usize], 1);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        vm.registers[Register::R3 as usize] = 65520; //0xFFF0 
        vm.not(Register::R2, Register::R3);
        assert_eq!(vm.registers[Register::R2 as usize], 15); //0x000F 
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        vm.registers[Register::R2 as usize] = 0xFFFF;
        vm.not(Register::R6, Register::R2);
        assert_eq!(vm.registers[Register::R6 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag.
    }

    #[test]
    fn load_indirect() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.memory[52] = 10;
        vm.registers[Register::R1 as usize] = 7;
        vm.load_indirect(Register::R0, 15);
        assert_eq!(vm.registers[Register::R0 as usize], 10);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC as usize] = 2;
        vm.memory[65530] = 50000;
        vm.memory[50000] = 55555;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load_indirect(Register::R1, 0b111111000);
        assert_eq!(vm.registers[Register::R1 as usize], 55555);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag. (PC is equal to 2 from previous assertion set up and address 2 in memory stores 0 for default)
        vm.load_indirect(Register::R0, 0);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn store_indirect() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset. (PC is equal to 0 for default)
        vm.memory[17] = 7;
        vm.registers[Register::R0 as usize] = 52;
        assert_eq!(vm.memory[15], 0);
        vm.store_indirect(Register::R0, 17);
        assert_eq!(vm.memory[7], 52);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::PC as usize] = 2;
        vm.memory[65530] = 50000;
        vm.registers[Register::R1 as usize] = 65000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.store_indirect(Register::R1, 0b111111000);
        assert_eq!(vm.memory[50000], 65000);
        assert_eq!(vm.registers[Register::COND as usize], 0); // Check flags. 
    }

    #[test]
    fn load_effective_address() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.load_effective_address(Register::R0, 15);
        assert_eq!(vm.registers[Register::R0 as usize], 15);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC as usize] = 2;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load_effective_address(Register::R1, 0b111111000);
        assert_eq!(vm.registers[Register::R1 as usize], 65530);
        assert_eq!(vm.registers[Register::COND as usize], 4); // Check Neg flag. 

        // Load to check Zro flag. (PC is equal to 2 from previous assertion, pc_offset is equal to -2 in two'2 complement notation.)
        vm.load_effective_address(Register::R0, 0xFFFE);
        assert_eq!(vm.registers[Register::R0 as usize], 0);
        assert_eq!(vm.registers[Register::COND as usize], 2); // Check Zro flag. 
    }

    #[test]
    fn execute() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R1 as usize] = 32;
        vm.registers[Register::R2 as usize] = 5;
        let instruction = 0b0001000001000010; //ADD r0, r1, r2
        vm.execute_instruction(instruction);
        assert_eq!(vm.registers[Register::R0 as usize], 37);
        assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag. 
    }
}
