use anyhow::{anyhow, Result};
use crc::{Crc, CRC_16_IBM_SDLC};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
const PROTOCOL_VERSION: u8 = 1;
#[allow(dead_code)]
const CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    FileStart = 0x01,
    FileChunk = 0x02,
    FileEnd = 0x03,
    Ack = 0x04,
    Error = 0x05,
}

#[allow(dead_code)]
impl MessageType {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(MessageType::FileStart),
            0x02 => Ok(MessageType::FileChunk),
            0x03 => Ok(MessageType::FileEnd),
            0x04 => Ok(MessageType::Ack),
            0x05 => Ok(MessageType::Error),
            _ => Err(anyhow!("Unknown message type: 0x{:02X}", value)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessagePayload {
    FileStart {
        filename: String,
        file_size: u64,
        checksum: Option<String>,
    },
    FileChunk {
        sequence: u32,
        data: Vec<u8>,
    },
    FileEnd {
        total_chunks: u32,
        checksum: Option<String>,
    },
    Ack {
        sequence: u32,
        message_type: u8,
    },
    Error {
        code: u32,
        message: String,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Frame {
    pub version: u8,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub checksum: u16,
}

#[allow(dead_code)]
impl Frame {
    pub fn new(message_type: MessageType, payload: MessagePayload) -> Result<Self> {
        let payload_bytes = bincode::serialize(&payload)
            .map_err(|e| anyhow!("Failed to serialize payload: {}", e))?;

        let mut frame = Frame {
            version: PROTOCOL_VERSION,
            message_type,
            payload: payload_bytes,
            checksum: 0,
        };

        frame.checksum = frame.calculate_checksum();
        Ok(frame)
    }

    pub fn calculate_checksum(&self) -> u16 {
        let mut data = Vec::new();
        data.push(self.version);
        data.push(self.message_type.to_u8());
        data.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        data.extend_from_slice(&self.payload);

        CRC16.checksum(&data)
    }

    pub fn verify_checksum(&self) -> bool {
        let calculated = self.calculate_checksum();
        calculated == self.checksum
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        buffer.push(self.version);
        buffer.push(self.message_type.to_u8());

        let payload_len = self.payload.len() as u32;
        buffer.extend_from_slice(&payload_len.to_be_bytes());

        buffer.extend_from_slice(&self.payload);

        buffer.extend_from_slice(&self.checksum.to_be_bytes());

        Ok(buffer)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Frame too short: {} bytes", data.len()));
        }

        let version = data[0];
        if version != PROTOCOL_VERSION {
            return Err(anyhow!("Unsupported protocol version: {}", version));
        }

        let message_type = MessageType::from_u8(data[1])?;

        let payload_len = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;

        if data.len() < 8 + payload_len {
            return Err(anyhow!(
                "Incomplete frame: expected {} bytes, got {}",
                8 + payload_len,
                data.len()
            ));
        }

        let payload = data[6..6 + payload_len].to_vec();

        let checksum = u16::from_be_bytes([data[6 + payload_len], data[6 + payload_len + 1]]);

        let frame = Frame {
            version,
            message_type,
            payload,
            checksum,
        };

        if !frame.verify_checksum() {
            return Err(anyhow!("Checksum verification failed"));
        }

        Ok(frame)
    }

    pub fn get_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        bincode::deserialize(&self.payload)
            .map_err(|e| anyhow!("Failed to deserialize payload: {}", e))
    }

    pub fn extract_payload(&self) -> Result<MessagePayload> {
        self.get_payload()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProtocolHandler {
    chunk_size: usize,
}

#[allow(dead_code)]
impl ProtocolHandler {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    pub fn get_chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn create_file_start(
        &self,
        filename: String,
        file_size: u64,
        checksum: Option<String>,
    ) -> Result<Frame> {
        let payload = MessagePayload::FileStart {
            filename,
            file_size,
            checksum,
        };
        Frame::new(MessageType::FileStart, payload)
    }

    pub fn create_file_chunk(&self, sequence: u32, data: Vec<u8>) -> Result<Frame> {
        let payload = MessagePayload::FileChunk { sequence, data };
        Frame::new(MessageType::FileChunk, payload)
    }

    pub fn create_file_end(&self, total_chunks: u32, checksum: Option<String>) -> Result<Frame> {
        let payload = MessagePayload::FileEnd {
            total_chunks,
            checksum,
        };
        Frame::new(MessageType::FileEnd, payload)
    }

    pub fn create_ack(&self, sequence: u32, message_type: MessageType) -> Result<Frame> {
        let payload = MessagePayload::Ack {
            sequence,
            message_type: message_type.to_u8(),
        };
        Frame::new(MessageType::Ack, payload)
    }

    pub fn create_error(&self, code: u32, message: String) -> Result<Frame> {
        let payload = MessagePayload::Error { code, message };
        Frame::new(MessageType::Error, payload)
    }

    pub fn parse_frame(&self, data: &[u8]) -> Result<Frame> {
        Frame::deserialize(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::FileStart.to_u8(), 0x01);
        assert_eq!(MessageType::from_u8(0x01).unwrap(), MessageType::FileStart);
        assert_eq!(MessageType::FileChunk.to_u8(), 0x02);
        assert_eq!(MessageType::from_u8(0x02).unwrap(), MessageType::FileChunk);
    }

    #[test]
    fn test_frame_serialization() {
        let payload = MessagePayload::FileStart {
            filename: "test.txt".to_string(),
            file_size: 1024,
            checksum: Some("abc123".to_string()),
        };

        let frame = Frame::new(MessageType::FileStart, payload).unwrap();
        let serialized = frame.serialize().unwrap();
        let deserialized = Frame::deserialize(&serialized).unwrap();

        assert_eq!(frame.version, deserialized.version);
        assert_eq!(frame.message_type, deserialized.message_type);
        assert_eq!(frame.payload, deserialized.payload);
        assert_eq!(frame.checksum, deserialized.checksum);
    }

    #[test]
    fn test_checksum_verification() {
        let payload = MessagePayload::FileChunk {
            sequence: 1,
            data: vec![1, 2, 3, 4, 5],
        };

        let frame = Frame::new(MessageType::FileChunk, payload).unwrap();
        assert!(frame.verify_checksum());

        let mut corrupted_frame = frame.clone();
        corrupted_frame.payload[0] ^= 0xFF;
        assert!(!corrupted_frame.verify_checksum());
    }

    #[test]
    fn test_protocol_handler() {
        let handler = ProtocolHandler::new(4096);

        let frame = handler
            .create_file_start("test.txt".to_string(), 1024, Some("hash".to_string()))
            .unwrap();

        let serialized = frame.serialize().unwrap();
        let parsed = handler.parse_frame(&serialized).unwrap();

        if let MessagePayload::FileStart {
            filename,
            file_size,
            ..
        } = parsed.extract_payload().unwrap()
        {
            assert_eq!(filename, "test.txt");
            assert_eq!(file_size, 1024);
        } else {
            panic!("Wrong payload type");
        }
    }

    #[test]
    fn test_ack_frame() {
        let handler = ProtocolHandler::new(4096);
        let frame = handler.create_ack(42, MessageType::FileChunk).unwrap();

        let serialized = frame.serialize().unwrap();
        let parsed = Frame::deserialize(&serialized).unwrap();

        if let MessagePayload::Ack {
            sequence,
            message_type,
        } = parsed.extract_payload().unwrap()
        {
            assert_eq!(sequence, 42);
            assert_eq!(message_type, MessageType::FileChunk.to_u8());
        } else {
            panic!("Wrong payload type");
        }
    }

    #[test]
    fn test_error_frame() {
        let handler = ProtocolHandler::new(4096);
        let frame = handler.create_error(404, "Not found".to_string()).unwrap();

        let serialized = frame.serialize().unwrap();
        let parsed = Frame::deserialize(&serialized).unwrap();

        if let MessagePayload::Error { code, message } = parsed.extract_payload().unwrap() {
            assert_eq!(code, 404);
            assert_eq!(message, "Not found");
        } else {
            panic!("Wrong payload type");
        }
    }

    #[test]
    fn test_invalid_version() {
        let data = vec![99, 0x01, 0, 0, 0, 0, 0, 0];
        let result = Frame::deserialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_message_type() {
        let data = vec![1, 0xFF, 0, 0, 0, 0, 0, 0];
        let result = Frame::deserialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_too_short() {
        let data = vec![1, 2, 3];
        let result = Frame::deserialize(&data);
        assert!(result.is_err());
    }
}
