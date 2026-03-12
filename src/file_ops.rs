use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Context, Result};

#[allow(dead_code)]
pub struct FileOperations;

#[allow(dead_code)]
impl FileOperations {
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

    pub async fn read_chunk(file: &mut File, buffer: &mut [u8]) -> Result<usize> {
        file.read(buffer)
            .await
            .context("Failed to read chunk from file")
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
