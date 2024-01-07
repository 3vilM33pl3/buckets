mod commands;
mod utils;

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
        Some(("version", _)) => commands::version::execute(),
        Some(("init", _)) => {
            if let Err(e) = commands::init::execute() {
                println!("Can not create repository: {}", e);
            } else {
                println!("Initialised bucket repository");
            }
        }
        _ => commands::version::execute(),
    }
}
