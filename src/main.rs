use lc3_vm::*;
mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let instruction = 0b0001000001000010; //ADD r0, r1, r2
    let _ = vm.execute_instruction(instruction);

    println!("Trap in executing next...");
    let _ = vm.trap_in();
    assert_ne!(vm.registers[Register::R0 as usize], 0);
    println!();
    println!();
    println!("Trap puts executing next...");
    vm.memory[15] = 'a' as u16;
    vm.memory[16] = 'b' as u16;
    vm.memory[17] = 'c' as u16;
    vm.memory[18] = 0;
    vm.registers[Register::R0 as usize] = 15;
    let _ = vm.trap_puts();
    println!();
    println!();
    println!("Trap out executing next...");
    vm.registers[Register::R0 as usize] = 'L' as u16;
    let _ = vm.trap_out();
    println!();
    println!();
    println!("Trap putsp executing next...");
    vm.memory[15] = 0x6f68; //"oh"
    vm.memory[16] = 0x616c; //"al"
    vm.memory[17] = 0x0021; //"NUL!"
    vm.registers[Register::R0 as usize] = 15;
    let _ = vm.trap_putsp();
    println!();
    println!();
    println!("Trap getc executing next, write a character...");
    let _ = vm.trap_getc();
    println!(
        "The character written was: {}",
        char::from_u32(vm.registers[Register::R0 as usize] as u32).unwrap()
    );
    assert_ne!(vm.registers[Register::R0 as usize], 0);
    println!();

    println!("Trap halt executing next...");
    vm.trap_halt();
    assert_eq!(vm.running, 0);
}
