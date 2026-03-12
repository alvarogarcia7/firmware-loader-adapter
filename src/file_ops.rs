use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub is_file: bool,
}

#[allow(dead_code)]
pub struct FileOperations;

#[allow(dead_code)]
impl FileOperations {
    pub async fn list_directory(path: &Path) -> Result<Vec<FileMetadata>> {
        let mut entries = Vec::new();

        let mut dir = tokio::fs::read_dir(path)
            .await
            .context("Failed to read directory")?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .context("Failed to read directory entry")?
        {
            let metadata = entry
                .metadata()
                .await
                .context("Failed to get entry metadata")?;
            let path = entry.path();

            let file_metadata = FileMetadata {
                path,
                size: metadata.len(),
                modified: metadata.modified().context("Failed to get modified time")?,
                is_file: metadata.is_file(),
            };

            entries.push(file_metadata);
        }

        Ok(entries)
    }

    pub async fn read_file(path: &Path) -> Result<Vec<u8>> {
        let mut file = File::open(path)
            .await
            .context("Failed to open file for reading")?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .context("Failed to read file")?;

        Ok(contents)
    }

    pub async fn read_file_chunked<F>(path: &Path, chunk_size: usize, mut callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        let mut file = File::open(path)
            .await
            .context("Failed to open file for reading")?;

        let mut buffer = vec![0u8; chunk_size];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .await
                .context("Failed to read chunk from file")?;

            if bytes_read == 0 {
                break;
            }

            callback(&buffer[..bytes_read])?;
        }

        Ok(())
    }

    pub async fn read_chunk(file: &mut File, buffer: &mut [u8]) -> Result<usize> {
        file.read(buffer)
            .await
            .context("Failed to read chunk from file")
    }

    pub async fn compute_sha256(path: &Path) -> Result<String> {
        let mut file = File::open(path)
            .await
            .context("Failed to open file for hashing")?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .await
                .context("Failed to read file for hashing")?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    pub async fn verify_sha256(path: &Path, expected_hash: &str) -> Result<bool> {
        let computed_hash = Self::compute_sha256(path).await?;
        Ok(computed_hash.eq_ignore_ascii_case(expected_hash))
    }

    pub async fn compute_sha256_bytes(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub async fn write_file(path: &Path, data: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .context("Failed to open file for writing")?;

        file.write_all(data)
            .await
            .context("Failed to write to file")?;

        file.flush().await.context("Failed to flush file")?;
        Ok(())
    }

    pub async fn append_to_file(file: &mut File, data: &[u8]) -> Result<()> {
        file.write_all(data)
            .await
            .context("Failed to append to file")?;

        file.flush().await.context("Failed to flush file")?;
        Ok(())
    }

    pub async fn get_file_size(path: &Path) -> Result<u64> {
        let metadata = tokio::fs::metadata(path)
            .await
            .context("Failed to get file metadata")?;

        Ok(metadata.len())
    }

    pub async fn create_file(path: &Path) -> Result<File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .context("Failed to create file")
    }
}
