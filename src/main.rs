use clap::Parser;
use http_server::{init, run, Args};
use log::error;

fn main() {
    let args = Args::parse();

    match init(&args) {
        Ok(configs) => {
            if let Err(e) = run(configs) {
                error!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Initialization error: {}", e);
            std::process::exit(1);
        }
    }
}
