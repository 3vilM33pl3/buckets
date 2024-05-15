mod commands;
mod data;
mod utils;

use clap::{arg, Command};
use std::io;
use std::process::exit;
use log::{debug, error, info};

fn cli() -> Command {
    Command::new("bucket")
        .version("1.0")
        .author("3vilM33pl3 <olivier@robotmotel.com>")
        .about("")
        .subcommand(Command::new("version").about("Displays the version of the bucket tool"))
        .subcommand(
            Command::new("init")
                .about("Initialises bucket repository")
                .arg(arg!(<NAME> "Name of the repository"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("create")
                .about("Creates a new bucket")
                .arg(arg!(<NAME> "Name of the bucket"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("commit")
                .about("Commits a bucket")
                .arg(
                    arg!(-m --message <MESSAGE> "The commit message")
                        .required(false)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                )
        )
        .subcommand(
            Command::new("status")
                .about("Displays the status of the bucket")
        )
}

fn main() {
    env_logger::init();

    let matches = cli().get_matches();

    match matches.subcommand() {
        None => {}
        Some(("version", _)) => commands::version::execute(&mut io::stdout()).unwrap(),
        Some(("init", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) = commands::init::execute(&arg.to_string()) {
                eprintln!("Can not create repository: {}", e);
                exit(1)
            } else {
                info!("Initialised bucket repository");
                exit(0)
            }
        }
        Some(("create", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) = commands::create::execute(&arg.to_string()) {
                eprintln!("Can not create bucket: {}", e);
                exit(1)
            } else {
                info!("Created bucket");
                exit(0)
            }
        }
        Some(("commit", sub_matches)) => {
            let message = match sub_matches.get_one::<String>("message") {
                Some(message) => message.to_string(),
                None => "".to_string(),
            };

            debug!("message: {}", message);

            if let Err(e) = commands::commit::execute(&message) {
                error!("Can not commit bucket: {}", e);
                exit(1)
            } else {
                info!("Committed bucket");
                exit(0)
            }
        }
        Some(("status", _)) => {
            match commands::status::execute() {
                Ok(_) => {
                    exit(0)
                }
                Err(e) => {
                    error!("Can not get status of the bucket: {}", e);
                    exit(1)
                }
            }
        }

        _ => commands::version::execute(&mut io::stdout()).unwrap(),
    }
}
