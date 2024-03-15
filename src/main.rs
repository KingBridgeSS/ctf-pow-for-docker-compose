mod handler;
mod pow;

use clap::Parser;
use env_logger;
use std::env;
use std::error::Error;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// The directory containing the docker-compose.tpl file (Due to limitations of third-party libraries, refrain from using relative paths such as "../example" to represent parent directories as --compose-dir.)
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

#[derive(thiserror::Error, Debug)]
pub enum DockerEnvError {
    #[error("docker not installed")]
    DockerNotInstalled,
    #[error("docker compose/docker-compose not installed")]
    DockerComposeNotInstalled,
}

#[tokio::main]
async fn main() {
    match docker_compose_emmbed() {
        Ok(support) => {
            if env::var("RUST_LOG").is_err() {
                env::set_var("RUST_LOG", "info")
            }
            env_logger::init();
            let args = Args::parse();
            // folder check
            if let Ok(metadata) = fs::metadata(args.compose_dir.clone()) {
                if metadata.is_dir() {
                    let handler = handler::Handler {
                        support_emmbed_cmd: support,
                        port: args.port,
                        compose_dir: args.compose_dir,
                        pow_difficulty: args.difficulty,
                        pow_timeout: args.pow_timeout,
                        service_timeout: args.service_timeout,
                    };
                    Arc::new(handler).handle().await.unwrap_or(());
                } else {
                    eprintln!("compose-dir not exist")
                }
            } else {
                eprintln!("compose-dir not exist")
            }
        }
        Err(e) => eprintln!("{}", e.to_string()),
    }
}

fn docker_compose_emmbed() -> Result<bool, Box<dyn Error>> {
    let mut emmbed_cmd = Command::new("docker");
    emmbed_cmd.args(&["compose"]);
    match emmbed_cmd.stderr(Stdio::piped()).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Define and run multi-container applications with Docker") {
                Ok(true)
            } else {
                // Docker < 1.27
                let mut individual_cmd = Command::new("docker-compose");
                match individual_cmd.output() {
                    Ok(_) => Ok(false),
                    Err(_) => Err(Box::new(DockerEnvError::DockerComposeNotInstalled)),
                }
            }
        }
        Err(_) => Err(Box::new(DockerEnvError::DockerNotInstalled)),
    }
}
