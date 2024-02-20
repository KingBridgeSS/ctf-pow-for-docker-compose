mod handler;
mod pow;

use clap::Parser;
use env_logger;
use std::env;
use std::process::Command;
use std::sync::Arc;
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// The directory containing the docker-compose.tpl file
    #[arg(long)]
    compose_dir: String,
    /// The port to listen on
    #[arg(long, default_value_t = String::from("1337"))]
    port: String,
    /// The difficulty of the proof of work
    #[arg(long, default_value_t = 6)]
    difficulty: usize,
    /// The timeout for the proof of work (seconds)
    #[arg(long, default_value_t = 30)]
    pow_timeout: u64,
    /// The timeout for the service (seconds)
    #[arg(long, default_value_t = 120)]
    service_timeout: u64,
}
#[tokio::main]
async fn main() {
    match Command::new("docker").output() {
        Ok(_) => {
            if env::var("RUST_LOG").is_err() {
                env::set_var("RUST_LOG", "info")
            }
            env_logger::init();
            let args = Args::parse();
            let handler = handler::Handler {
                port: args.port,
                compose_dir: args.compose_dir,
                pow_difficulty: args.difficulty,
                pow_timeout: args.pow_timeout,
                service_timeout: args.service_timeout,
            };
            Arc::new(handler).handle().await.unwrap_or(());
        }
        Err(_) => {
            eprintln!("Docker is not installed or not in PATH");
            std::process::exit(1);
        }
    }
}
