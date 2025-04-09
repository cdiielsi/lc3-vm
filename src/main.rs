use lc3_vm::*;
mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let instruction = 0b0001000001000010; //ADD r0, r1, r2
    vm.execute_instruction(instruction);

    let routine = TrapCode::TrapHalt;

    match routine {
        TrapCode::TrapGetc => {
            println!("Trap getc executing next, write a character and press enter...");
            let _ = vm.trap_getc(Register::R6);
            println!(
                "The character written was: {}",
                char::from_u32(vm.registers[Register::R6 as usize] as u32).unwrap()
            );
            assert_ne!(vm.registers[Register::R6 as usize], 0);
            println!();
        }
        TrapCode::TrapIn => {
            println!("Trap in executing next...");
            let _ = vm.trap_in(Register::R4);
            assert_ne!(vm.registers[Register::R4 as usize], 0);
        }
        TrapCode::TrapPuts => {
            println!("Trap puts executing next...");
            vm.memory[15] = 'a' as u16;
            vm.memory[16] = 'b' as u16;
            vm.memory[17] = 'c' as u16;
            vm.memory[18] = 0;
            vm.registers[Register::R0 as usize] = 15;
            let _ = vm.trap_puts(Register::R0);
            println!();
        }
        TrapCode::TrapOut => {
            println!("Trap out executing next...");
            vm.registers[Register::R2 as usize] = 'L' as u16;
            let _ = vm.trap_out(Register::R2);
            println!();
        }
        TrapCode::TrapPutsp => {
            println!("Trap putsp executing next...");
            vm.memory[15] = 0x6f68; //"oh"
            vm.memory[16] = 0x616c; //"al"
            vm.memory[17] = 0;
            vm.registers[Register::R0 as usize] = 15;
            let _ = vm.trap_putsp(Register::R0);
            println!();
        }
        TrapCode::TrapHalt => {
            println!("Trap halt executing next...");
            vm.trap_halt();
            assert_eq!(vm.running, 0);
        }
    }
}
