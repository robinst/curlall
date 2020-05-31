use std::process;
use structopt::StructOpt;

use curlall::{run, Opt, NAME};

fn main() {
    let opt = Opt::from_args();
    match run(opt) {
        Err(err) => {
            eprintln!("{}: {}", NAME, err);
            process::exit(1);
        }
        Ok(_) => {
            process::exit(0);
        }
    }
}
