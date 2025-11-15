pub mod bash;
pub mod fish;
pub mod zsh;

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;

pub fn print(shell: Shell) {
    let mut cmd = Cli::command().allow_external_subcommands(false);
    let bin_name = cmd.get_name().to_string();
    match shell {
        Shell::Bash => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", bash::patch(script));
        }
        Shell::Zsh => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", zsh::patch(script));
        }
        Shell::Fish => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", fish::patch(script));
        }
        other => {
            generate(other, &mut cmd, &bin_name, &mut io::stdout());
        }
    }
}
