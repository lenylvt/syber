//! Screen capture — wraps xcap 0.0.14 for cross-platform frame grabbing.

use anyhow::{Context, Result};
use xcap::Monitor;

pub struct CapturedFrame {
    pub width:  u32,
    pub height: u32,
    pub rgba:   Vec<u8>, // RGBA row-major
}

pub struct ScreenCapture {
    monitor_index: usize,
}

impl ScreenCapture {
    pub fn new(monitor_index: usize) -> Self {
        Self { monitor_index }
    }

    /// List available monitors: (index, name, width, height)
    pub fn list_monitors() -> Vec<(usize, String, u32, u32)> {
        Monitor::all()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(i, m)| {
                let name = format!("{} ({}×{})", m.name(), m.width(), m.height());
                (i, name, m.width(), m.height())
            })
            .collect()
    }

    /// Capture one frame, optionally scaled.
    pub fn capture(&self, scale: f32) -> Result<CapturedFrame> {
        let monitors = Monitor::all().context("list monitors")?;
        if monitors.is_empty() {
            anyhow::bail!("no monitors found");
        }
        let monitor = if self.monitor_index < monitors.len() {
            &monitors[self.monitor_index]
        } else {
            &monitors[0]
        };

        let img = monitor.capture_image().context("capture image")?;
        let w   = img.width();
        let h   = img.height();

        if (scale - 1.0).abs() < 0.01 {
            return Ok(CapturedFrame { width: w, height: h, rgba: img.into_raw() });
        }

        // Nearest-neighbour downscale
        let nw = ((w as f32 * scale) as u32).max(2);
        let nh = ((h as f32 * scale) as u32).max(2);

        let src = img.into_raw();
        let mut dst = vec![0u8; (nw * nh * 4) as usize];

        for dy in 0..nh {
            for dx in 0..nw {
                let sx = ((dx as f32 / nw as f32) * w as f32) as u32;
                let sy = ((dy as f32 / nh as f32) * h as f32) as u32;
                let si = ((sy * w + sx) * 4) as usize;
                let di = ((dy * nw + dx) * 4) as usize;
                if si + 4 <= src.len() && di + 4 <= dst.len() {
                    dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
                }
            }
        }

        Ok(CapturedFrame { width: nw, height: nh, rgba: dst })
    }
}

/// Convert RGBA → RGB (strip alpha channel, 3 bytes/pixel)
pub fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect()
}
