use lc3_vm::*;
use std::env;
use termios::*;
pub mod hardware;
mod lc3_vm;

fn main() -> Result<(), VMError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Run make with an image target");
        Ok(())
    } else {
        let image_file_path = &args[1];

        let mut term = Termios::from_fd(0).unwrap();
        disable_input_buffering(&mut term)?;

        let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
        vm.set_pc_with_origin();
        vm.turn_pos_flag_on();
        read_image(&mut vm, image_file_path)?;
        vm.run()?;

        restore_input_buffering(&mut term)?;
        Ok(())
    }
}
