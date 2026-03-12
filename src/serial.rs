use tokio_serial::{SerialStream, SerialPortBuilderExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Context, Result};

#[allow(dead_code)]
pub struct SerialConnection {
    port: SerialStream,
}

#[allow(dead_code)]
impl SerialConnection {
    pub fn new(port_name: &str, baud_rate: u32) -> Result<Self> {
        let port = tokio_serial::new(port_name, baud_rate)
            .open_native_async()
            .context("Failed to open serial port")?;

        Ok(Self { port })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let len = data.len() as u32;
        self.port
            .write_all(&len.to_be_bytes())
            .await
            .context("Failed to write length")?;

        self.port
            .write_all(data)
            .await
            .context("Failed to write data")?;

        self.port.flush().await.context("Failed to flush")?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        self.port
            .read_exact(&mut len_buf)
            .await
            .context("Failed to read length")?;

        let len = u32::from_be_bytes(len_buf) as usize;
        let mut data = vec![0u8; len];

        self.port
            .read_exact(&mut data)
            .await
            .context("Failed to read data")?;

        Ok(data)
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<()> {
        self.send(message).await
    }

    pub async fn receive_message(&mut self) -> Result<Vec<u8>> {
        self.receive().await
    }
}
