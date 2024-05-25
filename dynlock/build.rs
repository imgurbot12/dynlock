use std::io::Result;

use clap_builder::CommandFactory;
use dynlock_lib::Cli;

fn main() -> Result<()> {
    let man = clap_mangen::Man::new(Cli::command());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write("../dynlock.1", buffer)?;
    Ok(())
}
