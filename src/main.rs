mod commands;
mod utils;

use clap::{arg, Command};
use std::ffi::{OsStr, OsString};
use std::io;

fn cli() -> Command {
    Command::new("bucket")
        .version("1.0")
        .author("3vilM33pl3 <olivier@robotmotel.com>")
        .about("")
        .subcommand(Command::new("version").about("Displays the version of the bucket tool"))
        .subcommand(
            Command::new("init").about("Initialises bucket repository")
                .about("Initialises bucket repository")
                .arg(arg!(<NAME> "Name of the repository"))
                .arg_required_else_help(true)
        )
        .subcommand(
            Command::new("create").about("Creates a new bucket")
                .about("Creates a new bucket")
                .arg(arg!(<NAME> "Name of the bucket"))
                .arg_required_else_help(true)
        )
        .subcommand(Command::new("commit").about("Commits a bucket"))
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        None => {}
        Some(("version", _)) => commands::version::execute(&mut io::stdout()).unwrap(),
        Some(("init", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) = commands::init::execute(&arg.to_string()) {
                println!("Can not create repository: {}", e);
            } else {
                println!("Initialised bucket repository");
            }
        }
        Some(("create", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) =
                commands::create::execute(&arg.to_string())
            {
                println!("Can not create bucket: {}", e);
            } else {
                println!("Created bucket");
            }
        }
        Some(("commit", sub_m)) => {
            let ext_args: Vec<&OsStr> = sub_m
                .get_many::<OsString>("")
                .unwrap()
                .map(|s| s.as_os_str())
                .collect();

            if ext_args.len() != 0 {
                println!("To many arguments provided");
                return;
            }

            if let Err(e) = commands::commit::execute() {
                println!("Can not commit bucket: {}", e);
            } else {
                println!("Committed bucket");
            }
        }
        _ => commands::version::execute(&mut io::stdout()).unwrap(),
    }
}
