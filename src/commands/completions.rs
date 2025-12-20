use crate::args::{CliArguments, CompletionShell, CompletionsCommand};
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub struct Completions {
    args: CompletionsCommand,
}

impl BucketCommand for Completions {
    type Args = CompletionsCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let shell = match self.args.shell {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::Powershell => Shell::PowerShell,
            CompletionShell::Elvish => Shell::Elvish,
        };

        let mut cmd = CliArguments::command();
        let bin_name = cmd.get_name().to_string();

        let mut stdout = io::stdout();
        generate(shell, &mut cmd, bin_name, &mut stdout);
        Ok(())
    }
}

