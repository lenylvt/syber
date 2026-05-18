//! Client session — connects to a Syber server using kyproto QUIC.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};

use kyproto::{
    ClientAuth,
    VideoProtocol,
    quinn::QuinnClientOptions,
    Connection,
};
use kymux_types::{
    av::AVPacket,
    input::InputPacket as WireInputPacket,
    ProtocolSend,
};

// Endpoint IDs — client connects to server's even IDs
const VIDEO_ENDPOINT_ID: u16 = 0;
const INPUT_ENDPOINT_ID: u16 = 2;

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ClientStats {
    pub fps:          f32,
    pub bitrate_kbps: f32,
    pub rtt_ms:       f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ClientStatus {
    fn default() -> Self { Self::Disconnected }
}

pub struct VideoFrame {
    pub width:  u32,
    pub height: u32,
    pub rgba:   Vec<u8>,
}

pub struct SharedClientState {
    pub status:       ClientStatus,
    pub stats:        ClientStats,
    pub latest_frame: Option<VideoFrame>,
    pub frame_dirty:  bool,
}

impl SharedClientState {
    pub fn new() -> Self {
        Self {
            status:       ClientStatus::Disconnected,
            stats:        ClientStats::default(),
            latest_frame: None,
            frame_dirty:  false,
        }
    }
}

pub type SharedClientStateArc = Arc<Mutex<SharedClientState>>;

// ── Session control ───────────────────────────────────────────────────────────

pub enum ClientCmd {
    Disconnect,
    SendInput(WireInputPacket),
}

pub struct ConnectParams {
    pub host:           String,
    pub port:           u16,
    pub password:       String,
    pub cert_hash:      String,
    pub video_protocol: VideoProtocol,
}

pub fn start_client(
    state:  SharedClientStateArc,
    params: ConnectParams,
    cmd_rx: mpsc::Receiver<ClientCmd>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            if let Err(e) = run_client(state.clone(), params, cmd_rx).await {
                error!("Client error: {e:#}");
                state.lock().unwrap().status = ClientStatus::Error(e.to_string());
            }
        });
    });
}

async fn run_client(
    state:   SharedClientStateArc,
    params:  ConnectParams,
    mut cmd_rx: mpsc::Receiver<ClientCmd>,
) -> Result<()> {
    state.lock().unwrap().status = ClientStatus::Connecting;

    let addr: SocketAddr = format!("{}:{}", params.host, params.port)
        .parse().context("invalid address")?;

    let quinn_opts = QuinnClientOptions {
        max_idle_timeout:    Some(Duration::from_secs(30)),
        keep_alive_interval: Some(Duration::from_secs(5)),
        certificate_hash:    if params.cert_hash.is_empty() {
            None
        } else {
            // Normalise: strip colons/spaces, lowercase
            // UI shows "B8:66:4A:..." but kynet expects "b8664a..."
            let clean = params.cert_hash
                .replace(':', "")
                .replace(' ', "")
                .to_lowercase();
            Some(clean)
        },
    };

    let auth = ClientAuth::new(&params.password)
        .map_err(|_| anyhow::anyhow!("password too long"))?;

    info!("Connecting to {addr}…");
    let conn = Connection::quinn_connect_with_auth(addr, "syber-server", None, &quinn_opts, &auth)
        .await.context("connect")?;
    info!("Connected");

    state.lock().unwrap().status = ClientStatus::Connected;

    // Connect endpoints (client connects to server's registered endpoints)
    let video_ep = conn
        .connect_video_endpoint(VIDEO_ENDPOINT_ID, params.video_protocol)
        .context("connect video ep")?;
    let input_ep = conn
        .connect_input_endpoint(INPUT_ENDPOINT_ID)
        .context("connect input ep")?;

    let (mut video_proto, mut input_proto) = tokio::try_join!(
        video_ep.ready(),
        input_ep.ready(),
    ).context("endpoints ready")?;

    debug!("Endpoints ready — streaming");

    let mut decoder     = crate::decode::VideoDecoder::new().context("decoder")?;
    let mut codec_seen  = false;
    let mut frames      = 0u64;
    let mut bytes_recv  = 0u64;
    let mut last_stats  = Instant::now();

    loop {
        tokio::select! {
            // ── Video receive ──────────────────────────────────────────────
            recv_result = video_proto.recv.recv() => {
                match recv_result {
                    Ok(Some(AVPacket::Codec(_))) => {
                        codec_seen = true;
                        debug!("Codec packet received");
                    }
                    Ok(Some(AVPacket::Media(media))) => {
                        if !codec_seen { continue; }
                        bytes_recv += media.payload.len() as u64;

                        match decoder.decode_nal(&media.payload) {
                            Ok(Some(frame)) => {
                                frames += 1;
                                let mut s = state.lock().unwrap();
                                s.latest_frame = Some(VideoFrame {
                                    width:  frame.width,
                                    height: frame.height,
                                    rgba:   frame.rgba,
                                });
                                s.frame_dirty = true;
                            }
                            Ok(None) => {} // decoder buffering
                            Err(e)   => warn!("Decode: {e}"),
                        }

                        // Stats every second
                        let elapsed = last_stats.elapsed();
                        if elapsed >= Duration::from_secs(1) {
                            let fps  = frames as f32 / elapsed.as_secs_f32();
                            let kbps = (bytes_recv as f32 * 8.0) / (elapsed.as_secs_f32() * 1000.0);
                            let rtt  = conn.connection_stats().await
                                .rtt.unwrap_or_default()
                                .as_secs_f32() * 1000.0;
                            {
                                let mut s = state.lock().unwrap();
                                s.stats.fps          = fps;
                                s.stats.bitrate_kbps = kbps;
                                s.stats.rtt_ms       = rtt;
                            }
                            frames     = 0;
                            bytes_recv = 0;
                            last_stats = Instant::now();
                        }
                    }
                    Ok(Some(AVPacket::Hole(_))) => {}
                    Ok(None) | Err(_) => {
                        info!("Video stream ended");
                        break;
                    }
                }
            }

            // ── Commands from UI (input packets + disconnect) ──────────────
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(ClientCmd::SendInput(pkt)) => {
                        if let Err(e) = input_proto.send.send(pkt).await {
                            warn!("Input send: {e:?}");
                        }
                    }
                    Some(ClientCmd::Disconnect) | None => {
                        info!("User disconnect");
                        conn.close();
                        break;
                    }
                }
            }
        }
    }

    state.lock().unwrap().status = ClientStatus::Disconnected;
    Ok(())
}
