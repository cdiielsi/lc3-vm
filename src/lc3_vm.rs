use console::Term;
use raw_tty::GuardMode;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::Duration;
use std::{char, io};
use termios::Termios;
use timeout_readwrite::TimeoutReader;

use crate::hardware::{
    DecodedInstruction, Flags, HardwareError, Instruction, MemoryMappedRegisters, Register,
    TrapCode,
};

pub struct LC3VirtualMachine {
    pub memory: [u16; 1 << 16], /* 65536 locations */
    pub registers: [u16; 10],
    pub running: bool,
    pub origin: u16,
}

#[derive(PartialEq, Debug)]
pub enum VMError {
    FailedToLoadImage(String),
    InvalidInstruction(HardwareError),
    IOError(String),
    InvalidTrapCode(HardwareError),
    TerminalError(String),
    InvalidAddress(u16),
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            VMError::FailedToLoadImage(value) => &format!("Failed to load image: {:?}", value),
            VMError::InvalidInstruction(hardware_error) => {
                &format!("Invalid Instruction: {:?}", hardware_error.fmt(f))
            }
            VMError::IOError(value) => &format!("IO Error: {:?}", value),
            VMError::InvalidTrapCode(hardware_error) => {
                &format!("Invalid Instruction: {:?}", hardware_error.fmt(f))
            }
            VMError::TerminalError(value) => &format!("Terminal Error: {:?}", value),
            VMError::InvalidAddress(value) => &format!("Invalid Address: {}", value),
        };
        f.write_str(description)
    }
}

impl LC3VirtualMachine {
    pub fn new() -> Self {
        Self {
            memory: [0; 1 << 16],
            registers: [0; 10],
            running: false,
            origin: 0x3000,
        }
    }

    fn mem_available_space(&self) -> usize {
        self.memory.len() - self.origin as usize
    }

    pub fn set_pc_with_origin(&mut self) {
        self.registers[Register::PC] = self.origin;
    }

    pub fn turn_pos_flag_on(&mut self) {
        self.registers[Register::COND] = 1;
    }

    fn mem_write(&mut self, address: u16, value: u16) -> Result<(), VMError> {
        if address as usize > self.memory.len() {
            return Err(VMError::InvalidAddress(address));
        }
        self.memory[address as usize] = value;
        Ok(())
    }

    fn mem_read(&mut self, address: u16) -> Result<u16, VMError> {
        if address == MemoryMappedRegisters::MrKBSR as u16 {
            let mut stdin = std::io::stdin()
                .guard_mode()
                .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
            let mut input_buffer = [1; 1];
            let mut rdr = TimeoutReader::new(&mut *stdin, Duration::from_millis(50000));
            rdr.read_exact(&mut input_buffer)
                .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
            if input_buffer[0] != 0 {
                // If any key is being pressed
                self.memory[MemoryMappedRegisters::MrKBSR as usize] = 1 << 15;
                self.memory[MemoryMappedRegisters::MrKBDR as usize] = input_buffer[0] as u16;
            } else {
                self.memory[MemoryMappedRegisters::MrKBSR as usize] = 0;
            }
        }
        Ok(self.memory[address as usize])
    }

    pub fn run(&mut self) -> Result<(), VMError> {
        self.running = true;
        while self.running {
            let instruction_u16 = self.mem_read(self.registers[Register::PC])?; // Read Instruction from memory
            self.registers[Register::PC] = self.registers[Register::PC].wrapping_add(1); // PC + 1
            self.execute_instruction(instruction_u16)?;
        }
        Ok(())
    }

    fn execute_instruction(&mut self, instrucction_16: u16) -> Result<(), VMError> {
        let decoded_instruction = DecodedInstruction::decode_instruction(instrucction_16)
            .map_err(VMError::InvalidInstruction)?;
        match Instruction::from_u16(decoded_instruction.op_code)
            .map_err(VMError::InvalidInstruction)?
        {
            Instruction::OpBR =>
            /* branch */
            {
                self.branch(
                    Flags::from_u16(decoded_instruction.flags)
                        .map_err(VMError::InvalidInstruction)?,
                    decoded_instruction.imm9,
                );
                Ok(())
            }
            Instruction::OpADD =>
            /* add */
            {
                self.add(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.mode_alu,
                    decoded_instruction.alu_operand2,
                );
                Ok(())
            }

            Instruction::OpLD =>
            /* load */
            {
                self.load(decoded_instruction.dst, decoded_instruction.imm9)?;
                Ok(())
            }
            Instruction::OpST =>
            /* store */
            {
                self.store(decoded_instruction.dst, decoded_instruction.imm9)?;
                Ok(())
            }
            Instruction::OpJSR =>
            /* jump register */
            {
                let mut operand = decoded_instruction.imm11;
                if decoded_instruction.mode_jump == 0 {
                    operand = decoded_instruction.base_for_jump;
                }
                self.jump_register(decoded_instruction.mode_jump, operand);
                Ok(())
            }
            Instruction::OpAND =>
            /* bitwise and */
            {
                self.and(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.mode_alu,
                    decoded_instruction.alu_operand2,
                );
                Ok(())
            }
            Instruction::OpLDR =>
            /* load register */
            {
                self.load_register(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.imm6,
                )?;
                Ok(())
            }
            Instruction::OpSTR =>
            /* store register */
            {
                self.store_register(
                    decoded_instruction.dst,
                    decoded_instruction.src,
                    decoded_instruction.imm6,
                )?;
                Ok(())
            }
            Instruction::OpNOT =>
            /* bitwise not */
            {
                self.not(decoded_instruction.dst, decoded_instruction.src);
                Ok(())
            }
            Instruction::OpLDI =>
            /* load indirect */
            {
                self.load_indirect(decoded_instruction.dst, decoded_instruction.imm9)?;
                Ok(())
            }
            Instruction::OpSTI =>
            /* store indirect */
            {
                self.store_indirect(decoded_instruction.dst, decoded_instruction.imm9)?;
                Ok(())
            }
            Instruction::OpJMP => {
                /* jump */
                self.jump(
                    Register::from_u16(decoded_instruction.base_for_jump)
                        .map_err(VMError::InvalidInstruction)?,
                );
                Ok(())
            }
            Instruction::OpLEA => {
                /* load effective address */
                self.load_effective_address(decoded_instruction.dst, decoded_instruction.imm9);
                Ok(())
            }
            Instruction::OpTRAP => {
                /* execute trap */
                self.registers[Register::R7] = self.registers[Register::PC];
                self.execute_trap_routine(
                    TrapCode::from_u16(decoded_instruction.trapvect8)
                        .map_err(VMError::InvalidTrapCode)?,
                )?;
                self.registers[Register::PC] = self.registers[Register::R7];
                Ok(())
            }
        }
    }

    fn execute_trap_routine(&mut self, trap_code: TrapCode) -> Result<(), VMError> {
        match trap_code {
            TrapCode::Getc => self.trap_getc(),
            TrapCode::In => self.trap_in(),
            TrapCode::Out => self.trap_out(),
            TrapCode::Puts => self.trap_puts(),
            TrapCode::Putsp => self.trap_putsp(),
            TrapCode::Halt => {
                self.trap_halt();
                Ok(())
            }
        }
    }

    /// Checks if a determined flag is on.
    fn flag_is_on(&self, flag: Flags) -> bool {
        match flag {
            Flags::Pos => self.registers[Register::COND] & 0b001 == 1,
            Flags::Zro => self.registers[Register::COND] & 0b010 == 2,
            Flags::Neg => self.registers[Register::COND] & 0b100 == 4,
            Flags::PosZro => self.registers[Register::COND] & 0b011 > 0,
            Flags::NotZro => self.registers[Register::COND] & 0b001 == 0,
            Flags::PosNeg => self.registers[Register::COND] & 0b101 > 0,
            Flags::PosZroNeg => self.registers[Register::COND] & 0b111 > 0,
            _ => false,
        }
    }

    /// Only one flag at a time is on.
    fn update_flags(&mut self, result_from_operation: u16) {
        if result_from_operation == 0 {
            self.registers[Register::COND] = 2; // Flag Zro 0b10
        } else if result_from_operation & 0x8000 == 0 {
            self.registers[Register::COND] = 1; // Flag Pos 0b1
        } else {
            self.registers[Register::COND] = 4; // Flag Neg 0b100
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
            self.registers[Register::PC] = self.registers[Register::PC].wrapping_add(offset);
        }
    }

    /// Add istruction has two modes:
    /// Mode 0 => adds the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => adds the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// Add alters the flags depending on the result of the operation.
    fn add(&mut self, dst: Register, src1: Register, mode: u16, second_operand: u16) {
        let mut result = 0;
        if mode == 0 {
            result = self.registers[src1].wrapping_add(self.registers[second_operand as usize]);
        } else if mode == 1 {
            result = self.registers[src1].wrapping_add(self.extend_sign(second_operand, 5));
        }
        self.registers[dst] = result;
        self.update_flags(result);
    }

    /// Load instruction loads into dst register the content in the memory address pc + pc_offset (9 bit immediate).
    /// Load alters flags depending the content loaded into the register.
    fn load(&mut self, dst: Register, pc_offset: u16) -> Result<(), VMError> {
        let mem_adress = self.registers[Register::PC].wrapping_add(self.extend_sign(pc_offset, 9));
        self.registers[dst] = self.mem_read(mem_adress)?;
        self.update_flags(self.memory[mem_adress as usize]);
        Ok(())
    }

    /// Store instruction loads into the memory address pc + pc_offset (9 bit immediate) the content in src register.
    /// Store doesn't alter flags.
    fn store(&mut self, src: Register, pc_offset: u16) -> Result<(), VMError> {
        let mem_address =
            self.registers[Register::PC].wrapping_add(self.extend_sign(pc_offset, 9)) as usize;
        self.mem_write(mem_address as u16, self.registers[src])?;
        Ok(())
    }

    /// Jump Register stores the PC in R7 and then diverges in two modes:
    /// if long_flag == 1 the PC is updated to PC + operand (an 11 bit immediate value).
    /// if long_flag == 0 the PC takes the value stored in the register indicated by operand.
    fn jump_register(&mut self, long_flag: u16, operand: u16) {
        self.registers[Register::R7] = self.registers[Register::PC];
        if long_flag == 1 {
            // JSR
            self.registers[Register::PC] =
                self.registers[Register::PC].wrapping_add(self.extend_sign(operand, 11));
        } else {
            // JSRR
            self.registers[Register::PC] = self.registers[operand as usize];
        }
    }

    /// And istruction has two modes:
    /// Mode 0 => bitwise and between the data from registers src1 and second_operand and stores the result in dst register.
    /// Mode 1 => bitwise and between the data from register src1 and the 5 bit immediate second_operand and stores the result in dst register.
    /// And alters the flags depending on the result of the operation.
    fn and(&mut self, dst: Register, src1: Register, mode: u16, second_operand: u16) {
        let mut result = 0;
        if mode == 0 {
            result = self.registers[src1] & self.registers[second_operand as usize];
        } else if mode == 1 {
            result = self.registers[src1] & self.extend_sign(second_operand, 5);
        }
        self.registers[dst] = result;
        self.update_flags(result);
    }

    /// Load register instruction loads into dst register the content in the memory addres obtained by adding the
    /// content of src register and offset (6 bit immediate).
    /// Load register alters flags depending the content loaded into the dst register.
    fn load_register(&mut self, dst: Register, src: Register, offset: u16) -> Result<(), VMError> {
        let extended_offset = self.extend_sign(offset, 6);
        let data_in_memory = self.mem_read(self.registers[src].wrapping_add(extended_offset))?;
        self.registers[dst] = data_in_memory;
        self.update_flags(data_in_memory);
        Ok(())
    }

    /// Store register instruction stores in memory the content in the src register.
    /// The memory address to store the value is calculated by adding the offset to the content in the dst register.
    fn store_register(&mut self, src: Register, dst: Register, offset: u16) -> Result<(), VMError> {
        let memory_address = self.registers[dst].wrapping_add(self.extend_sign(offset, 6));
        self.mem_write(memory_address, self.registers[src])?;
        Ok(())
    }

    /// Not instruction computes a bitwise not operation on the data in src register and stores the result in dst register.
    fn not(&mut self, dst: Register, src: Register) {
        let result = !self.registers[src];
        self.registers[dst] = result;
        self.update_flags(result);
    }

    /// Load Indirect instruction loads into dst register the content in the memory address found in memory at pc + pc_offset (9 bit immediate).
    /// Load Indirect alters flags depending the content loaded into the register.
    fn load_indirect(&mut self, dst: Register, pc_offset: u16) -> Result<(), VMError> {
        let pc_offset_u16 = self.extend_sign(pc_offset, 9);

        let mem_adress = self.mem_read(self.registers[Register::PC].wrapping_add(pc_offset_u16))?;
        self.registers[dst] = self.mem_read(mem_adress)?;
        self.update_flags(self.memory[mem_adress as usize]);
        Ok(())
    }

    /// Store Indirect instruction stores in memory the content in the src register.
    /// The memory address to store de value is obtained from the memory position in address pc + pc_offset (9 bit immediate).
    fn store_indirect(&mut self, src: Register, pc_offset: u16) -> Result<(), VMError> {
        let memory_address = self.memory
            [self.registers[Register::PC].wrapping_add(self.extend_sign(pc_offset, 9)) as usize];
        self.mem_write(memory_address, self.registers[src])?;
        Ok(())
    }

    /// Jump instruction sets PC register with the value of the indicated register in the arguments.
    fn jump(&mut self, base_register: Register) {
        self.registers[Register::PC] = self.registers[base_register];
    }

    /// Load effective adress loads dst register with the adress stored in the PC plus an offset.
    fn load_effective_address(&mut self, dst: Register, pc_offset: u16) {
        let effective_adress =
            self.registers[Register::PC].wrapping_add(self.extend_sign(pc_offset, 9));
        self.registers[dst] = effective_adress;
        self.update_flags(effective_adress);
    }

    /// Writes in stdout string stored in memory address in R0. Each address stores one char.
    fn trap_puts(&mut self) -> Result<(), VMError> {
        let mut term = Term::stdout();
        let mut character_address_in_memory = self.registers[Register::R0] as usize;
        while self.memory[character_address_in_memory] != 0 {
            let char_to_write = self.memory[character_address_in_memory] as u8 as char;
            putchar(&mut term, char_to_write)?;
            character_address_in_memory += 1;
        }
        term.flush()
            .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
        Ok(())
    }

    /// Stores input character in R0.
    fn trap_getc(&mut self) -> Result<(), VMError> {
        let read_byte = getchar().map_err(|error| VMError::IOError(format!("{:?}", error)))?;
        self.registers[Register::R0] = read_byte as u16;
        Ok(())
    }

    /// Writes in stdout the char in store in R0.
    fn trap_out(&mut self) -> Result<(), VMError> {
        let mut term = Term::stdout();
        let char_to_write = self.registers[Register::R0] as u8 as char;
        putchar(&mut term, char_to_write)?;
        term.flush()
            .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
        Ok(())
    }

    /// Reads a character written in stdin, then writes it in stdout and stores it in R0.
    fn trap_in(&mut self) -> Result<(), VMError> {
        println!("Enter a character: ");
        let read_char = getchar()?;
        let mut term = Term::stdout();
        putchar(&mut term, read_char)?;
        term.flush()
            .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
        self.registers[Register::R0] = read_char as u16;
        self.update_flags(read_char as u16);
        Ok(())
    }

    /// Writes in stdout the stored in memory address in R0. Each address stores 4 chars in little endian format.
    fn trap_putsp(&mut self) -> Result<(), VMError> {
        let mut term = Term::stdout();
        let mut character_address_in_memory = self.registers[Register::R0] as usize;
        while (self.memory[character_address_in_memory]) != 0
            || (self.memory[character_address_in_memory]) != 3
        {
            let chars_to_write = self.memory[character_address_in_memory].to_le_bytes();
            // Turns two chars read from a word as little endian format into big endian format. Since chars are
            // already little  endian to turn them to the other format it's necesary to apply to_le_bytes() because
            // this is the function that makes the bytes interchange places.
            for char in chars_to_write {
                putchar(&mut term, char as char)?;
            }
            if (self.memory[character_address_in_memory] & 0xFF00) == 0
                || (self.memory[character_address_in_memory] & 0xFF00) == 0x0300
            {
                // When a string has an odd number of not NULL chars the NULL character is within
                // the same read word as another char, so loop condition misses the NUll character
                // and this extra check is necesary to figure out when the string ends.
                break;
            }
            character_address_in_memory += 1;
        }
        term.flush()
            .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
        Ok(())
    }

    fn trap_halt(&mut self) {
        Term::stdout().flush().expect("Stdout error");
        self.running = false;
    }
}

/// Reads input character from stdin.
fn getchar() -> Result<char, VMError> {
    let mut term = io::stdin();
    let mut buff: [u8; 1] = [0; 1];
    term.read(&mut buff)
        .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
    Ok(buff[0] as char)
}

/// Writes character in stdout.
fn putchar(term: &mut Term, char_to_write: char) -> Result<(), VMError> {
    term.write_all(&[char_to_write as u8])
        .map_err(|error| VMError::IOError(format!("{:?}", error)))?;
    Ok(())
}

pub fn read_image(vm: &mut LC3VirtualMachine, img_file_path: &str) -> Result<(), VMError> {
    let mut image = File::open(img_file_path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    image
        .read_to_end(&mut buffer)
        .map_err(|error| VMError::FailedToLoadImage(format!("{:?}", error)))?;
    read_image_file(vm, buffer)?;
    Ok(())
}

pub fn read_image_file(
    vm: &mut LC3VirtualMachine,
    image_in_buffer: Vec<u8>,
) -> Result<(), VMError> {
    // Image as vec<u8> has to have even length to convert to u16 words.
    // Image with length smaller than 2 is an invalid image
    // Image has to fit in memory space starting at origin address.
    if image_in_buffer.len() % 2 != 0
        || image_in_buffer.len() < 2
        || image_in_buffer.len() / 2 > vm.mem_available_space()
    {
        return Err(VMError::FailedToLoadImage(String::from(
            "Image size invalid",
        )));
    }
    let mut current_adress = u16::from_be_bytes([image_in_buffer[0], image_in_buffer[1]]);
    let mut i = 2;
    while i < image_in_buffer.len() - 1 {
        vm.mem_write(
            current_adress,
            u16::from_be_bytes([image_in_buffer[i], image_in_buffer[i + 1]]),
        )?;
        current_adress += 1;
        i += 2;
    }
    Ok(())
}

pub fn disable_input_buffering(original_tio: &mut Termios) -> Result<(), VMError> {
    termios::tcgetattr(0, original_tio)
        .map_err(|error| VMError::TerminalError(format!("{:?}", error)))?; // stdin fd
    let new_tio = original_tio;
    new_tio.c_lflag &= !termios::os::target::ICANON & !termios::os::target::ECHO;
    termios::tcsetattr(0, termios::os::target::TCSANOW, new_tio)
        .map_err(|error| VMError::TerminalError(format!("{:?}", error)))?;
    Ok(())
}

pub fn restore_input_buffering(original_tio: &mut Termios) -> Result<(), VMError> {
    termios::tcsetattr(0, termios::os::target::TCSANOW, original_tio)
        .map_err(|error| VMError::TerminalError(format!("{:?}", error)))?; // stdin fd
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_and_index_mut_with_registers() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        assert_eq!(vm.registers[Register::R0], 0);
        vm.registers[Register::R0] = 16;
        assert_eq!(vm.registers[Register::R0], 16);
    }

    #[test]
    /// When initializing the vm all registers are initialized with zero, so when executing a conditional branch PC
    /// should stay equal to zero.
    fn branch_instruction_no_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Register::PC], 0);
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Register::PC], 0);
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Register::PC], 0);
    }

    #[test]
    fn branch_instruction_branching() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::COND] = 1; // Set only Pos flag in 1.
        vm.branch(Flags::Pos, 16);
        assert_eq!(vm.registers[Register::PC], 16);
        vm.registers[Register::COND] = 2; // Set only Zro flag in 1.
        vm.branch(Flags::Zro, 16);
        assert_eq!(vm.registers[Register::PC], 32);
        vm.registers[Register::COND] = 4; // Set only Neg flag in 1.
        vm.branch(Flags::Neg, 16);
        assert_eq!(vm.registers[Register::PC], 48);
        vm.branch(Flags::Neg, 0xFFFF);
        assert_eq!(vm.registers[Register::PC], 47);
    }

    #[test]
    fn add_instruction_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        vm.registers[Register::R0] = 32;
        vm.registers[Register::R5] = 5;
        vm.add(Register::R4, Register::R5, 0, Register::R0 as u16);
        assert_eq!(vm.registers[Register::R4], 37);
        assert_eq!(vm.registers[Register::R0], 32);
        assert_eq!(vm.registers[Register::R5], 5);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        vm.registers[Register::R1] = 1;
        vm.registers[Register::R2] = 65530;
        vm.add(Register::R2, Register::R2, 0, Register::R1 as u16);
        assert_eq!(vm.registers[Register::R2], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        vm.add(Register::R2, Register::R2, 0, Register::R5 as u16);
        assert_eq!(vm.registers[Register::R2], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag.
    }

    #[test]
    fn add_instruction_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // Immediate mode add positive number.
        vm.add(Register::R5, Register::R0, 1, 5);
        assert_eq!(vm.registers[Register::R5], 5);
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 
        // Register mode.
        vm.registers[Register::R4] = 65535; // 65535 in u16 is 0xFFFF which is equal to -1 in two'2 complement notation.
        // Immediate mode add negative number.
        vm.add(Register::R7, Register::R4, 1, 1);
        assert_eq!(vm.registers[Register::R7], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag.

        vm.registers[Register::R2] = 65530;
        vm.add(Register::R2, Register::R2, 1, 1);
        assert_eq!(vm.registers[Register::R2], 65531); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        assert_eq!(Ok(()), vm.load(Register::R0, 15));
        assert_eq!(vm.registers[Register::R0], 52);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(Ok(()), vm.load(Register::R1, 0b111111000));
        assert_eq!(vm.registers[Register::R1], 50000);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        // Load to check Zro flag.
        assert_eq!(Ok(()), vm.load(Register::R0, 0));
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Register::R0] = 52;
        assert_eq!(vm.memory[15], 0);
        assert_eq!(Ok(()), vm.store(Register::R0, 15));
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::PC] = 2;
        vm.registers[Register::R1] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(vm.memory[65530], 0);
        assert_eq!(Ok(()), vm.store(Register::R1, 0b111111000));
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 
    }

    #[test]
    fn jump_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        // JSR with positive offset
        vm.registers[Register::PC] = 3;
        vm.jump_register(1, 4);
        assert_eq!(vm.registers[Register::PC], 7);
        assert_eq!(vm.registers[Register::R7], 3);
        // JSR with negative offset
        // PC is equal to 7 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.jump_register(1, 0b11111111000);
        assert_eq!(vm.registers[Register::PC], 65535);
        assert_eq!(vm.registers[Register::R7], 7);
        // JSRR
        vm.registers[Register::R6] = 365;
        vm.jump_register(0, Register::R6 as u16);
        assert_eq!(vm.registers[Register::PC], 365);
        assert_eq!(vm.registers[Register::R7], 65535);
    }

    #[test]
    fn bitwise_and_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R0] = 33;
        vm.registers[Register::R5] = 5;
        vm.and(Register::R4, Register::R5, 0, Register::R0 as u16);
        assert_eq!(vm.registers[Register::R4], 1);
        assert_eq!(vm.registers[Register::R0], 33);
        assert_eq!(vm.registers[Register::R5], 5);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        vm.registers[Register::R2] = 65535;
        vm.registers[Register::R3] = 65520;
        vm.and(Register::R2, Register::R2, 0, Register::R3 as u16);
        assert_eq!(vm.registers[Register::R2], 0xFFF0);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        vm.and(Register::R6, Register::R2, 0, Register::R1 as u16);
        assert_eq!(vm.registers[Register::R6], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 
    }

    #[test]
    fn bitwise_and_immediate_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.and(Register::R5, Register::R0, 1, 5);
        assert_eq!(vm.registers[Register::R5], 0);
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 

        // 20 is 1 0100 in binary, which is equal to -12 in two's complement notation for 9 bits.
        vm.registers[Register::R5] = 5;
        vm.and(Register::R7, Register::R5, 1, 20);
        assert_eq!(vm.registers[Register::R7], 4);
        assert_eq!(vm.registers[Register::R5], 5);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag.

        // Register mode to check neg flag.
        vm.registers[Register::R2] = 65535;
        vm.and(Register::R2, Register::R2, 1, 16);
        assert_eq!(vm.registers[Register::R2], 0xFFF0); // 65531 in u16 is 0xFFFB which is equal to -5 in two'2 complement notation.
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 
    }

    #[test]
    fn loading_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.registers[Register::R1] = 7;
        assert_eq!(Ok(()), vm.load_register(Register::R0, Register::R1, 8));
        assert_eq!(vm.registers[Register::R0], 52);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::R2] = 2;
        vm.memory[65530] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        assert_eq!(
            Ok(()),
            vm.load_register(Register::R1, Register::R2, 0b111000)
        );
        assert_eq!(vm.registers[Register::R1], 50000);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        // Load to check Zro flag. (R2 is equal to 2 from previous assertion set up)
        assert_eq!(Ok(()), vm.load_register(Register::R0, Register::R2, 0));
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 
    }

    #[test]
    fn storing_from_register_from_address_in_register() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset.
        vm.registers[Register::R0] = 52;
        vm.registers[Register::R1] = 7;
        assert_eq!(vm.memory[15], 0);
        assert_eq!(Ok(()), vm.store_register(Register::R0, Register::R1, 8));
        assert_eq!(vm.memory[15], 52);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::R2] = 2;
        vm.registers[Register::R1] = 50000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 6 bits = 0b111000
        assert_eq!(vm.memory[65530], 0);
        assert_eq!(
            Ok(()),
            vm.store_register(Register::R1, Register::R2, 0b111000)
        );
        assert_eq!(vm.memory[65530], 50000);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 
    }

    #[test]
    fn bitwise_not() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R5] = 1;
        vm.not(Register::R4, Register::R5);
        assert_eq!(vm.registers[Register::R4], 0xFFFE); // 0xFFFE is -2 in two'2 complement notation.
        assert_eq!(vm.registers[Register::R5], 1);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        vm.registers[Register::R3] = 65520; //0xFFF0 
        vm.not(Register::R2, Register::R3);
        assert_eq!(vm.registers[Register::R2], 15); //0x000F 
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        vm.registers[Register::R2] = 0xFFFF;
        vm.not(Register::R6, Register::R2);
        assert_eq!(vm.registers[Register::R6], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag.
    }

    #[test]
    fn load_indirect() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.memory[15] = 52;
        vm.memory[52] = 10;
        vm.registers[Register::R1] = 7;
        assert_eq!(Ok(()), vm.load_indirect(Register::R0, 15));
        assert_eq!(vm.registers[Register::R0], 10);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC] = 2;
        vm.memory[65530] = 50000;
        vm.memory[50000] = 55555;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(Ok(()), vm.load_indirect(Register::R1, 0b111111000));
        assert_eq!(vm.registers[Register::R1], 55555);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        // Load to check Zro flag. (PC is equal to 2 from previous assertion set up and address 2 in memory stores 0 for default)
        assert_eq!(Ok(()), vm.load_indirect(Register::R0, 0));
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 
    }

    #[test]
    fn store_indirect() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Store with positive offset. (PC is equal to 0 for default)
        vm.memory[17] = 7;
        vm.registers[Register::R0] = 52;
        assert_eq!(vm.memory[15], 0);
        assert_eq!(Ok(()), vm.store_indirect(Register::R0, 17));
        assert_eq!(vm.memory[7], 52);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 

        // Store with negative offset.
        vm.registers[Register::PC] = 2;
        vm.memory[65530] = 50000;
        vm.registers[Register::R1] = 65000;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        assert_eq!(Ok(()), vm.store_indirect(Register::R1, 0b111111000));
        assert_eq!(vm.memory[50000], 65000);
        assert_eq!(vm.registers[Register::COND], 0); // Check flags. 
    }

    #[test]
    fn load_effective_address() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();

        // Load with positive offset. (PC is equal to 0 for default)
        vm.load_effective_address(Register::R0, 15);
        assert_eq!(vm.registers[Register::R0], 15);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 

        // Load with negative offset.
        vm.registers[Register::PC] = 2;
        // PC is equal to 2 so the negative jump should be equal to -8 in 9 bits = 0b111111000
        vm.load_effective_address(Register::R1, 0b111111000);
        assert_eq!(vm.registers[Register::R1], 65530);
        assert_eq!(vm.registers[Register::COND], 4); // Check Neg flag. 

        // Load to check Zro flag. (PC is equal to 2 from previous assertion, pc_offset is equal to -2 in two'2 complement notation.)
        vm.load_effective_address(Register::R0, 0xFFFE);
        assert_eq!(vm.registers[Register::R0], 0);
        assert_eq!(vm.registers[Register::COND], 2); // Check Zro flag. 
    }

    #[test]
    fn executing_add_instruction_register_mode() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.registers[Register::R1] = 32;
        vm.registers[Register::R2] = 5;
        let instruction = 0b0001000001000010; //ADD r0, r1, r2
        let _ = vm.execute_instruction(instruction);
        assert_eq!(vm.registers[Register::R0], 37);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 
    }

    #[test]
    fn reading_image_file_with_trap_halt() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.origin = 0x00;
        // vector has two first elements as address to load image, and two last elements are instruction TRAP HALT
        let image_file = vec![0x00, 0x00, 0xF0, 0x25];
        assert_eq!(Ok(()), read_image_file(&mut vm, image_file));
        assert_eq!(Ok(()), vm.run());
        assert!(!vm.running);
    }

    #[test]
    fn reding_image_file_with_add_and_trap() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.origin = 0x00;
        // vector has two first elements as address to load image, then the two following elements are instruction ADD r0, r1, r2
        // and two last elements are instruction TRAP HALT
        let image_file = vec![0x00, 0x00, 0b00010000, 0b01000010, 0xF0, 0x25];
        vm.registers[Register::R1] = 32;
        vm.registers[Register::R2] = 5;

        assert_eq!(Ok(()), read_image_file(&mut vm, image_file));
        assert_eq!(Ok(()), vm.run());
        assert_eq!(vm.registers[Register::R0], 37);
        assert_eq!(vm.registers[Register::COND], 1); // Check Pos flag. 
        assert!(!vm.running);
    }

    #[test]
    fn reding_empty_image_file_throws_eror() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.origin = 0x00;
        let image_file = vec![];

        assert_eq!(
            Err(VMError::FailedToLoadImage(String::from(
                "Image size invalid"
            ))),
            read_image_file(&mut vm, image_file)
        );
    }

    #[test]
    fn reding_odd_size_image_file_throws_eror() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.origin = 0x00;
        let image_file = vec![0x00, 0x00, 0b00010000];

        assert_eq!(
            Err(VMError::FailedToLoadImage(String::from(
                "Image size invalid"
            ))),
            read_image_file(&mut vm, image_file)
        );
    }

    #[test]
    fn executin_invalid_trap_code_throws_error() {
        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.origin = 0x00;
        // vector has two first elements as address to load image, and two last elements are instruction TRAP with invalid trap code
        let image_file = vec![0x00, 0x00, 0xF0, 0xFF];
        assert_eq!(Ok(()), read_image_file(&mut vm, image_file));
        assert_eq!(
            Err(VMError::InvalidTrapCode(HardwareError::InvalidTrapCode(
                0xFF
            ))),
            vm.run()
        );
    }
}
