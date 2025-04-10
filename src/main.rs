use lc3_vm::*;
use termios::*;
pub mod hardware;
mod lc3_vm;

fn main() -> Result<(), VMError> {
    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    let mut term = Termios::from_fd(0).unwrap();
    disable_input_buffering(&mut term)?;
    vm.set_pc_with_origin();
    vm.turn_pos_flag_on();
    read_image(&mut vm, "2048.obj")?;
    vm.execute()?;
    restore_input_buffering(&mut term)?;
    Ok(())
}
