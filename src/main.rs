mod commands;
mod utils;

use std::ffi::{OsStr, OsString};
use std::io;
use clap::Command;

fn main() {
    let matches = Command::new("bucket")
        .version("1.0")
        .author("3vilM33pl3 <olivier@robotmotel.com>")
        .about("")
        .subcommand(Command::new("version").about("Displays the version of the bucket tool"))
        .subcommand(Command::new("init").about("Initialises bucket repository"))
        .get_matches();

    match matches.subcommand() {
        None => {}
        Some(("version", _)) => commands::version::execute(&mut io::stdout()).unwrap(),
        Some(("init", _)) => {
            if let Err(e) = commands::init::execute() {
                println!("Can not create repository: {}", e);
            } else {
                println!("Initialised bucket repository");
            }
        },
        Some(("create", sub_m)) => {
            let ext_args: Vec<&OsStr> = sub_m.get_many::<OsString>("")
                .unwrap().map(|s| s.as_os_str()).collect();

            if ext_args.len() != 1 {
                println!("Please provide a name for the bucket");
                return;
            }

            if let Err(e) = commands::create::execute(&ext_args.first().unwrap().to_string_lossy().into_owned()){
                println!("Can not create bucket: {}", e);
            } else {
                println!("Created bucket");
            }
        },
        Some(("commit", sub_m)) => {
            let ext_args: Vec<&OsStr> = sub_m.get_many::<OsString>("")
                .unwrap().map(|s| s.as_os_str()).collect();

            if ext_args.len() != 0 {
                println!("To many arguments provided");
                return;
            }

            if let Err(e) = commands::commit::execute(){
                println!("Can not commit bucket: {}", e);
            } else {
                println!("Committed bucket");
            }
        },
        _ => commands::version::execute(&mut io::stdout()).unwrap(),
    }
}
