//! AMQP protocol handling
//!
//! This module implements the AMQP 0.9.1 protocol for handling connections,
//! channels, and method frames.

use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// AMQP frame types
#[derive(Debug)]
pub enum FrameType {
    Method = 1,
    Header = 2,
    Body = 3,
    Heartbeat = 8,
}

/// AMQP frame
#[derive(Debug)]
pub struct Frame {
    pub frame_type: FrameType,
    pub channel: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Read a frame from the stream
    pub async fn read_from_stream(stream: &mut TcpStream) -> io::Result<Self> {
        // AMQP frame format: [frame-type (1 byte)] [channel (2 bytes)] [size (4 bytes)] [payload] [frame-end (1 byte)]
        let mut header = [0u8; 7];
        stream.read_exact(&mut header).await?;

        let frame_type = match header[0] {
            1 => FrameType::Method,
            2 => FrameType::Header,
            3 => FrameType::Body,
            8 => FrameType::Heartbeat,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid frame type")),
        };

        let channel = u16::from_be_bytes([header[1], header[2]]);
        let size = u32::from_be_bytes([header[3], header[4], header[5], header[6]]) as usize;

        let mut payload = vec![0u8; size];
        stream.read_exact(&mut payload).await?;

        // Read frame end marker
        let mut frame_end = [0u8; 1];
        stream.read_exact(&mut frame_end).await?;
        if frame_end[0] != 0xCE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid frame end marker"));
        }

        Ok(Frame {
            frame_type,
            channel,
            payload,
        })
    }

    /// Write a frame to the stream
    pub async fn write_to_stream(&self, stream: &mut TcpStream) -> io::Result<()> {
        let frame_type_byte = match self.frame_type {
            FrameType::Method => 1,
            FrameType::Header => 2,
            FrameType::Body => 3,
            FrameType::Heartbeat => 8,
        };

        let size = self.payload.len() as u32;
        let size_bytes = size.to_be_bytes();

        // Write frame header
        stream.write_all(&[frame_type_byte]).await?;
        stream.write_all(&self.channel.to_be_bytes()).await?;
        stream.write_all(&size_bytes).await?;

        // Write payload
        stream.write_all(&self.payload).await?;

        // Write frame end marker
        stream.write_all(&[0xCE]).await?;

        Ok(())
    }
}

/// AMQP connection handler
pub struct ConnectionHandler {
    stream: TcpStream,
}

impl ConnectionHandler {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    /// Handle the AMQP connection
    pub async fn handle(mut self) -> io::Result<()> {
        // Send protocol header
        self.send_protocol_header().await?;

        loop {
            match Frame::read_from_stream(&mut self.stream).await {
                Ok(frame) => {
                    if let Err(e) = self.handle_frame(frame).await {
                        tracing::error!("Error handling frame: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading frame: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn send_protocol_header(&mut self) -> io::Result<()> {
        // AMQP 0.9.1 protocol header: "AMQP\x00\x00\x09\x01"
        let header = b"AMQP\x00\x00\x09\x01";
        self.stream.write_all(header).await
    }

    async fn handle_frame(&mut self, frame: Frame) -> io::Result<()> {
        match frame.frame_type {
            FrameType::Method => {
                // TODO: Parse and handle AMQP methods
                tracing::debug!("Received method frame on channel {}", frame.channel);
            }
            FrameType::Heartbeat => {
                // Respond to heartbeat
                let response = Frame {
                    frame_type: FrameType::Heartbeat,
                    channel: frame.channel,
                    payload: vec![],
                };
                response.write_to_stream(&mut self.stream).await?;
            }
            _ => {
                tracing::debug!("Received frame type {:?} on channel {}", frame.frame_type, frame.channel);
            }
        }
        Ok(())
    }
}
