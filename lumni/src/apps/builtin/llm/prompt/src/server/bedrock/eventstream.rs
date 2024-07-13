use std::collections::HashMap;

use bytes::{Buf, Bytes};
use crc32fast::Hasher;

#[derive(Debug)]
pub struct EventStreamMessage {
    pub headers: HashMap<String, String>,
    pub payload: Option<Bytes>,
}

impl EventStreamMessage {
    pub fn from_bytes(
        mut buffer: Bytes,
    ) -> Result<(Self, Option<Bytes>), &'static str> {
        // Parse a single message from an EventStream buffer. This works similar as to what is described in the AWS documentation for the Amazon Transcribe Streaming API:
        //eprintln!("Raw buffer: {:?}", buffer);
        if buffer.remaining() < 16 {
            // Minimum size of a message
            return Err("Buffer too short");
        }

        // store pointer to start of message, so we can calculate final CRC32
        let message_excluding_final_crc =
            buffer.slice(..buffer.remaining() - 4).clone();

        // Copy the prelude part for CRC calculation before advancing the buffer
        let prelude_for_crc = buffer.slice(..8);

        // Read total length and headers length from the prelude
        let total_length = buffer.get_u32() as usize;
        let headers_length = buffer.get_u32() as usize;

        // Verify Prelude CRC, which comes after the prelude
        let prelude_crc = buffer.get_u32();
        if prelude_crc != calculate_crc32(&prelude_for_crc) {
            return Err("Prelude CRC mismatch");
        }

        // Ensure buffer has enough remaining bytes for the full message
        // Subtract size of prelude and prelude CRC
        if buffer.remaining() < total_length - 12 {
            return Err("Buffer doesn't contain full message");
        }

        // Parsing headers
        let headers_bytes = buffer.slice(..headers_length);
        buffer.advance(headers_length);
        let headers = parse_headers(headers_bytes);

        // Parsing message payload
        // Subtract 16 bytes to account for size of two CRCs and prelude
        let payload_length = total_length - headers_length - 16;
        // remaining bytes should be payload + message CRC
        if buffer.remaining() < payload_length + 4 {
            return Err("Insufficient data for payload and message CRC");
        }
        let payload = buffer.slice(..payload_length);
        buffer.advance(payload_length);

        if buffer.remaining() < 4 {
            return Err("Message CRC missing after payload");
        }
        let message_crc = buffer.get_u32();

        let remaining_bytes = if buffer.has_remaining() {
            Some(buffer)
        } else {
            None
        };

        if message_crc == calculate_crc32(&message_excluding_final_crc) {
            Ok((
                EventStreamMessage {
                    headers,
                    payload: Some(payload),
                },
                remaining_bytes,
            ))
        } else {
            Ok((
                EventStreamMessage {
                    headers,
                    payload: None,
                },
                remaining_bytes,
            ))
        }
    }
}

fn parse_headers(mut buffer: Bytes) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    while buffer.has_remaining() {
        if buffer.remaining() < 1 {
            break;
        }
        let name_len = buffer.get_u8() as usize;
        if buffer.remaining() < name_len {
            break;
        }
        let name =
            String::from_utf8_lossy(&buffer.slice(..name_len)).into_owned();
        buffer.advance(name_len);

        if buffer.remaining() < 1 {
            break;
        }
        let value_type = buffer.get_u8();
        let value = match value_type {
            7 => {
                // string type
                if buffer.remaining() < 2 {
                    break;
                }
                let value_len = buffer.get_u16() as usize;
                if buffer.remaining() < value_len {
                    break;
                }
                let value = String::from_utf8_lossy(&buffer.slice(..value_len))
                    .into_owned();
                buffer.advance(value_len);
                value
            }
            _ => {
                log::warn!("Unsupported header value type: {}", value_type);
                continue;
            }
        };

        headers.insert(name, value);
    }
    headers
}

fn calculate_crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}
