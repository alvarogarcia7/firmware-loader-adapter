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
        handle_send_file(&session, file_path).await?;
    } else if let Some(file_path) = receive {
        handle_receive_file(&session, file_path).await?;
    } else {
        anyhow::bail!("Please specify either --send or --receive");
    }

    Ok(())
}

async fn handle_send_file(session: &Session, file_path: PathBuf) -> Result<()> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    use std::time::Instant;

    println!("\nTransfer mode: SEND");
    println!("File: {}", file_path.display());
    println!("Port: {}", session.port);
    
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let metadata = tokio::fs::metadata(&file_path).await
        .context("Failed to get file metadata")?;
    let file_size = metadata.len();
    
    println!("Size: {} bytes", file_size);
    println!("\nComputing file checksum...");
    
    let checksum = file_ops::FileOperations::compute_sha256(&file_path).await
        .context("Failed to compute file checksum")?;
    
    println!("Checksum: {}", checksum);
    println!("\nEstablishing serial connection...");
    
    let serial_config = serial::SerialConfig::new(session.baud_rate);
    let mut serial_transfer = serial::SerialTransfer::new(&session.port, serial_config)
        .context("Failed to establish serial connection")?;
    
    println!("Connected to {}", session.port);
    
    let chunk_size = 4096;
    let protocol_handler = protocol::ProtocolHandler::new(chunk_size);
    
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    println!("\nSending FileStart message...");
    let file_start_frame = protocol_handler.create_file_start(
        filename.clone(),
        file_size,
        Some(checksum.clone())
    ).context("Failed to create FileStart frame")?;
    
    let file_start_data = file_start_frame.serialize()
        .context("Failed to serialize FileStart frame")?;
    
    serial_transfer.send_message_with_retry(&file_start_data).await
        .context("Failed to send FileStart message")?;
    
    println!("Waiting for acknowledgment...");
    let ack_data = serial_transfer.receive_message_with_retry().await
        .context("Failed to receive FileStart acknowledgment")?;
    
    let ack_frame = protocol::Frame::deserialize(&ack_data)
        .context("Failed to deserialize acknowledgment")?;
    
    if ack_frame.message_type != protocol::MessageType::Ack {
        if ack_frame.message_type == protocol::MessageType::Error {
            if let Ok(protocol::MessagePayload::Error { code, message }) = ack_frame.extract_payload() {
                anyhow::bail!("Receiver returned error {}: {}", code, message);
            }
        }
        anyhow::bail!("Expected Ack, got {:?}", ack_frame.message_type);
    }
    
    println!("FileStart acknowledged");
    
    let mut file = File::open(&file_path).await
        .context("Failed to open file for reading")?;
    
    let mut buffer = vec![0u8; chunk_size];
    let mut sequence: u32 = 0;
    let mut bytes_sent: u64 = 0;
    let total_chunks = ((file_size as f64) / (chunk_size as f64)).ceil() as u32;
    
    let start_time = Instant::now();
    
    println!("\nSending file data...");
    
    loop {
        let bytes_read = file.read(&mut buffer).await
            .context("Failed to read from file")?;
        
        if bytes_read == 0 {
            break;
        }
        
        let chunk_data = buffer[..bytes_read].to_vec();
        
        let chunk_frame = protocol_handler.create_file_chunk(sequence, chunk_data)
            .context("Failed to create FileChunk frame")?;
        
        let chunk_frame_data = chunk_frame.serialize()
            .context("Failed to serialize FileChunk frame")?;
        
        serial_transfer.send_message_with_retry(&chunk_frame_data).await
            .context(format!("Failed to send chunk {}", sequence))?;
        
        let ack_data = serial_transfer.receive_message_with_retry().await
            .context(format!("Failed to receive acknowledgment for chunk {}", sequence))?;
        
        let ack_frame = protocol::Frame::deserialize(&ack_data)
            .context("Failed to deserialize chunk acknowledgment")?;
        
        if ack_frame.message_type == protocol::MessageType::Error {
            if let Ok(protocol::MessagePayload::Error { code, message }) = ack_frame.extract_payload() {
                anyhow::bail!("Error at chunk {}: {} (code {})", sequence, message, code);
            }
            anyhow::bail!("Received error response for chunk {}", sequence);
        }
        
        if ack_frame.message_type != protocol::MessageType::Ack {
            anyhow::bail!("Expected Ack for chunk {}, got {:?}", sequence, ack_frame.message_type);
        }
        
        bytes_sent += bytes_read as u64;
        sequence += 1;
        
        let progress = (bytes_sent as f64 / file_size as f64) * 100.0;
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { bytes_sent as f64 / elapsed / 1024.0 } else { 0.0 };
        
        print!("\rProgress: {}/{} bytes ({:.1}%) - Chunk {}/{} - Speed: {:.2} KB/s",
               bytes_sent, file_size, progress, sequence, total_chunks, speed);
        io::stdout().flush()?;
    }
    
    println!();
    println!("\nSending FileEnd message...");
    
    let file_end_frame = protocol_handler.create_file_end(sequence, Some(checksum.clone()))
        .context("Failed to create FileEnd frame")?;
    
    let file_end_data = file_end_frame.serialize()
        .context("Failed to serialize FileEnd frame")?;
    
    serial_transfer.send_message_with_retry(&file_end_data).await
        .context("Failed to send FileEnd message")?;
    
    println!("Waiting for final acknowledgment...");
    let final_ack_data = serial_transfer.receive_message_with_retry().await
        .context("Failed to receive FileEnd acknowledgment")?;
    
    let final_ack_frame = protocol::Frame::deserialize(&final_ack_data)
        .context("Failed to deserialize final acknowledgment")?;
    
    if final_ack_frame.message_type == protocol::MessageType::Error {
        if let Ok(protocol::MessagePayload::Error { code, message }) = final_ack_frame.extract_payload() {
            anyhow::bail!("Transfer failed: {} (code {})", message, code);
        }
    }
    
    if final_ack_frame.message_type != protocol::MessageType::Ack {
        anyhow::bail!("Expected Ack for FileEnd, got {:?}", final_ack_frame.message_type);
    }
    
    let elapsed = start_time.elapsed();
    let avg_speed = if elapsed.as_secs_f64() > 0.0 {
        bytes_sent as f64 / elapsed.as_secs_f64() / 1024.0
    } else {
        0.0
    };
    
    println!("\n=== Transfer Complete ===");
    println!("File: {}", filename);
    println!("Size: {} bytes", file_size);
    println!("Chunks sent: {}", sequence);
    println!("Time elapsed: {:.2}s", elapsed.as_secs_f64());
    println!("Average speed: {:.2} KB/s", avg_speed);
    println!("Checksum: {}", checksum);
    
    Ok(())
}

async fn handle_receive_file(session: &Session, file_path: PathBuf) -> Result<()> {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use std::time::Instant;

    println!("\nTransfer mode: RECEIVE");
    println!("File: {}", file_path.display());
    println!("Port: {}", session.port);
    
    println!("\nEstablishing serial connection...");
    
    let serial_config = serial::SerialConfig::new(session.baud_rate);
    let mut serial_transfer = serial::SerialTransfer::new(&session.port, serial_config)
        .context("Failed to establish serial connection")?;
    
    println!("Connected to {}", session.port);
    
    let chunk_size = 4096;
    let protocol_handler = protocol::ProtocolHandler::new(chunk_size);
    
    println!("\nWaiting for FileStart message...");
    let file_start_data = serial_transfer.receive_message_with_retry().await
        .context("Failed to receive FileStart message")?;
    
    let file_start_frame = protocol::Frame::deserialize(&file_start_data)
        .context("Failed to deserialize FileStart frame")?;
    
    if file_start_frame.message_type != protocol::MessageType::FileStart {
        let error_frame = protocol_handler.create_error(1, "Expected FileStart message".to_string())?;
        let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
        anyhow::bail!("Expected FileStart, got {:?}", file_start_frame.message_type);
    }
    
    let (filename, file_size, expected_checksum) = match file_start_frame.extract_payload()? {
        protocol::MessagePayload::FileStart { filename, file_size, checksum } => {
            (filename, file_size, checksum)
        }
        _ => {
            let error_frame = protocol_handler.create_error(2, "Invalid FileStart payload".to_string())?;
            let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
            anyhow::bail!("Invalid FileStart payload");
        }
    };
    
    println!("Receiving file: {}", filename);
    println!("Expected size: {} bytes", file_size);
    if let Some(ref checksum) = expected_checksum {
        println!("Expected checksum: {}", checksum);
    }
    
    let ack_frame = protocol_handler.create_ack(0, protocol::MessageType::FileStart)
        .context("Failed to create acknowledgment")?;
    let ack_data = ack_frame.serialize()
        .context("Failed to serialize acknowledgment")?;
    
    serial_transfer.send_message_with_retry(&ack_data).await
        .context("Failed to send FileStart acknowledgment")?;
    
    let mut file = File::create(&file_path).await
        .context("Failed to create output file")?;
    
    let mut bytes_received: u64 = 0;
    let mut expected_sequence: u32 = 0;
    let start_time = Instant::now();
    
    println!("\nReceiving file data...");
    
    loop {
        let chunk_data = serial_transfer.receive_message_with_retry().await
            .context("Failed to receive message")?;
        
        let chunk_frame = protocol::Frame::deserialize(&chunk_data)
            .context("Failed to deserialize frame")?;
        
        match chunk_frame.message_type {
            protocol::MessageType::FileChunk => {
                let (sequence, data) = match chunk_frame.extract_payload()? {
                    protocol::MessagePayload::FileChunk { sequence, data } => (sequence, data),
                    _ => {
                        let error_frame = protocol_handler.create_error(3, "Invalid FileChunk payload".to_string())?;
                        let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                        anyhow::bail!("Invalid FileChunk payload");
                    }
                };
                
                if sequence != expected_sequence {
                    let error_frame = protocol_handler.create_error(
                        4,
                        format!("Sequence mismatch: expected {}, got {}", expected_sequence, sequence)
                    )?;
                    let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                    anyhow::bail!("Sequence mismatch: expected {}, got {}", expected_sequence, sequence);
                }
                
                file.write_all(&data).await
                    .context("Failed to write chunk to file")?;
                
                bytes_received += data.len() as u64;
                expected_sequence += 1;
                
                let ack_frame = protocol_handler.create_ack(sequence, protocol::MessageType::FileChunk)?;
                let ack_data = ack_frame.serialize()?;
                serial_transfer.send_message_with_retry(&ack_data).await
                    .context("Failed to send chunk acknowledgment")?;
                
                let progress = (bytes_received as f64 / file_size as f64) * 100.0;
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 { bytes_received as f64 / elapsed / 1024.0 } else { 0.0 };
                
                print!("\rProgress: {}/{} bytes ({:.1}%) - Chunk {} - Speed: {:.2} KB/s",
                       bytes_received, file_size, progress, sequence, speed);
                io::stdout().flush()?;
            }
            protocol::MessageType::FileEnd => {
                let (total_chunks, checksum) = match chunk_frame.extract_payload()? {
                    protocol::MessagePayload::FileEnd { total_chunks, checksum } => (total_chunks, checksum),
                    _ => {
                        let error_frame = protocol_handler.create_error(5, "Invalid FileEnd payload".to_string())?;
                        let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                        anyhow::bail!("Invalid FileEnd payload");
                    }
                };
                
                println!();
                println!("\nReceived FileEnd message");
                println!("Total chunks: {}", total_chunks);
                
                if total_chunks != expected_sequence {
                    let error_frame = protocol_handler.create_error(
                        6,
                        format!("Chunk count mismatch: expected {}, got {}", expected_sequence, total_chunks)
                    )?;
                    let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                    anyhow::bail!("Chunk count mismatch: expected {}, got {}", expected_sequence, total_chunks);
                }
                
                file.flush().await.context("Failed to flush file")?;
                drop(file);
                
                if let Some(ref expected_checksum) = expected_checksum {
                    println!("\nVerifying file integrity...");
                    let computed_checksum = file_ops::FileOperations::compute_sha256(&file_path).await
                        .context("Failed to compute received file checksum")?;
                    
                    if computed_checksum.eq_ignore_ascii_case(expected_checksum) {
                        println!("Checksum verification: PASSED");
                    } else {
                        let error_frame = protocol_handler.create_error(
                            7,
                            format!("Checksum mismatch: expected {}, got {}", expected_checksum, computed_checksum)
                        )?;
                        let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                        anyhow::bail!(
                            "Checksum verification failed!\nExpected: {}\nComputed: {}",
                            expected_checksum, computed_checksum
                        );
                    }
                }
                
                let ack_frame = protocol_handler.create_ack(total_chunks, protocol::MessageType::FileEnd)?;
                let ack_data = ack_frame.serialize()?;
                serial_transfer.send_message_with_retry(&ack_data).await
                    .context("Failed to send FileEnd acknowledgment")?;
                
                let elapsed = start_time.elapsed();
                let avg_speed = if elapsed.as_secs_f64() > 0.0 {
                    bytes_received as f64 / elapsed.as_secs_f64() / 1024.0
                } else {
                    0.0
                };
                
                println!("\n=== Transfer Complete ===");
                println!("File: {}", filename);
                println!("Saved to: {}", file_path.display());
                println!("Size: {} bytes", bytes_received);
                println!("Chunks received: {}", total_chunks);
                println!("Time elapsed: {:.2}s", elapsed.as_secs_f64());
                println!("Average speed: {:.2} KB/s", avg_speed);
                if let Some(checksum) = checksum {
                    println!("Checksum: {}", checksum);
                }
                
                break;
            }
            protocol::MessageType::Error => {
                if let Ok(protocol::MessagePayload::Error { code, message }) = chunk_frame.extract_payload() {
                    anyhow::bail!("Received error from sender: {} (code {})", message, code);
                }
                anyhow::bail!("Received error frame from sender");
            }
            _ => {
                let error_frame = protocol_handler.create_error(
                    8,
                    format!("Unexpected message type: {:?}", chunk_frame.message_type)
                )?;
                let _ = serial_transfer.send_message(&error_frame.serialize()?).await;
                anyhow::bail!("Unexpected message type: {:?}", chunk_frame.message_type);
            }
        }
    }
    
    Ok(())
}
