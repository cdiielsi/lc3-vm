use lc3_vm::*;
use termios::*;

mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let mut term = Termios::from_fd(0).unwrap();
    vm.disable_input_buffering(&mut term);

    vm.registers[Register::PC as usize] = vm.origin;
    vm.registers[Register::COND as usize] = 1;
    vm.read_image("2048.obj");
    vm.execute();
    vm.restore_input_buffering(&mut term);
    println!("{}", vm.memory[0]);
    /*
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    vm.origin = 0x00;
    // vector has two first elements as address to load image, and two last elements are instruction ADD r0, r1, r2
    let image_file = vec![0x00,0x00, 0b00010000,0b01000010,0xF0,0x20, 0xF0,0x25];
    vm.registers[Register::R1 as usize] = 32;
    vm.registers[Register::R2 as usize] = 5;
    vm.memory[37] = 'f' as u16;
    vm.read_image_file(image_file);
    vm.execute();
    assert_eq!(vm.registers[Register::R0 as usize], 'f' as u16);
    assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag.
    assert_eq!(vm.running,0);*/
}
