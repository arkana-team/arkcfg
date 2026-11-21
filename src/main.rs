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
            PowerAction::Restart { no_confirm } => power::cmd_restart(no_confirm)?,
            PowerAction::Poweroff { no_confirm } => power::cmd_poweroff(no_confirm)?,
        },
    }

    Ok(())
}

fn main() {
    // `PanicHookInfo`'s Display already starts with "panicked at <loc>:", so
    // formatting it as "Unexpected internal error: {}" duplicated that prefix and
    // hid the location. Print it as-is instead.
    //
    // Replacing the default hook also disables its backtrace printing, so print
    // a backtrace ourselves when RUST_BACKTRACE asks for one.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{}", info);
        if std::env::var("RUST_BACKTRACE").is_ok_and(|v| v != "0") {
            eprintln!("{}", std::backtrace::Backtrace::force_capture());
        }
    }));

    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}
