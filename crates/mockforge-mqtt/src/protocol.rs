//! MQTT 3.1.1 Protocol packet parsing and serialization
//!
//! This module implements the MQTT 3.1.1 protocol specification for packet encoding
//! and decoding. It supports all control packet types including CONNECT, PUBLISH,
//! SUBSCRIBE, and their acknowledgments.

use std::io::{self, Cursor, Read, Write as IoWrite};
use thiserror::Error;

/// MQTT Protocol Error types
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid packet type: {0}")]
    InvalidPacketType(u8),

    #[error("Invalid remaining length encoding")]
    InvalidRemainingLength,

    #[error("Invalid protocol name: {0}")]
    InvalidProtocolName(String),

    #[error("Invalid protocol version: {0}")]
    InvalidProtocolVersion(u8),

    #[error("Invalid QoS level: {0}")]
    InvalidQoS(u8),

    #[error("Invalid UTF-8 string")]
    InvalidUtf8,

    #[error("Packet too large: {0} bytes")]
    PacketTooLarge(usize),

    #[error("Incomplete packet: expected {expected} bytes, got {got}")]
    IncompletePacket { expected: usize, got: usize },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid connect flags")]
    InvalidConnectFlags,

    #[error("Malformed packet")]
    MalformedPacket,
}

/// Result type for protocol operations
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// MQTT Control Packet Types (4-bit identifier in first byte)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Connect = 1,
    Connack = 2,
    Publish = 3,
    Puback = 4,
    Pubrec = 5,
    Pubrel = 6,
    Pubcomp = 7,
    Subscribe = 8,
    Suback = 9,
    Unsubscribe = 10,
    Unsuback = 11,
    Pingreq = 12,
    Pingresp = 13,
    Disconnect = 14,
}

impl TryFrom<u8> for PacketType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> ProtocolResult<Self> {
        match value {
            1 => Ok(PacketType::Connect),
            2 => Ok(PacketType::Connack),
            3 => Ok(PacketType::Publish),
            4 => Ok(PacketType::Puback),
            5 => Ok(PacketType::Pubrec),
            6 => Ok(PacketType::Pubrel),
            7 => Ok(PacketType::Pubcomp),
            8 => Ok(PacketType::Subscribe),
            9 => Ok(PacketType::Suback),
            10 => Ok(PacketType::Unsubscribe),
            11 => Ok(PacketType::Unsuback),
            12 => Ok(PacketType::Pingreq),
            13 => Ok(PacketType::Pingresp),
            14 => Ok(PacketType::Disconnect),
            _ => Err(ProtocolError::InvalidPacketType(value)),
        }
    }
}

/// Quality of Service levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QoS {
    #[default]
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl TryFrom<u8> for QoS {
    type Error = ProtocolError;

    fn try_from(value: u8) -> ProtocolResult<Self> {
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err(ProtocolError::InvalidQoS(value)),
        }
    }
}

/// CONNACK return codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnackCode {
    Accepted = 0,
    UnacceptableProtocolVersion = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUsernamePassword = 4,
    NotAuthorized = 5,
}

impl From<ConnackCode> for u8 {
    fn from(code: ConnackCode) -> u8 {
        code as u8
    }
}

/// MQTT Control Packet representation
#[derive(Debug, Clone)]
pub enum Packet {
    Connect(ConnectPacket),
    Connack(ConnackPacket),
    Publish(PublishPacket),
    Puback(PubackPacket),
    Pubrec(PubrecPacket),
    Pubrel(PubrelPacket),
    Pubcomp(PubcompPacket),
    Subscribe(SubscribePacket),
    Suback(SubackPacket),
    Unsubscribe(UnsubscribePacket),
    Unsuback(UnsubackPacket),
    Pingreq,
    Pingresp,
    Disconnect,
}

/// CONNECT packet from client
#[derive(Debug, Clone)]
pub struct ConnectPacket {
    pub protocol_name: String,
    pub protocol_level: u8,
    pub clean_session: bool,
    pub keep_alive: u16,
    pub client_id: String,
    pub will: Option<Will>,
    pub username: Option<String>,
    pub password: Option<Vec<u8>>,
}

/// Last Will and Testament
#[derive(Debug, Clone)]
pub struct Will {
    pub topic: String,
    pub message: Vec<u8>,
    pub qos: QoS,
    pub retain: bool,
}

/// CONNACK packet to client
#[derive(Debug, Clone)]
pub struct ConnackPacket {
    pub session_present: bool,
    pub return_code: ConnackCode,
}

/// PUBLISH packet
#[derive(Debug, Clone)]
pub struct PublishPacket {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub topic: String,
    pub packet_id: Option<u16>,
    pub payload: Vec<u8>,
}

/// PUBACK packet (QoS 1 acknowledgment)
#[derive(Debug, Clone)]
pub struct PubackPacket {
    pub packet_id: u16,
}

/// PUBREC packet (QoS 2 step 1)
#[derive(Debug, Clone)]
pub struct PubrecPacket {
    pub packet_id: u16,
}

/// PUBREL packet (QoS 2 step 2)
#[derive(Debug, Clone)]
pub struct PubrelPacket {
    pub packet_id: u16,
}

/// PUBCOMP packet (QoS 2 step 3)
#[derive(Debug, Clone)]
pub struct PubcompPacket {
    pub packet_id: u16,
}

/// SUBSCRIBE packet
#[derive(Debug, Clone)]
pub struct SubscribePacket {
    pub packet_id: u16,
    pub subscriptions: Vec<(String, QoS)>,
}

/// SUBACK packet
#[derive(Debug, Clone)]
pub struct SubackPacket {
    pub packet_id: u16,
    pub return_codes: Vec<SubackReturnCode>,
}

/// SUBACK return codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubackReturnCode {
    SuccessQoS0,
    SuccessQoS1,
    SuccessQoS2,
    Failure,
}

impl From<SubackReturnCode> for u8 {
    fn from(code: SubackReturnCode) -> u8 {
        match code {
            SubackReturnCode::SuccessQoS0 => 0x00,
            SubackReturnCode::SuccessQoS1 => 0x01,
            SubackReturnCode::SuccessQoS2 => 0x02,
            SubackReturnCode::Failure => 0x80,
        }
    }
}

impl SubackReturnCode {
    /// Create a success return code for the given QoS
    pub fn success(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => SubackReturnCode::SuccessQoS0,
            QoS::AtLeastOnce => SubackReturnCode::SuccessQoS1,
            QoS::ExactlyOnce => SubackReturnCode::SuccessQoS2,
        }
    }
}

/// UNSUBSCRIBE packet
#[derive(Debug, Clone)]
pub struct UnsubscribePacket {
    pub packet_id: u16,
    pub topics: Vec<String>,
}

/// UNSUBACK packet
#[derive(Debug, Clone)]
pub struct UnsubackPacket {
    pub packet_id: u16,
}

/// Packet decoder for parsing MQTT packets from bytes
pub struct PacketDecoder;

impl PacketDecoder {
    /// Decode a single MQTT packet from a byte buffer
    ///
    /// Returns the parsed packet and number of bytes consumed
    pub fn decode(buffer: &[u8]) -> ProtocolResult<Option<(Packet, usize)>> {
        if buffer.is_empty() {
            return Ok(None);
        }

        // Read fixed header
        let first_byte = buffer[0];
        let packet_type = PacketType::try_from(first_byte >> 4)?;
        let flags = first_byte & 0x0F;

        // Read remaining length (variable length encoding)
        let (remaining_length, header_len) = Self::decode_remaining_length(&buffer[1..])?;

        let total_len = 1 + header_len + remaining_length;
        if buffer.len() < total_len {
            return Ok(None); // Need more data
        }

        let payload = &buffer[1 + header_len..total_len];

        let packet = match packet_type {
            PacketType::Connect => Packet::Connect(Self::decode_connect(payload)?),
            PacketType::Connack => Packet::Connack(Self::decode_connack(payload)?),
            PacketType::Publish => Packet::Publish(Self::decode_publish(flags, payload)?),
            PacketType::Puback => Packet::Puback(Self::decode_puback(payload)?),
            PacketType::Pubrec => Packet::Pubrec(Self::decode_pubrec(payload)?),
            PacketType::Pubrel => Packet::Pubrel(Self::decode_pubrel(payload)?),
            PacketType::Pubcomp => Packet::Pubcomp(Self::decode_pubcomp(payload)?),
            PacketType::Subscribe => Packet::Subscribe(Self::decode_subscribe(payload)?),
            PacketType::Suback => Packet::Suback(Self::decode_suback(payload)?),
            PacketType::Unsubscribe => Packet::Unsubscribe(Self::decode_unsubscribe(payload)?),
            PacketType::Unsuback => Packet::Unsuback(Self::decode_unsuback(payload)?),
            PacketType::Pingreq => Packet::Pingreq,
            PacketType::Pingresp => Packet::Pingresp,
            PacketType::Disconnect => Packet::Disconnect,
        };

        Ok(Some((packet, total_len)))
    }

    /// Decode variable-length remaining length field
    fn decode_remaining_length(buffer: &[u8]) -> ProtocolResult<(usize, usize)> {
        let mut multiplier = 1usize;
        let mut value = 0usize;
        let mut pos = 0;

        loop {
            if pos >= buffer.len() {
                return Err(ProtocolError::IncompletePacket {
                    expected: pos + 1,
                    got: buffer.len(),
                });
            }

            let byte = buffer[pos];
            value += (byte & 0x7F) as usize * multiplier;

            if multiplier > 128 * 128 * 128 {
                return Err(ProtocolError::InvalidRemainingLength);
            }

            multiplier *= 128;
            pos += 1;

            if byte & 0x80 == 0 {
                break;
            }
        }

        Ok((value, pos))
    }

    /// Decode a UTF-8 string with length prefix
    fn decode_string(cursor: &mut Cursor<&[u8]>) -> ProtocolResult<String> {
        let mut len_buf = [0u8; 2];
        cursor.read_exact(&mut len_buf)?;
        let len = u16::from_be_bytes(len_buf) as usize;

        let mut str_buf = vec![0u8; len];
        cursor.read_exact(&mut str_buf)?;

        String::from_utf8(str_buf).map_err(|_| ProtocolError::InvalidUtf8)
    }

    /// Decode binary data with length prefix
    fn decode_binary(cursor: &mut Cursor<&[u8]>) -> ProtocolResult<Vec<u8>> {
        let mut len_buf = [0u8; 2];
        cursor.read_exact(&mut len_buf)?;
        let len = u16::from_be_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        cursor.read_exact(&mut data)?;
        Ok(data)
    }

    /// Decode u16 from cursor
    fn decode_u16(cursor: &mut Cursor<&[u8]>) -> ProtocolResult<u16> {
        let mut buf = [0u8; 2];
        cursor.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn decode_connect(payload: &[u8]) -> ProtocolResult<ConnectPacket> {
        let mut cursor = Cursor::new(payload);

        // Protocol name
        let protocol_name = Self::decode_string(&mut cursor)?;
        if protocol_name != "MQTT" && protocol_name != "MQIsdp" {
            return Err(ProtocolError::InvalidProtocolName(protocol_name));
        }

        // Protocol level
        let mut level_buf = [0u8; 1];
        cursor.read_exact(&mut level_buf)?;
        let protocol_level = level_buf[0];

        // For MQTT 3.1.1, protocol level should be 4
        // For MQTT 3.1, protocol level should be 3
        if protocol_level != 4 && protocol_level != 3 {
            return Err(ProtocolError::InvalidProtocolVersion(protocol_level));
        }

        // Connect flags
        let mut flags_buf = [0u8; 1];
        cursor.read_exact(&mut flags_buf)?;
        let flags = flags_buf[0];

        let clean_session = (flags & 0x02) != 0;
        let will_flag = (flags & 0x04) != 0;
        let will_qos = QoS::try_from((flags >> 3) & 0x03)?;
        let will_retain = (flags & 0x20) != 0;
        let password_flag = (flags & 0x40) != 0;
        let username_flag = (flags & 0x80) != 0;

        // Reserved bit must be 0
        if flags & 0x01 != 0 {
            return Err(ProtocolError::InvalidConnectFlags);
        }

        // Keep alive
        let keep_alive = Self::decode_u16(&mut cursor)?;

        // Client ID
        let client_id = Self::decode_string(&mut cursor)?;

        // Will
        let will = if will_flag {
            let topic = Self::decode_string(&mut cursor)?;
            let message = Self::decode_binary(&mut cursor)?;
            Some(Will {
                topic,
                message,
                qos: will_qos,
                retain: will_retain,
            })
        } else {
            None
        };

        // Username
        let username = if username_flag {
            Some(Self::decode_string(&mut cursor)?)
        } else {
            None
        };

        // Password
        let password = if password_flag {
            Some(Self::decode_binary(&mut cursor)?)
        } else {
            None
        };

        Ok(ConnectPacket {
            protocol_name,
            protocol_level,
            clean_session,
            keep_alive,
            client_id,
            will,
            username,
            password,
        })
    }

    fn decode_connack(payload: &[u8]) -> ProtocolResult<ConnackPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }

        let session_present = (payload[0] & 0x01) != 0;
        let return_code = match payload[1] {
            0 => ConnackCode::Accepted,
            1 => ConnackCode::UnacceptableProtocolVersion,
            2 => ConnackCode::IdentifierRejected,
            3 => ConnackCode::ServerUnavailable,
            4 => ConnackCode::BadUsernamePassword,
            5 => ConnackCode::NotAuthorized,
            _ => return Err(ProtocolError::MalformedPacket),
        };

        Ok(ConnackPacket {
            session_present,
            return_code,
        })
    }

    fn decode_publish(flags: u8, payload: &[u8]) -> ProtocolResult<PublishPacket> {
        let dup = (flags & 0x08) != 0;
        let qos = QoS::try_from((flags >> 1) & 0x03)?;
        let retain = (flags & 0x01) != 0;

        let mut cursor = Cursor::new(payload);

        // Topic name
        let topic = Self::decode_string(&mut cursor)?;

        // Packet ID (only for QoS > 0)
        let packet_id = if qos != QoS::AtMostOnce {
            Some(Self::decode_u16(&mut cursor)?)
        } else {
            None
        };

        // Remaining bytes are the payload
        let pos = cursor.position() as usize;
        let message_payload = payload[pos..].to_vec();

        Ok(PublishPacket {
            dup,
            qos,
            retain,
            topic,
            packet_id,
            payload: message_payload,
        })
    }

    fn decode_puback(payload: &[u8]) -> ProtocolResult<PubackPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }
        Ok(PubackPacket {
            packet_id: u16::from_be_bytes([payload[0], payload[1]]),
        })
    }

    fn decode_pubrec(payload: &[u8]) -> ProtocolResult<PubrecPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }
        Ok(PubrecPacket {
            packet_id: u16::from_be_bytes([payload[0], payload[1]]),
        })
    }

    fn decode_pubrel(payload: &[u8]) -> ProtocolResult<PubrelPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }
        Ok(PubrelPacket {
            packet_id: u16::from_be_bytes([payload[0], payload[1]]),
        })
    }

    fn decode_pubcomp(payload: &[u8]) -> ProtocolResult<PubcompPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }
        Ok(PubcompPacket {
            packet_id: u16::from_be_bytes([payload[0], payload[1]]),
        })
    }

    fn decode_subscribe(payload: &[u8]) -> ProtocolResult<SubscribePacket> {
        let mut cursor = Cursor::new(payload);

        let packet_id = Self::decode_u16(&mut cursor)?;
        let mut subscriptions = Vec::new();

        while (cursor.position() as usize) < payload.len() {
            let topic = Self::decode_string(&mut cursor)?;
            let mut qos_buf = [0u8; 1];
            cursor.read_exact(&mut qos_buf)?;
            let qos = QoS::try_from(qos_buf[0] & 0x03)?;
            subscriptions.push((topic, qos));
        }

        if subscriptions.is_empty() {
            return Err(ProtocolError::MalformedPacket);
        }

        Ok(SubscribePacket {
            packet_id,
            subscriptions,
        })
    }

    fn decode_suback(payload: &[u8]) -> ProtocolResult<SubackPacket> {
        if payload.len() < 3 {
            return Err(ProtocolError::MalformedPacket);
        }

        let packet_id = u16::from_be_bytes([payload[0], payload[1]]);
        let mut return_codes = Vec::new();

        for &byte in &payload[2..] {
            let code = match byte {
                0x00 => SubackReturnCode::SuccessQoS0,
                0x01 => SubackReturnCode::SuccessQoS1,
                0x02 => SubackReturnCode::SuccessQoS2,
                0x80 => SubackReturnCode::Failure,
                _ => return Err(ProtocolError::MalformedPacket),
            };
            return_codes.push(code);
        }

        Ok(SubackPacket {
            packet_id,
            return_codes,
        })
    }

    fn decode_unsubscribe(payload: &[u8]) -> ProtocolResult<UnsubscribePacket> {
        let mut cursor = Cursor::new(payload);

        let packet_id = Self::decode_u16(&mut cursor)?;
        let mut topics = Vec::new();

        while (cursor.position() as usize) < payload.len() {
            let topic = Self::decode_string(&mut cursor)?;
            topics.push(topic);
        }

        if topics.is_empty() {
            return Err(ProtocolError::MalformedPacket);
        }

        Ok(UnsubscribePacket { packet_id, topics })
    }

    fn decode_unsuback(payload: &[u8]) -> ProtocolResult<UnsubackPacket> {
        if payload.len() < 2 {
            return Err(ProtocolError::MalformedPacket);
        }
        Ok(UnsubackPacket {
            packet_id: u16::from_be_bytes([payload[0], payload[1]]),
        })
    }
}

/// Packet encoder for serializing MQTT packets to bytes
pub struct PacketEncoder;

impl PacketEncoder {
    /// Encode a packet to bytes
    pub fn encode(packet: &Packet) -> ProtocolResult<Vec<u8>> {
        match packet {
            Packet::Connect(p) => Self::encode_connect(p),
            Packet::Connack(p) => Self::encode_connack(p),
            Packet::Publish(p) => Self::encode_publish(p),
            Packet::Puback(p) => Self::encode_puback(p),
            Packet::Pubrec(p) => Self::encode_pubrec(p),
            Packet::Pubrel(p) => Self::encode_pubrel(p),
            Packet::Pubcomp(p) => Self::encode_pubcomp(p),
            Packet::Subscribe(p) => Self::encode_subscribe(p),
            Packet::Suback(p) => Self::encode_suback(p),
            Packet::Unsubscribe(p) => Self::encode_unsubscribe(p),
            Packet::Unsuback(p) => Self::encode_unsuback(p),
            Packet::Pingreq => Self::encode_pingreq(),
            Packet::Pingresp => Self::encode_pingresp(),
            Packet::Disconnect => Self::encode_disconnect(),
        }
    }

    /// Encode remaining length as variable-length integer
    fn encode_remaining_length(length: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let mut x = length;

        loop {
            let mut byte = (x % 128) as u8;
            x /= 128;
            if x > 0 {
                byte |= 0x80;
            }
            result.push(byte);
            if x == 0 {
                break;
            }
        }

        result
    }

    /// Encode a UTF-8 string with length prefix
    fn encode_string(s: &str) -> Vec<u8> {
        let bytes = s.as_bytes();
        let len = bytes.len() as u16;
        let mut result = Vec::with_capacity(2 + bytes.len());
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(bytes);
        result
    }

    /// Encode binary data with length prefix
    fn encode_binary(data: &[u8]) -> Vec<u8> {
        let len = data.len() as u16;
        let mut result = Vec::with_capacity(2 + data.len());
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(data);
        result
    }

    fn encode_connect(packet: &ConnectPacket) -> ProtocolResult<Vec<u8>> {
        let mut payload = Vec::new();

        // Protocol name
        payload.extend(Self::encode_string(&packet.protocol_name));

        // Protocol level
        payload.push(packet.protocol_level);

        // Connect flags
        let mut flags = 0u8;
        if packet.clean_session {
            flags |= 0x02;
        }
        if let Some(ref will) = packet.will {
            flags |= 0x04; // Will flag
            flags |= (will.qos as u8) << 3;
            if will.retain {
                flags |= 0x20;
            }
        }
        if packet.password.is_some() {
            flags |= 0x40;
        }
        if packet.username.is_some() {
            flags |= 0x80;
        }
        payload.push(flags);

        // Keep alive
        payload.extend_from_slice(&packet.keep_alive.to_be_bytes());

        // Client ID
        payload.extend(Self::encode_string(&packet.client_id));

        // Will
        if let Some(ref will) = packet.will {
            payload.extend(Self::encode_string(&will.topic));
            payload.extend(Self::encode_binary(&will.message));
        }

        // Username
        if let Some(ref username) = packet.username {
            payload.extend(Self::encode_string(username));
        }

        // Password
        if let Some(ref password) = packet.password {
            payload.extend(Self::encode_binary(password));
        }

        // Build final packet
        let mut result = Vec::new();
        result.push(0x10); // CONNECT packet type
        result.extend(Self::encode_remaining_length(payload.len()));
        result.extend(payload);

        Ok(result)
    }

    fn encode_connack(packet: &ConnackPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0x20); // CONNACK packet type
        result.push(0x02); // Remaining length

        let ack_flags = if packet.session_present { 0x01 } else { 0x00 };
        result.push(ack_flags);
        result.push(packet.return_code as u8);

        Ok(result)
    }

    fn encode_publish(packet: &PublishPacket) -> ProtocolResult<Vec<u8>> {
        let mut payload = Vec::new();

        // Topic
        payload.extend(Self::encode_string(&packet.topic));

        // Packet ID (only for QoS > 0)
        if let Some(packet_id) = packet.packet_id {
            payload.extend_from_slice(&packet_id.to_be_bytes());
        }

        // Payload
        payload.extend_from_slice(&packet.payload);

        // Build fixed header
        let mut first_byte = 0x30u8; // PUBLISH packet type
        if packet.dup {
            first_byte |= 0x08;
        }
        first_byte |= (packet.qos as u8) << 1;
        if packet.retain {
            first_byte |= 0x01;
        }

        let mut result = Vec::new();
        result.push(first_byte);
        result.extend(Self::encode_remaining_length(payload.len()));
        result.extend(payload);

        Ok(result)
    }

    fn encode_puback(packet: &PubackPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0x40); // PUBACK packet type
        result.push(0x02); // Remaining length
        result.extend_from_slice(&packet.packet_id.to_be_bytes());
        Ok(result)
    }

    fn encode_pubrec(packet: &PubrecPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0x50); // PUBREC packet type
        result.push(0x02); // Remaining length
        result.extend_from_slice(&packet.packet_id.to_be_bytes());
        Ok(result)
    }

    fn encode_pubrel(packet: &PubrelPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0x62); // PUBREL packet type (with required flags)
        result.push(0x02); // Remaining length
        result.extend_from_slice(&packet.packet_id.to_be_bytes());
        Ok(result)
    }

    fn encode_pubcomp(packet: &PubcompPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0x70); // PUBCOMP packet type
        result.push(0x02); // Remaining length
        result.extend_from_slice(&packet.packet_id.to_be_bytes());
        Ok(result)
    }

    fn encode_subscribe(packet: &SubscribePacket) -> ProtocolResult<Vec<u8>> {
        let mut payload = Vec::new();

        // Packet ID
        payload.extend_from_slice(&packet.packet_id.to_be_bytes());

        // Topic filters
        for (topic, qos) in &packet.subscriptions {
            payload.extend(Self::encode_string(topic));
            payload.push(*qos as u8);
        }

        let mut result = Vec::new();
        result.push(0x82); // SUBSCRIBE packet type (with required flags)
        result.extend(Self::encode_remaining_length(payload.len()));
        result.extend(payload);

        Ok(result)
    }

    fn encode_suback(packet: &SubackPacket) -> ProtocolResult<Vec<u8>> {
        let mut payload = Vec::new();

        // Packet ID
        payload.extend_from_slice(&packet.packet_id.to_be_bytes());

        // Return codes
        for code in &packet.return_codes {
            payload.push((*code).into());
        }

        let mut result = Vec::new();
        result.push(0x90); // SUBACK packet type
        result.extend(Self::encode_remaining_length(payload.len()));
        result.extend(payload);

        Ok(result)
    }

    fn encode_unsubscribe(packet: &UnsubscribePacket) -> ProtocolResult<Vec<u8>> {
        let mut payload = Vec::new();

        // Packet ID
        payload.extend_from_slice(&packet.packet_id.to_be_bytes());

        // Topic filters
        for topic in &packet.topics {
            payload.extend(Self::encode_string(topic));
        }

        let mut result = Vec::new();
        result.push(0xA2); // UNSUBSCRIBE packet type (with required flags)
        result.extend(Self::encode_remaining_length(payload.len()));
        result.extend(payload);

        Ok(result)
    }

    fn encode_unsuback(packet: &UnsubackPacket) -> ProtocolResult<Vec<u8>> {
        let mut result = Vec::new();
        result.push(0xB0); // UNSUBACK packet type
        result.push(0x02); // Remaining length
        result.extend_from_slice(&packet.packet_id.to_be_bytes());
        Ok(result)
    }

    fn encode_pingreq() -> ProtocolResult<Vec<u8>> {
        Ok(vec![0xC0, 0x00]) // PINGREQ packet
    }

    fn encode_pingresp() -> ProtocolResult<Vec<u8>> {
        Ok(vec![0xD0, 0x00]) // PINGRESP packet
    }

    fn encode_disconnect() -> ProtocolResult<Vec<u8>> {
        Ok(vec![0xE0, 0x00]) // DISCONNECT packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type_from_u8() {
        assert_eq!(PacketType::try_from(1).unwrap(), PacketType::Connect);
        assert_eq!(PacketType::try_from(2).unwrap(), PacketType::Connack);
        assert_eq!(PacketType::try_from(3).unwrap(), PacketType::Publish);
        assert!(PacketType::try_from(0).is_err());
        assert!(PacketType::try_from(15).is_err());
    }

    #[test]
    fn test_qos_from_u8() {
        assert_eq!(QoS::try_from(0).unwrap(), QoS::AtMostOnce);
        assert_eq!(QoS::try_from(1).unwrap(), QoS::AtLeastOnce);
        assert_eq!(QoS::try_from(2).unwrap(), QoS::ExactlyOnce);
        assert!(QoS::try_from(3).is_err());
    }

    #[test]
    fn test_encode_decode_connect() {
        let connect = ConnectPacket {
            protocol_name: "MQTT".to_string(),
            protocol_level: 4,
            clean_session: true,
            keep_alive: 60,
            client_id: "test-client".to_string(),
            will: None,
            username: None,
            password: None,
        };

        let encoded = PacketEncoder::encode(&Packet::Connect(connect.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Connect(decoded_connect) = decoded {
            assert_eq!(decoded_connect.protocol_name, connect.protocol_name);
            assert_eq!(decoded_connect.protocol_level, connect.protocol_level);
            assert_eq!(decoded_connect.clean_session, connect.clean_session);
            assert_eq!(decoded_connect.keep_alive, connect.keep_alive);
            assert_eq!(decoded_connect.client_id, connect.client_id);
        } else {
            panic!("Expected Connect packet");
        }
    }

    #[test]
    fn test_encode_decode_connack() {
        let connack = ConnackPacket {
            session_present: false,
            return_code: ConnackCode::Accepted,
        };

        let encoded = PacketEncoder::encode(&Packet::Connack(connack.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Connack(decoded_connack) = decoded {
            assert_eq!(decoded_connack.session_present, connack.session_present);
            assert_eq!(decoded_connack.return_code, connack.return_code);
        } else {
            panic!("Expected Connack packet");
        }
    }

    #[test]
    fn test_encode_decode_publish_qos0() {
        let publish = PublishPacket {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: "test/topic".to_string(),
            packet_id: None,
            payload: b"Hello, MQTT!".to_vec(),
        };

        let encoded = PacketEncoder::encode(&Packet::Publish(publish.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Publish(decoded_publish) = decoded {
            assert_eq!(decoded_publish.topic, publish.topic);
            assert_eq!(decoded_publish.payload, publish.payload);
            assert_eq!(decoded_publish.qos, publish.qos);
        } else {
            panic!("Expected Publish packet");
        }
    }

    #[test]
    fn test_encode_decode_publish_qos1() {
        let publish = PublishPacket {
            dup: false,
            qos: QoS::AtLeastOnce,
            retain: true,
            topic: "sensor/temp".to_string(),
            packet_id: Some(1234),
            payload: b"25.5".to_vec(),
        };

        let encoded = PacketEncoder::encode(&Packet::Publish(publish.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Publish(decoded_publish) = decoded {
            assert_eq!(decoded_publish.topic, publish.topic);
            assert_eq!(decoded_publish.packet_id, publish.packet_id);
            assert_eq!(decoded_publish.retain, publish.retain);
        } else {
            panic!("Expected Publish packet");
        }
    }

    #[test]
    fn test_encode_decode_subscribe() {
        let subscribe = SubscribePacket {
            packet_id: 100,
            subscriptions: vec![
                ("topic/a".to_string(), QoS::AtMostOnce),
                ("topic/b/#".to_string(), QoS::AtLeastOnce),
            ],
        };

        let encoded = PacketEncoder::encode(&Packet::Subscribe(subscribe.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Subscribe(decoded_sub) = decoded {
            assert_eq!(decoded_sub.packet_id, subscribe.packet_id);
            assert_eq!(decoded_sub.subscriptions.len(), 2);
            assert_eq!(decoded_sub.subscriptions[0].0, "topic/a");
            assert_eq!(decoded_sub.subscriptions[1].1, QoS::AtLeastOnce);
        } else {
            panic!("Expected Subscribe packet");
        }
    }

    #[test]
    fn test_encode_decode_suback() {
        let suback = SubackPacket {
            packet_id: 100,
            return_codes: vec![SubackReturnCode::SuccessQoS0, SubackReturnCode::SuccessQoS1],
        };

        let encoded = PacketEncoder::encode(&Packet::Suback(suback.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Suback(decoded_suback) = decoded {
            assert_eq!(decoded_suback.packet_id, suback.packet_id);
            assert_eq!(decoded_suback.return_codes.len(), 2);
        } else {
            panic!("Expected Suback packet");
        }
    }

    #[test]
    fn test_encode_decode_unsubscribe() {
        let unsubscribe = UnsubscribePacket {
            packet_id: 200,
            topics: vec!["topic/a".to_string(), "topic/b".to_string()],
        };

        let encoded = PacketEncoder::encode(&Packet::Unsubscribe(unsubscribe.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Unsubscribe(decoded_unsub) = decoded {
            assert_eq!(decoded_unsub.packet_id, unsubscribe.packet_id);
            assert_eq!(decoded_unsub.topics.len(), 2);
        } else {
            panic!("Expected Unsubscribe packet");
        }
    }

    #[test]
    fn test_encode_decode_pingreq() {
        let encoded = PacketEncoder::encode(&Packet::Pingreq).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        assert!(matches!(decoded, Packet::Pingreq));
    }

    #[test]
    fn test_encode_decode_pingresp() {
        let encoded = PacketEncoder::encode(&Packet::Pingresp).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        assert!(matches!(decoded, Packet::Pingresp));
    }

    #[test]
    fn test_encode_decode_disconnect() {
        let encoded = PacketEncoder::encode(&Packet::Disconnect).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        assert!(matches!(decoded, Packet::Disconnect));
    }

    #[test]
    fn test_incomplete_packet() {
        let partial = vec![0x10, 0x0A]; // CONNECT header, but no payload
        let result = PacketDecoder::decode(&partial);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_remaining_length_encoding() {
        // Test various lengths
        assert_eq!(PacketEncoder::encode_remaining_length(0), vec![0x00]);
        assert_eq!(PacketEncoder::encode_remaining_length(127), vec![0x7F]);
        assert_eq!(PacketEncoder::encode_remaining_length(128), vec![0x80, 0x01]);
        assert_eq!(PacketEncoder::encode_remaining_length(16383), vec![0xFF, 0x7F]);
        assert_eq!(PacketEncoder::encode_remaining_length(16384), vec![0x80, 0x80, 0x01]);
    }

    #[test]
    fn test_connect_with_credentials() {
        let connect = ConnectPacket {
            protocol_name: "MQTT".to_string(),
            protocol_level: 4,
            clean_session: false,
            keep_alive: 120,
            client_id: "secure-client".to_string(),
            will: None,
            username: Some("user".to_string()),
            password: Some(b"pass".to_vec()),
        };

        let encoded = PacketEncoder::encode(&Packet::Connect(connect.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Connect(decoded_connect) = decoded {
            assert_eq!(decoded_connect.username, Some("user".to_string()));
            assert_eq!(decoded_connect.password, Some(b"pass".to_vec()));
            assert!(!decoded_connect.clean_session);
        } else {
            panic!("Expected Connect packet");
        }
    }

    #[test]
    fn test_connect_with_will() {
        let connect = ConnectPacket {
            protocol_name: "MQTT".to_string(),
            protocol_level: 4,
            clean_session: true,
            keep_alive: 60,
            client_id: "will-client".to_string(),
            will: Some(Will {
                topic: "last/will".to_string(),
                message: b"goodbye".to_vec(),
                qos: QoS::AtLeastOnce,
                retain: true,
            }),
            username: None,
            password: None,
        };

        let encoded = PacketEncoder::encode(&Packet::Connect(connect.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Connect(decoded_connect) = decoded {
            let will = decoded_connect.will.unwrap();
            assert_eq!(will.topic, "last/will");
            assert_eq!(will.message, b"goodbye");
            assert_eq!(will.qos, QoS::AtLeastOnce);
            assert!(will.retain);
        } else {
            panic!("Expected Connect packet");
        }
    }

    #[test]
    fn test_puback_roundtrip() {
        let puback = PubackPacket { packet_id: 12345 };
        let encoded = PacketEncoder::encode(&Packet::Puback(puback.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Puback(decoded_puback) = decoded {
            assert_eq!(decoded_puback.packet_id, puback.packet_id);
        } else {
            panic!("Expected Puback packet");
        }
    }

    #[test]
    fn test_qos2_handshake_packets() {
        // PUBREC
        let pubrec = PubrecPacket { packet_id: 1000 };
        let encoded = PacketEncoder::encode(&Packet::Pubrec(pubrec.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        if let Packet::Pubrec(d) = decoded {
            assert_eq!(d.packet_id, 1000);
        } else {
            panic!("Expected Pubrec");
        }

        // PUBREL
        let pubrel = PubrelPacket { packet_id: 1000 };
        let encoded = PacketEncoder::encode(&Packet::Pubrel(pubrel.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        if let Packet::Pubrel(d) = decoded {
            assert_eq!(d.packet_id, 1000);
        } else {
            panic!("Expected Pubrel");
        }

        // PUBCOMP
        let pubcomp = PubcompPacket { packet_id: 1000 };
        let encoded = PacketEncoder::encode(&Packet::Pubcomp(pubcomp.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();
        if let Packet::Pubcomp(d) = decoded {
            assert_eq!(d.packet_id, 1000);
        } else {
            panic!("Expected Pubcomp");
        }
    }

    #[test]
    fn test_unsuback_roundtrip() {
        let unsuback = UnsubackPacket { packet_id: 999 };
        let encoded = PacketEncoder::encode(&Packet::Unsuback(unsuback.clone())).unwrap();
        let (decoded, _) = PacketDecoder::decode(&encoded).unwrap().unwrap();

        if let Packet::Unsuback(decoded_unsuback) = decoded {
            assert_eq!(decoded_unsuback.packet_id, unsuback.packet_id);
        } else {
            panic!("Expected Unsuback packet");
        }
    }
}
