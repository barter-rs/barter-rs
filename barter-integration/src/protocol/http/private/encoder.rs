use base64::Engine;

/// Encodes bytes data.
pub trait Encoder {
    /// Encodes the bytes data into some `String` format.
    fn encode<Bytes>(&self, data: Bytes) -> String
    where
        Bytes: AsRef<[u8]>;
}

/// Encodes bytes data as a hex `String` using lowercase characters.
#[derive(Debug, Copy, Clone)]
pub struct HexEncoder;

impl Encoder for HexEncoder {
    fn encode<Bytes>(&self, data: Bytes) -> String
    where
        Bytes: AsRef<[u8]>,
    {
        hex::encode(data)
    }
}

/// Encodes bytes data as a base64 `String`.
#[derive(Debug, Copy, Clone)]
pub struct Base64Encoder;

impl Encoder for Base64Encoder {
    fn encode<Bytes>(&self, data: Bytes) -> String
    where
        Bytes: AsRef<[u8]>,
    {
        base64::engine::general_purpose::STANDARD.encode(data)
    }
}
