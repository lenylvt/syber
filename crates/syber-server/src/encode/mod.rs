//! H.264 software encoder via openh264.
//! Produces AVPacket-compatible byte buffers for kyproto's VideoProtocol.

use anyhow::{Context, Result};
use bytes::Bytes;
use openh264::{
    encoder::{Encoder, EncoderConfig},
    formats::{RgbSliceU8, YUVBuffer},
    OpenH264API,
};

pub struct EncodedPacket {
    pub is_keyframe: bool,
    pub is_config:   bool,
    pub pts:         u64,
    pub data:        Bytes,
}

pub struct VideoEncoder {
    encoder: Encoder,
    pts:     u64,
    width:   u32,
    height:  u32,
    // Reusable YUV buffer to avoid repeated allocation
    yuv_buf: YUVBuffer,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, fps: f32, bitrate_kbps: u32) -> Result<Self> {
        let api    = OpenH264API::from_source();
        let config = EncoderConfig::new()
            .max_frame_rate(fps)
            .set_bitrate_bps(bitrate_kbps * 1000)
            .debug(false);

        let encoder = Encoder::with_api_config(api, config)
            .context("create H.264 encoder")?;

        let yuv_buf = YUVBuffer::new(width as usize, height as usize);

        Ok(Self { encoder, pts: 0, width, height, yuv_buf })
    }

    /// Encode one RGB frame (3 bytes/pixel, row-major). Returns 0 or more packets.
    pub fn encode_rgb(&mut self, rgb: &[u8]) -> Result<Vec<EncodedPacket>> {
        // Convert RGB → YUV420 in-place
        let rgb_src = RgbSliceU8::new(rgb, (self.width as usize, self.height as usize));
        self.yuv_buf.read_rgb8(rgb_src);

        let pts = self.pts;
        self.pts += 1;

        let bitstream = self.encoder.encode(&self.yuv_buf)
            .context("encode frame")?;

        let mut packets = Vec::new();

        // Get all NAL bytes
        let nal_bytes = bitstream.to_vec();
        if nal_bytes.is_empty() {
            return Ok(packets);
        }

        // Split into individual NAL units using openh264's helper
        // NAL start codes: 0x00 0x00 0x00 0x01 or 0x00 0x00 0x01
        // For simplicity, send the entire bitstream as one packet
        // and detect keyframe/config by NAL type of first unit
        let first_nal_type = find_first_nal_type(&nal_bytes);
        let is_config  = first_nal_type == 7 || first_nal_type == 8; // SPS/PPS
        let is_keyframe = first_nal_type == 5; // IDR

        packets.push(EncodedPacket {
            is_keyframe,
            is_config,
            pts,
            data: Bytes::from(nal_bytes),
        });

        Ok(packets)
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Find the NAL unit type of the first NAL unit in the bitstream.
/// Returns 0 if not found.
fn find_first_nal_type(data: &[u8]) -> u8 {
    let mut i = 0;
    while i + 4 < data.len() {
        // 4-byte start code: 0x00 0x00 0x00 0x01
        if data[i] == 0 && data[i+1] == 0 && data[i+2] == 0 && data[i+3] == 1 {
            if i + 4 < data.len() {
                return data[i+4] & 0x1F;
            }
        }
        // 3-byte start code: 0x00 0x00 0x01
        if data[i] == 0 && data[i+1] == 0 && data[i+2] == 1 {
            if i + 3 < data.len() {
                return data[i+3] & 0x1F;
            }
        }
        i += 1;
    }
    0
}
