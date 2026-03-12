use tokio_serial::{SerialStream, SerialPortBuilderExt, DataBits, Parity, StopBits};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration, sleep};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub timeout: Duration,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_secs(5),
        }
    }
}

#[allow(dead_code)]
impl SerialConfig {
    pub fn new(baud_rate: u32) -> Self {
        Self {
            baud_rate,
            ..Default::default()
        }
    }

    pub fn with_data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub fn with_parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn with_stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

#[allow(dead_code)]
impl RetryConfig {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64 
            * self.backoff_multiplier.powi(attempt as i32);
        let delay = Duration::from_millis(delay_ms as u64);
        
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PortInfo {
    pub port_name: String,
    pub port_type: String,
}

#[allow(dead_code)]
pub struct SerialTransfer {
    port: SerialStream,
    config: SerialConfig,
    retry_config: RetryConfig,
}

#[allow(dead_code)]
impl SerialTransfer {
    pub fn new(port_name: &str, config: SerialConfig) -> Result<Self> {
        Self::new_with_retry(port_name, config, RetryConfig::default())
    }

    pub fn new_with_retry(port_name: &str, config: SerialConfig, retry_config: RetryConfig) -> Result<Self> {
        let port = tokio_serial::new(port_name, config.baud_rate)
            .data_bits(config.data_bits)
            .parity(config.parity)
            .stop_bits(config.stop_bits)
            .open_native_async()
            .context(format!("Failed to open serial port: {}", port_name))?;

        Ok(Self {
            port,
            config,
            retry_config,
        })
    }

    pub fn enumerate_ports() -> Result<Vec<PortInfo>> {
        let ports = tokio_serial::available_ports()
            .context("Failed to enumerate serial ports")?;

        Ok(ports
            .into_iter()
            .map(|port| PortInfo {
                port_name: port.port_name,
                port_type: format!("{:?}", port.port_type),
            })
            .collect())
    }

    pub fn get_config(&self) -> &SerialConfig {
        &self.config
    }

    pub fn get_retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_config = retry_config;
    }

    async fn send_internal(&mut self, data: &[u8]) -> Result<()> {
        let len = data.len() as u32;
        self.port
            .write_all(&len.to_be_bytes())
            .await
            .context("Failed to write data length")?;

        self.port
            .write_all(data)
            .await
            .context("Failed to write data")?;

        self.port
            .flush()
            .await
            .context("Failed to flush serial port")?;

        Ok(())
    }

    async fn receive_internal(&mut self) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        self.port
            .read_exact(&mut len_buf)
            .await
            .context("Failed to read data length")?;

        let len = u32::from_be_bytes(len_buf) as usize;
        
        if len == 0 {
            return Err(anyhow!("Received zero-length message"));
        }

        if len > 10_000_000 {
            return Err(anyhow!("Received message length too large: {} bytes", len));
        }

        let mut data = vec![0u8; len];
        self.port
            .read_exact(&mut data)
            .await
            .context("Failed to read data")?;

        Ok(data)
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        match timeout(self.config.timeout, self.send_internal(data)).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!("Send operation timed out after {:?}", self.config.timeout)),
        }
    }

    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        match timeout(self.config.timeout, self.receive_internal()).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!("Receive operation timed out after {:?}", self.config.timeout)),
        }
    }

    pub async fn send_with_retry(&mut self, data: &[u8]) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = self.retry_config.calculate_delay(attempt - 1);
                sleep(delay).await;
            }

            match self.send(data).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(anyhow!(
            "Send failed after {} attempts: {}",
            self.retry_config.max_retries + 1,
            last_error.unwrap()
        ))
    }

    pub async fn receive_with_retry(&mut self) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = self.retry_config.calculate_delay(attempt - 1);
                sleep(delay).await;
            }

            match self.receive().await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(anyhow!(
            "Receive failed after {} attempts: {}",
            self.retry_config.max_retries + 1,
            last_error.unwrap()
        ))
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<()> {
        self.send(message).await
    }

    pub async fn receive_message(&mut self) -> Result<Vec<u8>> {
        self.receive().await
    }

    pub async fn send_message_with_retry(&mut self, message: &[u8]) -> Result<()> {
        self.send_with_retry(message).await
    }

    pub async fn receive_message_with_retry(&mut self) -> Result<Vec<u8>> {
        self.receive_with_retry().await
    }
}

#[allow(dead_code)]
pub fn list_available_ports() -> Result<Vec<String>> {
    let ports = tokio_serial::available_ports()
        .context("Failed to enumerate serial ports")?;

    Ok(ports.into_iter().map(|p| p.port_name).collect())
}

#[allow(dead_code)]
pub fn get_port_details() -> Result<HashMap<String, String>> {
    let ports = tokio_serial::available_ports()
        .context("Failed to enumerate serial ports")?;

    let mut details = HashMap::new();
    for port in ports {
        details.insert(port.port_name.clone(), format!("{:?}", port.port_type));
    }

    Ok(details)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_config_default() {
        let config = SerialConfig::default();
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
    }

    #[test]
    fn test_serial_config_builder() {
        let config = SerialConfig::new(9600)
            .with_data_bits(DataBits::Seven)
            .with_parity(Parity::Even)
            .with_stop_bits(StopBits::Two)
            .with_timeout(Duration::from_secs(10));

        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.data_bits, DataBits::Seven);
        assert_eq!(config.parity, Parity::Even);
        assert_eq!(config.stop_bits, StopBits::Two);
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(5));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new(5)
            .with_initial_delay(Duration::from_millis(50))
            .with_max_delay(Duration::from_secs(10))
            .with_backoff_multiplier(1.5);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(50));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.backoff_multiplier, 1.5);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let config = RetryConfig::default();
        
        let delay0 = config.calculate_delay(0);
        assert_eq!(delay0, Duration::from_millis(100));
        
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_millis(200));
        
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_millis(400));
        
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3, Duration::from_millis(800));
    }

    #[test]
    fn test_exponential_backoff_max_delay() {
        let config = RetryConfig::default()
            .with_max_delay(Duration::from_millis(300));
        
        let delay0 = config.calculate_delay(0);
        assert_eq!(delay0, Duration::from_millis(100));
        
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_millis(200));
        
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_millis(300));
        
        let delay10 = config.calculate_delay(10);
        assert_eq!(delay10, Duration::from_millis(300));
    }

    #[test]
    fn test_list_available_ports() {
        let result = list_available_ports();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_port_details() {
        let result = get_port_details();
        assert!(result.is_ok());
    }

    #[test]
    fn test_enumerate_ports() {
        let result = SerialTransfer::enumerate_ports();
        assert!(result.is_ok());
    }
}
