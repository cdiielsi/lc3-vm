use std::ops::{Index, IndexMut};
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
    pub fn from_u16(value: u16) -> Result<Self, RegisterError> {
        match value {
            0 => Ok(Self::R0),
            1 => Ok(Self::R1),
            2 => Ok(Self::R2),
            3 => Ok(Self::R3),
            4 => Ok(Self::R4),
            5 => Ok(Self::R5),
            6 => Ok(Self::R6),
            7 => Ok(Self::R7),
            8 => Ok(Self::PC),
            9 => Ok(Self::COND),
            _ => {
                Err(RegisterError::ErrorDecodingRegister) //Invalid Register
            }
        }
    }
}

pub enum RegisterError {
    ErrorDecodingRegister,
}

impl<T> Index<Register> for [T] {
    type Output = T;

    fn index(&self, register: Register) -> &Self::Output {
        match register {
            Register::R0 => &self[0],
            Register::R1 => &self[1],
            Register::R2 => &self[2],
            Register::R3 => &self[3],
            Register::R4 => &self[4],
            Register::R5 => &self[5],
            Register::R6 => &self[6],
            Register::R7 => &self[7],
            Register::PC => &self[8],
            Register::COND => &self[9],
        }
    }
}

impl<T> IndexMut<Register> for [T] {
    fn index_mut(&mut self, register: Register) -> &mut Self::Output {
        match register {
            Register::R0 => &mut self[0],
            Register::R1 => &mut self[1],
            Register::R2 => &mut self[2],
            Register::R3 => &mut self[3],
            Register::R4 => &mut self[4],
            Register::R5 => &mut self[5],
            Register::R6 => &mut self[6],
            Register::R7 => &mut self[7],
            Register::PC => &mut self[8],
            Register::COND => &mut self[9],
        }
    }
}

pub enum Flags {
    Pos,
    Zro,
    Neg,
    PosZro,
    NotZro,
    PosNeg,
    PosZroNeg,
    NoFlag,
}

impl Flags {
    pub fn from_u16(value: u16) -> Result<Self, FlagError> {
        match value {
            0 => Ok(Self::NoFlag),
            1 => Ok(Self::Pos),
            2 => Ok(Self::Zro),
            3 => Ok(Self::PosZro),
            4 => Ok(Self::Neg),
            5 => Ok(Self::PosNeg),
            6 => Ok(Self::NotZro),
            7 => Ok(Self::PosZroNeg),
            _ => {
                Err(FlagError::ErrorDecodingFlag) //Invalid Flag
            }
        }
    }
}

pub enum FlagError {
    ErrorDecodingFlag,
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
    OpNOT,  /* bitwise not */
    OpLDI,  /* load indirect */
    OpSTI,  /* store indirect */
    OpJMP,  /* jump */
    OpLEA,  /* load effective address */
    OpTRAP, /* execute trap */
}

impl Instruction {
    pub fn from_u16(value: u16) -> Result<Self, InstructionError> {
        match value {
            0 => Ok(Self::OpBR),    /* branch */
            1 => Ok(Self::OpADD),   /* add  */
            2 => Ok(Self::OpLD),    /* load */
            3 => Ok(Self::OpST),    /* store */
            4 => Ok(Self::OpJSR),   /* jump register */
            5 => Ok(Self::OpAND),   /* bitwise and */
            6 => Ok(Self::OpLDR),   /* load register */
            7 => Ok(Self::OpSTR),   /* store register */
            9 => Ok(Self::OpNOT),   /* bitwise not */
            10 => Ok(Self::OpLDI),  /* load indirect */
            11 => Ok(Self::OpSTI),  /* store indirect */
            12 => Ok(Self::OpJMP),  /* jump */
            14 => Ok(Self::OpLEA),  /* load effective address */
            15 => Ok(Self::OpTRAP), /* execute trap */
            _ => Err(InstructionError::InvalidInstruction),
        }
    }
}

pub enum InstructionError {
    InvalidInstruction,
}

pub struct DecodedInstruction {
    pub op_code: u16,
    pub dst: Register,
    pub src: Register,
    pub alu_operand2: u16, //It can be either an imm of 5 bits or a register number
    pub imm6: u16,
    pub imm9: u16,
    pub imm11: u16,
    pub base_for_jump: u16,
    pub mode_alu: u16,
    pub flags: u16,
    pub mode_jump: u16,
    pub trapvect8: u16,
}

impl DecodedInstruction {
    pub fn decode_instruction(
        instrucction_16: u16,
    ) -> Result<DecodedInstruction, InstructionError> {
        Ok(Self {
            op_code: instrucction_16 >> 12,
            dst: Register::from_u16((instrucction_16 >> 9) & 0x7)
                .map_err(|_| InstructionError::InvalidInstruction)?,
            src: Register::from_u16((instrucction_16 >> 6) & 0x7)
                .map_err(|_| InstructionError::InvalidInstruction)?,
            alu_operand2: instrucction_16 & 0x1F,
            imm6: instrucction_16 & 0x3F,
            imm9: instrucction_16 & 0x1FF,
            imm11: instrucction_16 & 0x7FF,
            base_for_jump: (instrucction_16 >> 6) & 0x7,
            mode_alu: (instrucction_16 >> 5) & 0x1,
            flags: (instrucction_16 >> 9) & 0x7,
            mode_jump: (instrucction_16 >> 11) & 0x1,
            trapvect8: instrucction_16 & 0xFF,
        })
    }
}

pub enum TrapCode {
    Getc = 0x20,  /* get character from keyboard, not echoed onto the terminal */
    Out = 0x21,   /* output a character */
    Puts = 0x22,  /* output a word string */
    In = 0x23,    /* get character from keyboard, echoed onto the terminal */
    Putsp = 0x24, /* output a byte string */
    Halt = 0x25,  /* halt the program */
}

impl TrapCode {
    pub fn from_u16(value: u16) -> Result<Self, InvalidTrapCode> {
        match value {
            0x20 => Ok(Self::Getc),
            0x21 => Ok(Self::Out),
            0x22 => Ok(Self::Puts),
            0x23 => Ok(Self::In),
            0x24 => Ok(Self::Putsp),
            0x25 => Ok(Self::Halt),
            _ => Err(InvalidTrapCode::InvalidICode),
        }
    }
}

pub enum InvalidTrapCode {
    InvalidICode,
}

pub enum MemoryMappedRegisters {
    MrKBSR = 0xFE00, /* keyboard status */
    MrKBDR = 0xFE02, /* keyboard data */
}
