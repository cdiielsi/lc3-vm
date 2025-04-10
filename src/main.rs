use lc3_vm::*;
use termios::*;

mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let mut term = Termios::from_fd(0).unwrap();
    vm.disable_input_buffering(&mut term);

    vm.registers[Register::PC as usize] = vm.origin;
    vm.registers[Register::COND as usize] = 1;
    vm.read_image("rogue.obj");
    vm.execute();
    vm.restore_input_buffering(&mut term);
    println!("{}", vm.memory[0]);
}
