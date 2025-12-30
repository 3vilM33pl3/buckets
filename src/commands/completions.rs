use crate::args::{CliArguments, CompletionShell, CompletionsCommand};
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;
use std::path::PathBuf;
use log::info;
use dirs;

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

        if self.args.install {
            let path = self.get_install_path(shell)?;
             if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::File::create(&path)?;
            generate(shell, &mut cmd, bin_name, &mut file);
            info!("Completion script installed to {:?}", path);
            eprintln!("Completion script installed to {:?}", path); 
        } else if let Some(path) = &self.args.output {
            let mut file = std::fs::File::create(path)?;
            generate(shell, &mut cmd, bin_name, &mut file);
        } else {
            let mut stdout = io::stdout();
            generate(shell, &mut cmd, bin_name, &mut stdout);
        }
        Ok(())
    }
}

impl Completions {
    fn get_install_path(&self, shell: Shell) -> Result<PathBuf, BucketError> {
        match shell {
             Shell::Bash => {
                 let mut path = dirs::data_local_dir().ok_or_else(|| BucketError::from("Could not determine local data directory"))?;
                 path.push("bash-completion");
                 path.push("completions");
                 path.push("buckets");
                 Ok(path)
             }
             Shell::Zsh => {
                 let mut path = dirs::data_local_dir().ok_or_else(|| BucketError::from("Could not determine local data directory"))?;
                 path.push("zsh");
                 path.push("completions");
                 path.push("_buckets");
                 Ok(path)
             }
             Shell::Fish => {
                 let mut path = dirs::config_dir().ok_or_else(|| BucketError::from("Could not determine config directory"))?;
                 path.push("fish");
                 path.push("completions");
                 path.push("buckets.fish");
                 Ok(path)
             }
             _ => Err(BucketError::from("Automatic installation is not supported for this shell. Please use --output to install manually."))
        }
    }
}

