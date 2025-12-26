//! File transfer protocol for Code Bridge

use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for file transfers
#[derive(Debug, Clone, Default)]
pub struct TransferProtocol;

impl AsRef<str> for TransferProtocol {
    fn as_ref(&self) -> &str {
        "/codebridge/transfer/1.0.0"
    }
}

/// File request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    /// Content hash of the requested file
    pub hash: String,

    /// Optional: specific byte range
    pub range: Option<(u64, u64)>,

    /// Request metadata
    pub metadata_only: bool,
}

/// File response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResponse {
    /// Content hash
    pub hash: String,

    /// File data (if not metadata_only)
    pub data: Option<Vec<u8>>,

    /// File metadata
    pub metadata: FileMetadata,

    /// Error message if request failed
    pub error: Option<String>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Original filename
    pub name: String,

    /// File size in bytes
    pub size: u64,

    /// MIME type
    pub mime_type: Option<String>,

    /// Creation time (Unix timestamp)
    pub created_at: Option<i64>,

    /// Modification time (Unix timestamp)
    pub modified_at: Option<i64>,

    /// Is this a directory?
    pub is_directory: bool,

    /// Chunk hashes for large files
    pub chunks: Option<Vec<String>>,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            size: 0,
            mime_type: None,
            created_at: None,
            modified_at: None,
            is_directory: false,
            chunks: None,
        }
    }
}

#[async_trait]
impl request_response::Codec for TransferProtocol {
    type Protocol = TransferProtocol;
    type Request = FileRequest;
    type Response = FileResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read JSON data
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read JSON data
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write JSON data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write JSON data
        io.write_all(&data).await?;
        io.flush().await?;

        Ok(())
    }
}
