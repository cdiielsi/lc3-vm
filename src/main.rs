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
    let image_file = vec![0x00,0x00, 0xE0, 0x04,0xF0,0x22, 0xF0,0x25,0x00,0x00,0x00,0x6f,0x00,0x68,0x00,0x61,0x00, 0x6c,0x00, 0x21,0x00, 0x00];
    vm.read_image_file(image_file);
    vm.execute();
    println!();
    println!("{:X?}",&vm.memory[0..20]);
    assert_eq!(vm.registers[Register::COND as usize], 1); // Check Pos flag.
    assert_eq!(vm.running,0);  */
}
