use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Result;

// ─── Video options ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
}

impl Default for VideoCodec {
    fn default() -> Self { Self::H264 }
}

impl VideoCodec {
    pub fn label(&self) -> &str {
        match self { Self::H264 => "H.264", Self::H265 => "H.265 (HEVC)" }
    }
    /// FourCC used in CodecPacketHeader.codec field
    pub fn codec_id(&self) -> u32 {
        match self {
            Self::H264 => 27,  // AV_CODEC_ID_H264
            Self::H265 => 173, // AV_CODEC_ID_HEVC
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoEncoder {
    Auto,
    Software,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
}

impl Default for VideoEncoder {
    fn default() -> Self { Self::Auto }
}

impl VideoEncoder {
    pub fn label(&self) -> &str {
        match self {
            Self::Auto     => "Auto (recommandé)",
            Self::Software => "Logiciel (CPU)",
            Self::Nvenc    => "NVENC (NVIDIA)",
            Self::Qsv      => "QuickSync (Intel)",
            Self::Amf      => "AMF (AMD)",
            Self::Vaapi    => "VA-API (Linux GPU)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoProtocolChoice {
    Reliable,
    GopStream,
    UnreliableFec,
}

impl Default for VideoProtocolChoice {
    fn default() -> Self { Self::Reliable }
}

impl VideoProtocolChoice {
    pub fn label(&self) -> &str {
        match self {
            Self::Reliable      => "Fiable (réseau parfait)",
            Self::GopStream     => "GOP Stream (recommandé)",
            Self::UnreliableFec => "Non-fiable + FEC (LAN rapide)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityPreset {
    Fast,    // Low bitrate, performance
    Good,    // Balanced
    Best,    // High quality
}

impl Default for QualityPreset {
    fn default() -> Self { Self::Good }
}

impl QualityPreset {
    pub fn label(&self) -> &str {
        match self {
            Self::Fast => "Rapide (faible latence)",
            Self::Good => "Équilibré",
            Self::Best => "Qualité maximale",
        }
    }
    pub fn bitrate_kbps(&self) -> u32 {
        match self { Self::Fast => 5_000, Self::Good => 10_000, Self::Best => 20_000 }
    }
    pub fn fps(&self) -> f32 {
        match self { Self::Fast => 60.0, Self::Good => 60.0, Self::Best => 60.0 }
    }
}

// ─── Server Config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    // Simple
    pub port:     u16,
    pub password: String,
    pub preset:   QualityPreset,

    // Advanced
    pub codec:            VideoCodec,
    pub encoder:          VideoEncoder,
    pub bitrate_kbps:     u32,
    pub fps:              f32,
    pub resolution_scale: f32,  // 0.25 → 1.0
    pub video_protocol:   VideoProtocolChoice,
    pub display_index:    usize,

    // Internal (not shown in UI)
    pub cert_pem:  String,
    pub key_pem:   String,
    pub cert_hash: String, // SHA-256 hex, shown to user
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port:             8080,
            password:         Self::generate_password(),
            preset:           QualityPreset::default(),
            codec:            VideoCodec::default(),
            encoder:          VideoEncoder::default(),
            bitrate_kbps:     10_000,
            fps:              60.0,
            resolution_scale: 1.0,
            video_protocol:   VideoProtocolChoice::default(),
            display_index:    0,
            cert_pem:         String::new(),
            key_pem:          String::new(),
            cert_hash:        String::new(),
        }
    }
}

impl ServerConfig {
    pub fn generate_password() -> String {
        use rand::Rng;
        let chars: Vec<char> = "ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789"
            .chars().collect();
        let mut rng = rand::thread_rng();
        (0..12).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
    }

    pub fn apply_preset(&mut self) {
        self.bitrate_kbps = self.preset.bitrate_kbps();
        self.fps = self.preset.fps();
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("syber")
            .join("server.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&data) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// ─── Client Config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentConnection {
    pub host:      String,
    pub port:      u16,
    pub cert_hash: String,
    pub last_used: u64, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub last_host:      String,
    pub last_port:      u16,
    pub last_cert_hash: String,
    pub recents:        Vec<RecentConnection>,

    // Input settings
    pub capture_keyboard: bool,
    pub capture_mouse:    bool,
    pub capture_gamepad:  bool,
    pub grab_cursor:      bool,
    pub hotkey_disconnect: String, // e.g. "ctrl+shift+q"

    // Display
    pub fullscreen_on_connect: bool,
    pub show_stats_overlay:    bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            last_host:             String::new(),
            last_port:             8080,
            last_cert_hash:        String::new(),
            recents:               Vec::new(),
            capture_keyboard:      true,
            capture_mouse:         true,
            capture_gamepad:       false,
            grab_cursor:           true,
            hotkey_disconnect:     "ctrl+shift+q".to_string(),
            fullscreen_on_connect: false,
            show_stats_overlay:    true,
        }
    }
}

impl ClientConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("syber")
            .join("client.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&data) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn add_recent(&mut self, host: &str, port: u16, cert_hash: &str) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.recents.retain(|r| !(r.host == host && r.port == port));
        self.recents.insert(0, RecentConnection {
            host:      host.to_string(),
            port,
            cert_hash: cert_hash.to_string(),
            last_used: ts,
        });
        self.recents.truncate(5);
    }
}
