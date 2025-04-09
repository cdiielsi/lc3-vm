use lc3_vm::*;
mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    vm.registers[Register::PC as usize] = vm.origin;
    vm.registers[Register::COND as usize] = 1;
    vm.read_image("2048.obj");
    vm.execute();
}
