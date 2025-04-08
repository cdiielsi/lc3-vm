use lc3_vm::*;
mod lc3_vm;

fn main() {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let instruction = 0b0001000001000010; //ADD r0, r1, r2
    vm.execute_instruction(instruction);
}
