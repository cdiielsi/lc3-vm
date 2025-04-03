use lc3_vm::{LC3VirtualMachine, Registers};

mod lc3_vm;

fn main() {
    let vm: LC3VirtualMachine = LC3VirtualMachine::new();
    println!("Hello, world!");
}
