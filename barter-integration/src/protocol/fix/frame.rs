//! Deterministic FIX frame boundary detection over a byte stream.
//!
//! A FIX message (`8=...<SOH>9=NNN<SOH>...body...10=NNN<SOH>`) carries its own
//! length in the **BodyLength** (tag 9) field: the number of bytes between the
//! SOH following tag 9 and the SOH preceding the CheckSum (tag 10) field. This
//! makes every frame boundary fully determined by the first two header fields,
//! with no reliance on message contents.
//!
//! [`FrameAccumulator`] is a **pure, stateful** scanner: bytes are pushed in
//! arbitrary chunk sizes (TCP segments rarely align to message boundaries) and
//! complete frames are extracted in order. It performs no I/O, so every
//! behavior is unit-testable and bit-reproducible — the same philosophy as the
//! `fix-codec` crate it sits alongside.
//!
//! Frame extraction only locates boundaries using BodyLength. **Integrity**
//! (declared BodyLength/CheckSum match the actual bytes) is validated
//! downstream by [`fix_codec::decode`]; a malformed or truncated frame here is
//! surfaced as a [`FrameError`] so the caller can resync or reconnect.

use bytes::Bytes;

/// Maximum frame length the accumulator will accept (1 MiB). Guards against
/// unbounded buffer growth from a malicious or broken peer. FIX session
/// messages (logon/heartbeat/order/execution report) are orders of magnitude
/// smaller.
pub const MAX_FRAME_LEN: usize = 1 << 20;

/// The wire length of a canonical CheckSum field: `10=` + 3 digits + SOH.
const CHECKSUM_FIELD_LEN: usize = 7;

/// Error emitted when a byte buffer cannot be split into a valid FIX frame.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// The buffered bytes do not begin with a BeginString (`8=`) field.
    ///
    /// FIX session framing is deterministic; a stream that does not start with
    /// `8=` is not message-aligned (or is garbage) and requires resync.
    #[error("buffered bytes do not begin with a FIX BeginString (tag 8) field")]
    NotBeginString,

    /// Missing or malformed BodyLength (`9=`) field after BeginString.
    #[error("missing or malformed BodyLength (tag 9) field after BeginString")]
    InvalidBodyLength,

    /// The header (BeginString + BodyLength fields) exceeded [`MAX_FRAME_LEN`]
    /// without terminating.
    #[error("FIX header exceeds maximum size: {len} bytes")]
    HeaderTooLarge { len: usize },

    /// The declared BodyLength implies a frame exceeding [`MAX_FRAME_LEN`].
    #[error("declared FIX frame length exceeds maximum: {len} bytes")]
    FrameTooLarge { len: usize },
}

/// Deterministic FIX frame boundary detector.
///
/// Bytes are appended with [`push`](Self::push) and complete frames are pulled
/// one at a time with [`next_frame`](Self::next_frame), which returns:
/// - `Ok(Some(frame))` — one complete frame was extracted;
/// - `Ok(None)` — the buffer holds a prefix of a frame, more bytes required;
/// - `Err(error)` — the buffer is not a valid FIX frame prefix.
#[derive(Debug, Clone)]
pub struct FrameAccumulator {
    buffer: Vec<u8>,
    max_frame_len: usize,
}

impl Default for FrameAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameAccumulator {
    /// Construct an empty accumulator with the default [`MAX_FRAME_LEN`] cap.
    pub fn new() -> Self {
        Self::with_max_frame_len(MAX_FRAME_LEN)
    }

    /// Construct an empty accumulator with a custom frame length cap.
    pub fn with_max_frame_len(max_frame_len: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(4096),
            max_frame_len,
        }
    }

    /// Append a chunk of raw bytes read from the wire.
    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    /// Extract the next complete frame from the buffer, if one is available.
    ///
    /// Returns `Ok(None)` when the buffer only contains a prefix of a frame and
    /// more bytes must be pushed before a frame can be emitted. On `Err`, the
    /// buffer is left unchanged; call [`reset`](Self::reset) to drop it.
    pub fn next_frame(&mut self) -> Result<Option<Bytes>, FrameError> {
        let buffer = &self.buffer;

        // Not enough bytes to hold a BeginString field prefix ("8=").
        if buffer.len() < 2 {
            return Ok(None);
        }
        if &buffer[0..2] != b"8=" {
            return Err(FrameError::NotBeginString);
        }

        // Locate the SOH terminating the BeginString field.
        let Some(end_begin_string) = find_byte(buffer, 2, 0x01) else {
            if buffer.len() > self.max_frame_len {
                return Err(FrameError::HeaderTooLarge { len: buffer.len() });
            }
            return Ok(None);
        };

        // The next field must be BodyLength: "9=NNN" terminated by SOH.
        let body_length_field = end_begin_string + 1;
        if buffer.len() < body_length_field + 3 {
            return Ok(None);
        }
        if &buffer[body_length_field..body_length_field + 2] != b"9=" {
            return Err(FrameError::InvalidBodyLength);
        }

        let Some(end_body_length) = find_byte(buffer, body_length_field + 2, 0x01) else {
            if buffer.len() > self.max_frame_len {
                return Err(FrameError::HeaderTooLarge { len: buffer.len() });
            }
            return Ok(None);
        };

        // BodyLength digits; leading zeros are accepted (venues such as Binance
        // zero-pad BodyLength to a fixed width, e.g. `9=0000113`).
        let digits = &buffer[body_length_field + 2..end_body_length];
        let Some(body_length) = parse_usize(digits) else {
            return Err(FrameError::InvalidBodyLength);
        };

        // Frame = BodyLength body + the CheckSum field (`10=NNN<SOH>`).
        let frame_len = match (end_body_length + 1).checked_add(body_length) {
            Some(frame_len) => frame_len + CHECKSUM_FIELD_LEN,
            None => return Err(FrameError::FrameTooLarge { len: usize::MAX }),
        };
        if frame_len > self.max_frame_len {
            return Err(FrameError::FrameTooLarge { len: frame_len });
        }
        if buffer.len() < frame_len {
            return Ok(None);
        }

        let frame = Bytes::copy_from_slice(&buffer[..frame_len]);
        self.buffer.drain(..frame_len);
        Ok(Some(frame))
    }

    /// Drop all buffered bytes, discarding any partial frame.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Number of bytes currently buffered.
    pub fn pending(&self) -> usize {
        self.buffer.len()
    }

    /// True when no bytes are buffered.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Find the first occurrence of `byte` at or after `from`, if any.
fn find_byte(buffer: &[u8], from: usize, byte: u8) -> Option<usize> {
    buffer[from..]
        .iter()
        .position(|&b| b == byte)
        .map(|i| i + from)
}

/// Parse an ASCII digit string into a `usize`, accepting leading zeros.
fn parse_usize(digits: &[u8]) -> Option<usize> {
    if digits.is_empty() {
        return None;
    }
    let mut value: usize = 0;
    for &b in digits {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(usize::from(b - b'0'))?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fix_codec::tags::{BEGIN_STRING, MSG_SEQ_NUM, MSG_TYPE, SENDER_COMP_ID, TARGET_COMP_ID};
    use fix_codec::{Message, encode};

    /// Encode a minimal FIX message with a tag 11 (ClOrdID) payload.
    fn frame(cl_ord_id: &str) -> Vec<u8> {
        let mut message = Message::new();
        message.push(BEGIN_STRING, "FIX.4.4");
        message.push(MSG_TYPE, "D");
        message.push(MSG_SEQ_NUM, "1");
        message.push(SENDER_COMP_ID, "CLIENT1");
        message.push(TARGET_COMP_ID, "EXECUTOR");
        message.push(11, cl_ord_id);
        encode(&message).unwrap()
    }

    fn drain_frames(accumulator: &mut FrameAccumulator) -> Vec<Bytes> {
        let mut frames = Vec::new();
        while let Some(frame) = accumulator.next_frame().unwrap() {
            frames.push(frame);
        }
        frames
    }

    #[test]
    fn test_frame_single_message() {
        let mut accumulator = FrameAccumulator::new();
        let bytes = frame("1");
        accumulator.push(&bytes);

        let frames = drain_frames(&mut accumulator);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], Bytes::from(bytes.clone()));
        assert!(accumulator.is_empty());
    }

    #[test]
    fn test_frame_split_across_every_chunk_size() {
        let bytes = frame("2");
        for chunk_size in 1..bytes.len() {
            let mut accumulator = FrameAccumulator::new();
            for chunk in bytes.chunks(chunk_size) {
                assert_eq!(
                    accumulator.next_frame().unwrap(),
                    None,
                    "frame emitted before full message for chunk_size {chunk_size}"
                );
                accumulator.push(chunk);
            }
            let frames = drain_frames(&mut accumulator);
            assert_eq!(frames.len(), 1, "chunk_size {chunk_size}");
            assert_eq!(
                frames[0],
                Bytes::from(bytes.clone()),
                "chunk_size {chunk_size}"
            );
            assert!(accumulator.is_empty());
        }
    }

    #[test]
    fn test_frame_multiple_messages_one_chunk() {
        let mut accumulator = FrameAccumulator::new();
        let mut expected = Vec::new();
        let mut all = Vec::new();
        for i in 0..3 {
            let bytes = frame(&i.to_string());
            expected.push(Bytes::from(bytes.clone()));
            all.extend_from_slice(&bytes);
        }
        accumulator.push(&all);

        assert_eq!(drain_frames(&mut accumulator), expected);
        assert!(accumulator.is_empty());
    }

    #[test]
    fn test_frame_batched_messages() {
        let mut accumulator = FrameAccumulator::new();
        let mut expected = Vec::new();
        for i in 0..3 {
            let bytes = frame(&i.to_string());
            expected.push(Bytes::from(bytes.clone()));
            accumulator.push(&bytes);
        }
        assert_eq!(drain_frames(&mut accumulator), expected);
    }

    #[test]
    fn test_frame_incomplete_body_returns_none() {
        let mut accumulator = FrameAccumulator::new();
        let bytes = frame("3");
        accumulator.push(&bytes[..bytes.len() - 5]);
        assert_eq!(accumulator.next_frame().unwrap(), None);
        assert_eq!(accumulator.pending(), bytes.len() - 5);
    }

    #[test]
    fn test_frame_empty_buffer_is_incomplete() {
        // An empty buffer is a valid prefix of a frame (needs more bytes), not
        // an error.
        let mut accumulator = FrameAccumulator::new();
        assert_eq!(accumulator.next_frame().unwrap(), None);
        assert!(accumulator.is_empty());
    }

    #[test]
    fn test_frame_not_begin_string() {
        struct TestCase {
            input: &'static [u8],
        }
        let tests = vec![
            TestCase { input: b"9=" },
            TestCase {
                input: b"7=34\x0135=D\x0110=231\x01",
            },
            TestCase {
                input: b"8FIX.4.4\x01",
            },
        ];
        for (index, test) in tests.into_iter().enumerate() {
            let mut accumulator = FrameAccumulator::new();
            accumulator.push(test.input);
            match accumulator.next_frame() {
                Err(FrameError::NotBeginString) => {}
                actual => panic!("TC{index} failed: expected NotBeginString, got {actual:?}"),
            }
        }
    }

    #[test]
    fn test_frame_missing_or_malformed_body_length() {
        struct TestCase {
            input: &'static [u8],
        }
        let tests = vec![
            // Missing 9= field entirely
            TestCase {
                input: b"8=FIX.4.4\x0135=D\x0110=231\x01",
            },
            // Non-digit BodyLength
            TestCase {
                input: b"8=FIX.4.4\x019=abc\x0135=D\x0110=231\x01",
            },
            // Empty BodyLength
            TestCase {
                input: b"8=FIX.4.4\x019=\x0135=D\x0110=231\x01",
            },
            // Wrong tag after BeginString
            TestCase {
                input: b"8=FIX.4.4\x0134=49\x0135=D\x0110=231\x01",
            },
        ];
        for (index, test) in tests.into_iter().enumerate() {
            let mut accumulator = FrameAccumulator::new();
            accumulator.push(test.input);
            match accumulator.next_frame() {
                Err(FrameError::InvalidBodyLength) => {}
                actual => panic!("TC{index} failed: expected InvalidBodyLength, got {actual:?}"),
            }
        }
    }

    #[test]
    fn test_frame_zero_padded_body_length_accepted() {
        // Binance Spot FIX zero-pads BodyLength (e.g. `9=0000113`).
        let mut accumulator = FrameAccumulator::new();
        let bytes = frame("4");
        // Reconstruct the message with a zero-padded BodyLength by padding the
        // digits of the declared length to a fixed width.
        let padded = pad_body_length(&bytes);
        accumulator.push(&padded);
        let frames = drain_frames(&mut accumulator);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], Bytes::from(padded));
    }

    #[test]
    fn test_frame_reset_discards_partial_frame() {
        let mut accumulator = FrameAccumulator::new();
        let bytes = frame("5");
        accumulator.push(&bytes[..bytes.len() - 3]);
        assert!(accumulator.pending() > 0);
        accumulator.reset();
        assert!(accumulator.is_empty());
        // A fresh frame can be accumulated after reset.
        accumulator.push(&bytes);
        assert_eq!(drain_frames(&mut accumulator).len(), 1);
    }

    #[test]
    fn test_frame_declared_length_cap_enforced() {
        let mut accumulator = FrameAccumulator::with_max_frame_len(32);
        let bytes = frame("6");
        accumulator.push(&bytes);
        assert!(matches!(
            accumulator.next_frame(),
            Err(FrameError::FrameTooLarge { .. })
        ));
    }

    /// Replace the BodyLength digits in an encoded message with a fixed-width,
    /// zero-padded equivalent.
    fn pad_body_length(bytes: &[u8]) -> Vec<u8> {
        // Find the digits of tag 9 ("9=NNN<SOH>") and re-emit them zero-padded.
        let eq9 = bytes.windows(2).position(|w| w == b"9=").unwrap();
        let start = eq9 + 2;
        let end = start + bytes[start..].iter().position(|&b| b == 0x01).unwrap();
        let declared: String = String::from_utf8(bytes[start..end].to_vec()).unwrap();
        let padded = format!("{:0>6}", declared);

        let mut out = bytes[..start].to_vec();
        out.extend_from_slice(padded.as_bytes());
        out.extend_from_slice(&bytes[end..]);
        out
    }
}
