use clap::Parser;
use std::process;

mod cli;
mod display;
mod errors;
mod power;
mod sysinfo;

use cli::{Cli, Commands, DisplayAction, PowerAction};

fn run() -> errors::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Display { action } => match action {
            DisplayAction::List => display::cmd_list()?,
            DisplayAction::Current => display::cmd_current()?,
            DisplayAction::Set { resolution, output } => {
                display::cmd_set(&resolution, output.as_deref())?
            }
        },
        Commands::Sysinfo => sysinfo::cmd_sysinfo()?,
        Commands::Power { action } => match action {
            PowerAction::Restart => power::cmd_restart()?,
            PowerAction::Poweroff => power::cmd_poweroff()?,
        },
    }

    Ok(())
}

fn main() {
    // Match arkpkg's clean panic hook
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Unexpected internal error: {}", info);
    }));

    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}
