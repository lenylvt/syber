//! Server session — manages one client connection using kyproto.
//! Architecture: kyproto CommonServer → accept → video + input endpoints.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};

use kyproto::{
    VideoProtocol,
    Server,  // trait: accept_with_auth, close, wait_idle
    common::{CommonServer, CommonServerOptions},
};
use kymux_types::{
    av::{AVPacket, CodecPacket, CodecPacketHeader, MediaPacket, MediaPacketHeader},
    ProtocolSend,
};

use syber_common::config::{ServerConfig, VideoProtocolChoice};
use crate::capture::{ScreenCapture, rgba_to_rgb};
use crate::encode::VideoEncoder;
use crate::input;

// Endpoint IDs — server uses even IDs (parity = INITIATOR_SERVER = 0)
const VIDEO_ENDPOINT_ID: u16 = 0;
const INPUT_ENDPOINT_ID: u16 = 2;

// ── Shared state (UI ↔ session) ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub fps:          f32,
    pub bitrate_kbps: f32,
    pub client_count: usize,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self { fps: 0.0, bitrate_kbps: 0.0, client_count: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running { addr: SocketAddr },
    Error(String),
}

pub struct SharedState {
    pub status: ServerStatus,
    pub stats:  SessionStats,
    pub config: ServerConfig,
}

impl SharedState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            status: ServerStatus::Stopped,
            stats:  SessionStats::default(),
            config,
        }
    }
}

pub type SharedStateArc = Arc<Mutex<SharedState>>;

// ── Session control ───────────────────────────────────────────────────────────

pub enum SessionCmd {
    Stop,
}

/// Spawn the server session in a dedicated thread with its own tokio runtime.
pub fn start_server(state: SharedStateArc, mut cmd_rx: mpsc::Receiver<SessionCmd>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            if let Err(e) = run_server(state.clone(), &mut cmd_rx).await {
                error!("Server error: {e:#}");
                let mut s = state.lock().unwrap();
                s.status = ServerStatus::Error(e.to_string());
            }
        });
    });
}

async fn run_server(
    state:  SharedStateArc,
    cmd_rx: &mut mpsc::Receiver<SessionCmd>,
) -> Result<()> {
    let config = state.lock().unwrap().config.clone();
    state.lock().unwrap().status = ServerStatus::Starting;

    // Build cert
    let (cert_chain, key) = crate::cert::ServerCert::from_pem(&config.cert_pem, &config.key_pem)
        .context("load cert")?
        .to_kynet_types()
        .context("convert cert")?;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let server_opts = CommonServerOptions {
        max_idle_timeout:    Some(Duration::from_secs(30)),
        keep_alive_interval: Some(Duration::from_secs(5)),
    };

    let server = CommonServer::start_on_addr(addr, cert_chain, key, &server_opts)
        .context("start kyproto server")?;

    info!("Syber server listening on {addr}");
    state.lock().unwrap().status = ServerStatus::Running { addr };

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(SessionCmd::Stop) | None => {
                        info!("Server stopping");
                        server.close(0, "stopped");
                        break;
                    }
                }
            }
            conn_result = server.accept_with_auth() => {
                match conn_result {
                    Ok(unauth) => {
                        let token = unauth.get_auth().token().to_string();
                        if token != config.password {
                            warn!("Client rejected: wrong password");
                            unauth.reject_authentication();
                            continue;
                        }
                        let conn = match unauth.accept_authentication().await {
                            Ok(c)  => c,
                            Err(e) => { warn!("Auth finalize error: {e}"); continue; }
                        };
                        info!("Client authenticated");
                        state.lock().unwrap().stats.client_count += 1;

                        let config_c = config.clone();
                        let state_c  = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(conn, config_c, state_c.clone()).await {
                                warn!("Client session ended: {e:#}");
                            }
                            state_c.lock().unwrap().stats.client_count =
                                state_c.lock().unwrap().stats.client_count.saturating_sub(1);
                        });
                    }
                    Err(e) => warn!("Accept error: {e}"),
                }
            }
        }
    }

    state.lock().unwrap().status = ServerStatus::Stopped;
    Ok(())
}

async fn handle_client(
    conn:   kyproto::Connection,
    config: ServerConfig,
    state:  SharedStateArc,
) -> Result<()> {
    // Register endpoints
    let video_ep = conn
        .register_video_endpoint(Some(VIDEO_ENDPOINT_ID), map_video_proto(&config.video_protocol))
        .await.context("register video ep")?;
    let input_ep = conn
        .register_input_endpoint(Some(INPUT_ENDPOINT_ID))
        .await.context("register input ep")?;

    // Wait for both to be ready
    let (mut video_proto, input_proto) = tokio::try_join!(
        video_ep.ready(),
        input_ep.ready(),
    ).context("endpoints ready")?;

    // Destructure input protocol: we only need recv on the server
    let kymux_types::InputProtocol { send: _input_send, recv: mut input_recv } = input_proto;

    // Send codec info first
    let codec_pkt = AVPacket::Codec(CodecPacket {
        header: CodecPacketHeader {
            codec:      config.codec.codec_id(),
            rotation:   0,
            frame_size: 0,
        },
    });
    video_proto.send.send(codec_pkt).await
        .map_err(|e| anyhow::anyhow!("send codec: {e:?}"))?;
    debug!("Codec packet sent (id={})", config.codec.codec_id());

    // Spawn input receiver task
    tokio::spawn(async move {
        loop {
            match input_recv.recv().await {
                Ok(Some(wire_pkt)) => {
                    if let Err(e) = input::inject(&wire_pkt) {
                        warn!("Input inject: {e}");
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }
    });

    // Video capture + encode + send loop
    let capture = ScreenCapture::new(config.display_index);
    let scale   = config.resolution_scale;

    // First frame to establish resolution
    let first = capture.capture(scale).context("first capture")?;
    let (enc_w, enc_h) = (first.width, first.height);

    let mut encoder = VideoEncoder::new(enc_w, enc_h, config.fps, config.bitrate_kbps)
        .context("encoder init")?;

    // Stats
    let mut frame_count = 0u64;
    let mut bytes_sent  = 0u64;
    let mut last_stats  = Instant::now();
    let frame_dur       = Duration::from_secs_f32(1.0 / config.fps.max(1.0));

    send_frame(&first.rgba, &mut encoder, &mut video_proto.send, &config).await?;

    loop {
        let t0 = Instant::now();

        let frame = match capture.capture(scale) {
            Ok(f)  => f,
            Err(e) => { warn!("Capture: {e}"); break; }
        };

        let sz = send_frame(&frame.rgba, &mut encoder, &mut video_proto.send, &config).await?;
        frame_count += 1;
        bytes_sent  += sz as u64;

        // Update stats every second
        let elapsed = last_stats.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let fps  = frame_count as f32 / elapsed.as_secs_f32();
            let kbps = (bytes_sent as f32 * 8.0) / (elapsed.as_secs_f32() * 1000.0);
            let mut s = state.lock().unwrap();
            s.stats.fps          = fps;
            s.stats.bitrate_kbps = kbps;
            frame_count = 0;
            bytes_sent  = 0;
            last_stats  = Instant::now();
        }

        // Rate-limit to target fps
        let elapsed = t0.elapsed();
        if elapsed < frame_dur {
            tokio::time::sleep(frame_dur - elapsed).await;
        }
    }

    Ok(())
}

async fn send_frame(
    rgba:    &[u8],
    encoder: &mut VideoEncoder,
    send:    &mut ProtocolSend<AVPacket>,
    _cfg:    &ServerConfig,
) -> Result<usize> {
    let rgb     = rgba_to_rgb(rgba);
    let packets = encoder.encode_rgb(&rgb)?;
    let mut total = 0usize;

    for pkt in packets {
        total += pkt.data.len();
        let av_pkt = AVPacket::Media(MediaPacket {
            header: MediaPacketHeader {
                is_config: pkt.is_config,
                is_key:    pkt.is_keyframe,
                pts:       pkt.pts,
                size:      pkt.data.len() as u32,
            },
            payload: pkt.data,
        });
        send.send(av_pkt).await
            .map_err(|e| anyhow::anyhow!("send video: {e:?}"))?;
    }

    Ok(total)
}

fn map_video_proto(choice: &VideoProtocolChoice) -> VideoProtocol {
    match choice {
        VideoProtocolChoice::Reliable      => VideoProtocol::Reliable,
        VideoProtocolChoice::GopStream     => VideoProtocol::GopStream,
        VideoProtocolChoice::UnreliableFec => VideoProtocol::UnreliableFec,
    }
}
