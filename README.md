# Secure Serial Transfer

A secure, reliable file transfer tool over serial connections with built-in authentication and integrity verification.

## Features

- 🔐 **Secure Authentication**: Password-based authentication with scrypt key derivation
- 📦 **Reliable Transfer**: CRC-16 checksums and acknowledgment-based protocol
- 🔄 **Automatic Retry**: Built-in retry mechanism for failed transmissions
- ✅ **Integrity Verification**: SHA-256 checksums for complete file validation
- ⚙️ **Configurable**: TOML-based configuration for defaults
- 📊 **Progress Tracking**: Real-time transfer progress and speed monitoring

## Installation

### Prerequisites

- Rust 1.70 or later
- Access to serial port hardware (e.g., `/dev/ttyUSB0`, `/dev/ttyACM0`, or `COM3`)

### Build from Source

```bash
git clone <repository-url>
cd secure-serial-transfer
cargo build --release
```

The binary will be available at `target/release/secure-serial-transfer`.

### Install Globally

```bash
cargo install --path .
```

## Configuration

The tool uses a TOML configuration file for default settings. The config file is located at:

- **Linux/macOS**: `~/.secure-serial-transfer/config.toml`
- **Windows**: `%USERPROFILE%\.secure-serial-transfer\config.toml`

### Example Configuration

Create a `config.toml` file with the following structure:

```toml
# Default serial port to use
serial_port = "/dev/ttyUSB0"  # Linux/macOS
# serial_port = "COM3"        # Windows

# Default baud rate
baud_rate = 115200

# Default directory for file operations
origin_folder = "."

# Credentials storage location
credentials_path = "/home/user/.secure-serial-transfer/credentials.json"
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `serial_port` | String | `/dev/ttyUSB0` (Linux) or `COM3` (Windows) | Serial port device path |
| `baud_rate` | Integer | `115200` | Serial communication baud rate |
| `origin_folder` | String | `.` | Default directory for file listings |
| `credentials_path` | String | `~/.secure-serial-transfer/credentials.json` | Location for encrypted credentials |

Command-line arguments override configuration file settings.

## Usage

### 1. Register a New User

First-time setup requires registering a user:

```bash
# Using config defaults
secure-serial-transfer login --username alice --register

# Specifying port and baud rate
secure-serial-transfer login --username alice --port /dev/ttyUSB0 --baud-rate 115200 --register
```

You will be prompted to enter and confirm a password. Credentials are encrypted and stored securely.

### 2. Login

Create an authenticated session:

```bash
secure-serial-transfer login --username alice
```

Sessions remain valid for 1 hour.

### 3. List Files

List files in a directory:

```bash
# List files in current directory (or origin_folder from config)
secure-serial-transfer list-files

# List files in specific directory
secure-serial-transfer list-files /path/to/directory
```

### 4. Transfer Files

#### Send a File

```bash
secure-serial-transfer transfer --send /path/to/file.txt
```

#### Receive a File

```bash
secure-serial-transfer transfer --receive /path/to/output.txt
```

### Using Custom Config

Specify a custom configuration file:

```bash
secure-serial-transfer --config /path/to/custom-config.toml login --username alice
```

## Protocol Specification

The Secure Serial Transfer protocol is a binary, frame-based protocol designed for reliable file transfers over serial connections.

### Frame Structure

All messages follow this frame structure:

```
+----------+-------------+---------------+---------+----------+
| Version  | Message Type| Payload Length| Payload | Checksum |
| (1 byte) | (1 byte)    | (4 bytes)     | (N bytes)| (2 bytes)|
+----------+-------------+---------------+---------+----------+
```

- **Version**: Protocol version (currently 0x01)
- **Message Type**: Type of message (see below)
- **Payload Length**: Length of payload in bytes (big-endian)
- **Payload**: Serialized message data (bincode format)
- **Checksum**: CRC-16-IBM-SDLC of version + type + length + payload

### Message Types

| Type | Value | Name | Description |
|------|-------|------|-------------|
| 0x01 | FileStart | File Start | Initiates file transfer with metadata |
| 0x02 | FileChunk | File Chunk | Contains a chunk of file data |
| 0x03 | FileEnd | File End | Signals end of file transfer |
| 0x04 | Ack | Acknowledgment | Acknowledges received message |
| 0x05 | Error | Error | Reports an error condition |

### Message Payloads

#### FileStart
```rust
{
    filename: String,        // Name of the file
    file_size: u64,          // Total size in bytes
    checksum: Option<String> // SHA-256 checksum (hex)
}
```

#### FileChunk
```rust
{
    sequence: u32,      // Sequential chunk number (0-based)
    data: Vec<u8>       // Chunk data (max 4096 bytes)
}
```

#### FileEnd
```rust
{
    total_chunks: u32,       // Total number of chunks sent
    checksum: Option<String> // SHA-256 checksum (hex)
}
```

#### Ack
```rust
{
    sequence: u32,      // Sequence number being acknowledged
    message_type: u8    // Type of message being acknowledged
}
```

#### Error
```rust
{
    code: u32,         // Error code
    message: String    // Human-readable error message
}
```

### Transfer Flow

#### Sender Side
1. Send **FileStart** with file metadata
2. Wait for **Ack**
3. For each chunk:
   - Send **FileChunk** with sequence and data
   - Wait for **Ack**
4. Send **FileEnd** with total chunks and checksum
5. Wait for final **Ack**

#### Receiver Side
1. Wait for **FileStart**
2. Send **Ack**
3. For each chunk:
   - Wait for **FileChunk**
   - Verify sequence number
   - Write data to file
   - Send **Ack**
4. Wait for **FileEnd**
5. Verify total chunks and checksum
6. Send final **Ack**

### Error Handling

- Each message transmission has automatic retry (up to 3 attempts)
- Retry timeout: 10 seconds
- Checksum mismatch results in error response
- Sequence number mismatch aborts transfer
- SHA-256 verification performed at transfer completion

## Security Considerations

### Authentication

- **Password Storage**: Passwords are never stored. Only scrypt-derived hashes are persisted.
- **Key Derivation**: Uses scrypt with parameters: N=32768, r=8, p=1, keylen=32
- **Encryption**: Credentials are encrypted with AES-256-GCM before storage
- **Salt**: Unique random salt generated for each password hash

### Credential Storage

Credentials are stored at `~/.secure-serial-transfer/credentials.json` (or as specified in config):

```json
{
  "salt": "base64-encoded-salt",
  "nonce": "hex-encoded-nonce",
  "encrypted_credentials": "hex-encoded-encrypted-data"
}
```

**⚠️ Security Best Practices:**

1. **Protect credential files**: Ensure proper file permissions (e.g., `chmod 600`)
2. **Use strong passwords**: Minimum 12 characters with mixed character types
3. **Secure the serial connection**: Physical security of serial cables and devices
4. **Verify checksums**: Always check transfer integrity via SHA-256 verification
5. **Session timeout**: Sessions expire after 1 hour for security
6. **Keep credentials private**: Never commit credential files to version control

### File Integrity

- **Pre-transfer checksum**: SHA-256 computed before sending
- **Post-transfer verification**: SHA-256 verified after receiving
- **Chunk-level validation**: CRC-16 checksum on each frame
- **Sequence verification**: Ensures chunks arrive in order without gaps

### Threat Model

**Protected Against:**
- Transmission errors and data corruption
- Unauthorized access (password authentication)
- Credential theft (encrypted storage)
- Man-in-the-middle (checksum verification)

**Not Protected Against:**
- Physical compromise of serial connection
- Keylogging or password theft
- Compromise of host system
- Side-channel attacks

**Note**: This tool is designed for local serial connections. It does not encrypt the data in transit over the serial line. For sensitive data, consider additional encryption layers.

## Troubleshooting

### Permission Denied on Serial Port

On Linux/macOS, you may need permissions to access the serial port:

```bash
# Add user to dialout group (Linux)
sudo usermod -a -G dialout $USER

# Or change permissions temporarily
sudo chmod 666 /dev/ttyUSB0
```

Log out and back in for group changes to take effect.

### Port Already in Use

Ensure no other application is using the serial port. Check with:

```bash
# Linux
lsof | grep ttyUSB0

# macOS
lsof | grep tty.usb
```

### Checksum Verification Failed

This indicates data corruption during transfer. Possible causes:
- Incorrect baud rate settings
- Poor quality serial cable
- Electrical interference
- Hardware issues

Try:
- Reducing baud rate (e.g., 9600)
- Using a shorter, higher-quality cable
- Checking hardware connections

### Session Expired

Sessions are valid for 1 hour. Simply login again:

```bash
secure-serial-transfer login --username alice
```

## Example Workflow

Complete workflow for transferring a file between two computers:

**On Computer A (Sender):**
```bash
# Initial setup
secure-serial-transfer login --username sender --port /dev/ttyUSB0 --register

# Login (subsequent uses)
secure-serial-transfer login --username sender

# Send file
secure-serial-transfer transfer --send document.pdf
```

**On Computer B (Receiver):**
```bash
# Initial setup
secure-serial-transfer login --username receiver --port /dev/ttyUSB0 --register

# Login (subsequent uses)
secure-serial-transfer login --username receiver

# Receive file
secure-serial-transfer transfer --receive document.pdf
```

Both systems must be connected via serial cable and configured with matching baud rates.

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

### Project Structure

```
src/
├── main.rs          # CLI interface and command handlers
├── auth.rs          # Authentication and credential management
├── config.rs        # Configuration file handling
├── protocol.rs      # Protocol frame definitions and handlers
├── serial.rs        # Serial port communication layer
├── file_ops.rs      # File operations and checksums
└── session.rs       # Session management
```

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please submit pull requests or open issues for bugs and feature requests.
