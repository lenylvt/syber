//! H.264 software decoder via openh264.
//! Outputs RGBA frames ready for egui texture upload.

use anyhow::{Context, Result};
use openh264::{
    decoder::Decoder,
    formats::YUVSource,
};

pub struct VideoDecoder {
    decoder: Decoder,
}

pub struct DecodedFrame {
    pub width:  u32,
    pub height: u32,
    pub rgba:   Vec<u8>,
}

impl VideoDecoder {
    pub fn new() -> Result<Self> {
        let decoder = Decoder::new().context("create H.264 decoder")?;
        Ok(Self { decoder })
    }

    /// Decode one or more NAL units. Returns None if the decoder is buffering.
    pub fn decode_nal(&mut self, nal: &[u8]) -> Result<Option<DecodedFrame>> {
        let result = self.decoder.decode(nal).context("decode nal")?;
        let Some(yuv) = result else {
            return Ok(None);
        };

        // dimensions() returns (width, height) from YUVSource trait
        let (w, h) = yuv.dimensions();
        let w = w as u32;
        let h = h as u32;

        // Write directly to RGBA buffer (openh264 sets alpha=255)
        let rgba_len = yuv.estimate_rgba_u8_size();
        let mut rgba = vec![0u8; rgba_len];
        yuv.write_rgba8(&mut rgba);

        Ok(Some(DecodedFrame { width: w, height: h, rgba }))
    }
}
