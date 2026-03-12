mod auth;
mod protocol;
mod serial;
mod file_ops;
mod session;

use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use session::{Session, SessionManager, get_default_session_path, get_default_credentials_path};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "secure-serial-transfer")]
#[command(about = "Secure file transfer over serial connection", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Login and create a new session")]
    Login {
        #[arg(short, long, help = "Username for authentication")]
        username: String,

        #[arg(short, long, help = "Serial port (e.g., /dev/ttyUSB0 or COM3)")]
        port: String,

        #[arg(short, long, default_value = "115200", help = "Baud rate for serial connection")]
        baud_rate: u32,

        #[arg(long, help = "Register a new user instead of logging in")]
        register: bool,
    },

    #[command(about = "List files in a directory (requires active session)")]
    ListFiles {
        #[arg(help = "Directory path to list files from")]
        directory: Option<PathBuf>,
    },

    #[command(about = "Transfer a file over serial connection (requires active session)")]
    Transfer {
        #[arg(short, long, help = "File path to send")]
        send: Option<PathBuf>,

        #[arg(short, long, help = "File path to receive")]
        receive: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login { username, port, baud_rate, register } => {
            handle_login(username, port, baud_rate, register).await?;
        }
        Commands::ListFiles { directory } => {
            handle_list_files(directory).await?;
        }
        Commands::Transfer { send, receive } => {
            handle_transfer(send, receive).await?;
        }
    }

    Ok(())
}

async fn handle_login(username: String, port: String, baud_rate: u32, register: bool) -> Result<()> {
    let credentials_path = get_default_credentials_path();
    let session_manager = SessionManager::new(get_default_session_path());

    if register {
        if credentials_path.exists() {
            print!("Credentials file already exists. Overwrite? (y/N): ");
            io::stdout().flush()?;
            
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("Registration cancelled.");
                return Ok(());
            }
        }

        let password = rpassword::prompt_password("Enter password: ")
            .context("Failed to read password")?;
        let password_confirm = rpassword::prompt_password("Confirm password: ")
            .context("Failed to read password confirmation")?;

        if password != password_confirm {
            anyhow::bail!("Passwords do not match");
        }

        if password.is_empty() {
            anyhow::bail!("Password cannot be empty");
        }

        let credentials = auth::Credentials::new(username.clone(), &password)
            .context("Failed to create credentials")?;
        
        auth::store_credentials(&credentials, &password, &credentials_path)
            .context("Failed to store credentials")?;

        println!("User '{}' registered successfully!", username);

        let session = Session::new(username.clone(), port.clone(), baud_rate);
        session_manager.save_session(&session)
            .context("Failed to save session")?;

        println!("Session created for user '{}' on port {} at {} baud", username, port, baud_rate);
    } else {
        if !credentials_path.exists() {
            anyhow::bail!("No credentials found. Please register first using --register flag.");
        }

        let password = rpassword::prompt_password("Password: ")
            .context("Failed to read password")?;

        let is_valid = auth::verify_credentials(&username, &password, &credentials_path)
            .context("Failed to verify credentials")?;

        if !is_valid {
            anyhow::bail!("Invalid username or password");
        }

        let session = Session::new(username.clone(), port.clone(), baud_rate);
        session_manager.save_session(&session)
            .context("Failed to save session")?;

        println!("Login successful!");
        println!("Session created for user '{}' on port {} at {} baud", username, port, baud_rate);
    }

    Ok(())
}

async fn handle_list_files(directory: Option<PathBuf>) -> Result<()> {
    let session_manager = SessionManager::new(get_default_session_path());
    let session = session_manager.get_session_or_error()
        .context("Authentication required")?;

    println!("Active session: user '{}' on port {} at {} baud", 
             session.username, session.port, session.baud_rate);

    let dir = directory.unwrap_or_else(|| PathBuf::from("."));
    
    println!("\nListing files in: {}", dir.display());
    
    let entries = tokio::fs::read_dir(&dir)
        .await
        .context(format!("Failed to read directory: {}", dir.display()))?;

    let mut entries_vec = Vec::new();
    let mut entries_stream = entries;
    
    while let Some(entry) = entries_stream.next_entry().await? {
        entries_vec.push(entry);
    }

    if entries_vec.is_empty() {
        println!("(empty directory)");
    } else {
        for entry in entries_vec {
            let metadata = entry.metadata().await?;
            let file_type = if metadata.is_dir() {
                "DIR "
            } else {
                "FILE"
            };
            let size = if metadata.is_file() {
                format!("{:>10} bytes", metadata.len())
            } else {
                String::from("           -")
            };
            
            println!("{} {} {}", file_type, size, entry.file_name().to_string_lossy());
        }
    }

    Ok(())
}

async fn handle_transfer(send: Option<PathBuf>, receive: Option<PathBuf>) -> Result<()> {
    let session_manager = SessionManager::new(get_default_session_path());
    let session = session_manager.get_session_or_error()
        .context("Authentication required")?;

    println!("Active session: user '{}' on port {} at {} baud", 
             session.username, session.port, session.baud_rate);

    if let Some(file_path) = send {
        println!("\nTransfer mode: SEND");
        println!("File: {}", file_path.display());
        println!("Port: {}", session.port);
        
        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file_path.display());
        }

        let metadata = tokio::fs::metadata(&file_path).await?;
        println!("Size: {} bytes", metadata.len());
        
        println!("\n[Transfer implementation pending]");
    } else if let Some(file_path) = receive {
        println!("\nTransfer mode: RECEIVE");
        println!("File: {}", file_path.display());
        println!("Port: {}", session.port);
        
        println!("\n[Transfer implementation pending]");
    } else {
        anyhow::bail!("Please specify either --send or --receive");
    }

    Ok(())
}
