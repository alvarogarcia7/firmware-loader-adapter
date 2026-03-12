---
id: A-1
title: 'Baseline implementation: Secure Rust CLI for Serial File Transfer'
status: Done
assignee: []
created_date: '2026-03-12 11:44'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Secure Rust CLI for Serial File Transfer

Build a Rust CLI application that authenticates users with AES-256-GCM encrypted credentials, lists files from a source directory, and transfers them to a serial device using a custom extensible protocol with error handling and integrity verification.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Key Decisions

Use AES-256-GCM with scrypt key derivation for password-based credential storage rather than asymmetric cryptography to balance security and simplicity

Implement a custom frame-based serial protocol with type-length-value (TLV) encoding for extensibility rather than using existing protocols like XMODEM

Use tokio-serial with async I/O for serial communication to support non-blocking operations and future scalability
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Initialize Rust project with Cargo.toml dependencies (clap for CLI, tokio for async runtime, tokio-serial for serial I/O, aes-gcm and scrypt for encryption, serde for serialization, anyhow for error handling) and create basic project structure with src/main.rs, src/auth.rs, src/protocol.rs, src/serial.rs, and src/file_ops.rs modules.

Implement authentication module in src/auth.rs with Credentials struct, store_credentials() function using AES-256-GCM encryption with scrypt-derived keys, verify_credentials() function for login validation, and secure key derivation from user password with salt storage.

Implement CLI interface in src/main.rs using clap with subcommands login, list-files, and transfer, password input using rpassword crate for secure terminal input, and session management storing authenticated state.

Implement file operations module in src/file_ops.rs with list_directory() function to enumerate files in origin folder with metadata (size, modified time), file reading with chunked I/O for large files, and integrity checking using SHA-256 hashing.

Design and implement extensible serial protocol in src/protocol.rs with TLV frame structure (message type enum for FILE_START, FILE_CHUNK, FILE_END, ACK, ERROR), serialization/deserialization using bincode, CRC-16 checksums for frame integrity, and version field for protocol evolution.

Implement serial communication in src/serial.rs with SerialTransfer struct wrapping tokio-serial, port enumeration and configuration (baud rate, data bits, parity), async send/receive methods with timeout handling, and retry logic with exponential backoff for failed transmissions.

Integrate all components in src/main.rs to wire transfer subcommand: authenticate user, list files from origin directory, establish serial connection, send files using protocol with progress reporting, handle acknowledgments and errors, and log transfer completion with statistics.

Add configuration file support (TOML format) for default serial port, baud rate, origin folder path, and credential storage location; add README.md with usage examples, security considerations, and protocol specification; add .gitignore for Rust artifacts and credential files.
<!-- SECTION:NOTES:END -->
