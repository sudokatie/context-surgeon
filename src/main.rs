mod cli;
mod config;
mod models;
mod segmenter;
mod tokenizer;
mod analysis;
mod compression;
mod output;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("input error: {0}")]
    Input(String),
    #[error("argument error: {0}")]
    Argument(String),
    #[error("budget error: {0}")]
    Budget(String),
    #[error("model error: {0}")]
    Model(#[from] models::ModelError),
    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("compression error: {0}")]
    Compression(#[from] compression::CompressionError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Input(_) => 1,
            Error::Argument(_) => 2,
            Error::Budget(_) | Error::Model(_) => 3,
            Error::Config(_) | Error::Compression(_) | Error::Io(_) => 4,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(e.exit_code());
    }
}

fn run() -> Result<(), Error> {
    let args = cli::parse();
    cli::validate(&args).map_err(|e| Error::Argument(e.to_string()))?;
    
    println!("Budget: {:?}", args.budget);
    println!("Model: {:?}", args.model);
    Ok(())
}
