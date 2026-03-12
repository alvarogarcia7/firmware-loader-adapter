mod auth;
mod protocol;
mod serial;
mod file_ops;

use clap::Parser;
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(name = "secure-serial-transfer")]
#[command(about = "Secure file transfer over serial connection", long_about = None)]
struct Args {
    #[arg(short, long)]
    port: String,

    #[arg(short, long, default_value = "115200")]
    baud_rate: u32,

    #[arg(short, long)]
    send: Option<String>,

    #[arg(short, long)]
    receive: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Secure Serial Transfer");
    println!("Port: {}", args.port);
    println!("Baud Rate: {}", args.baud_rate);

    if let Some(file_path) = args.send {
        println!("Mode: Send file {}", file_path);
    } else if let Some(file_path) = args.receive {
        println!("Mode: Receive file to {}", file_path);
    } else {
        println!("No operation specified. Use --send or --receive");
    }

    Ok(())
}
