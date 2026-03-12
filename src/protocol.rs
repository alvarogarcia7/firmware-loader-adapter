use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub enum Message {
    FileHeader {
        filename: String,
        size: u64,
    },
    FileChunk {
        sequence: u32,
        data: Vec<u8>,
    },
    Acknowledgment {
        sequence: u32,
    },
    Error {
        message: String,
    },
    Complete,
}

#[allow(dead_code)]
impl Message {
    pub fn serialize(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))
    }
}

#[allow(dead_code)]
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

    pub fn create_file_header(&self, filename: String, size: u64) -> Message {
        Message::FileHeader { filename, size }
    }

    pub fn create_file_chunk(&self, sequence: u32, data: Vec<u8>) -> Message {
        Message::FileChunk { sequence, data }
    }

    pub fn create_acknowledgment(&self, sequence: u32) -> Message {
        Message::Acknowledgment { sequence }
    }

    pub fn create_error(&self, message: String) -> Message {
        Message::Error { message }
    }

    pub fn create_complete(&self) -> Message {
        Message::Complete
    }
}
