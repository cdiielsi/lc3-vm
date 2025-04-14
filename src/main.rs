use clap::Parser;
use lc3_vm::{
    LC3VirtualMachine, VMError, disable_input_buffering, read_image, restore_input_buffering,
};
use termios::Termios;
pub mod hardware;
mod lc3_vm;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the image to run on the vm
    #[arg(short, long)]
    path: String,
}

fn main() -> Result<(), VMError> {
    let args = Args::parse();

    let mut term = Termios::from_fd(0).unwrap();
    disable_input_buffering(&mut term)?;

    let mut vm: LC3VirtualMachine = LC3VirtualMachine::new();
    vm.set_pc_with_origin();
    vm.turn_pos_flag_on();
    read_image(&mut vm, &args.path)?;
    vm.run()?;

    restore_input_buffering(&mut term)?;
    Ok(())
}
