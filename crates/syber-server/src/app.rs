//! Server egui application — minimalist, everything in UI.

use std::sync::{Arc, Mutex};
use egui::{Color32, RichText, Ui, Vec2};
use tokio::sync::mpsc;

use syber_common::config::{
    ServerConfig, QualityPreset, VideoCodec, VideoEncoder, VideoProtocolChoice,
};
use crate::cert::ServerCert;
use crate::session::{SharedState, SharedStateArc, ServerStatus, SessionCmd, start_server};
use crate::capture::ScreenCapture;

// ── Tabs ──────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum SettingsTab { Simple, Advanced }

// ── App ───────────────────────────────────────────────────────────────────────

pub struct ServerApp {
    state:   SharedStateArc,
    cmd_tx:  Option<mpsc::Sender<SessionCmd>>,

    // UI state (not persisted)
    tab:          SettingsTab,
    show_password: bool,
    status_msg:    String,

    // Temporary edit buffers (strings for text inputs)
    port_str:     String,
    bitrate_str:  String,
    fps_str:      String,

    // Available monitors (cached)
    monitors:     Vec<(usize, String, u32, u32)>,
}

impl ServerApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut config = ServerConfig::load();

        // Generate cert if not present
        if config.cert_pem.is_empty() {
            if let Ok(cert) = ServerCert::generate() {
                config.cert_pem  = cert.cert_pem;
                config.key_pem   = cert.key_pem;
                config.cert_hash = cert.cert_hash;
                let _ = config.save();
            }
        }

        let port_str    = config.port.to_string();
        let bitrate_str = config.bitrate_kbps.to_string();
        let fps_str     = format!("{:.0}", config.fps);
        let monitors    = ScreenCapture::list_monitors();

        let state = Arc::new(Mutex::new(SharedState::new(config)));

        Self {
            state,
            cmd_tx:        None,
            tab:           SettingsTab::Simple,
            show_password: false,
            status_msg:    String::new(),
            port_str,
            bitrate_str,
            fps_str,
            monitors,
        }
    }

    // ── Server control ─────────────────────────────────────────────────────

    fn start(&mut self) {
        // Parse and commit text fields
        if let Ok(port) = self.port_str.parse::<u16>() {
            self.state.lock().unwrap().config.port = port;
        }
        if let Ok(kbps) = self.bitrate_str.parse::<u32>() {
            self.state.lock().unwrap().config.bitrate_kbps = kbps;
        }
        if let Ok(fps) = self.fps_str.parse::<f32>() {
            let fps = fps.clamp(1.0, 240.0);
            self.state.lock().unwrap().config.fps = fps;
        }

        // Save config
        let _ = self.state.lock().unwrap().config.save();

        let (cmd_tx, cmd_rx) = mpsc::channel::<SessionCmd>(4);
        self.cmd_tx = Some(cmd_tx);
        start_server(self.state.clone(), cmd_rx);
    }

    fn stop(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.try_send(SessionCmd::Stop);
        }
    }

    fn is_running(&self) -> bool {
        matches!(self.state.lock().unwrap().status,
            ServerStatus::Running { .. } | ServerStatus::Starting)
    }
}

// ── egui eframe App ───────────────────────────────────────────────────────────

impl eframe::App for ServerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        let (status, stats, config) = {
            let s = self.state.lock().unwrap();
            (s.status.clone(), s.stats.clone(), s.config.clone())
        };

        // Top bar
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("⬡ SYBER SERVER").strong());
                ui.separator();
                match &status {
                    ServerStatus::Stopped => {
                        ui.label(RichText::new("⬤ Arrêté").color(Color32::GRAY));
                    }
                    ServerStatus::Starting => {
                        ui.label(RichText::new("⬤ Démarrage…").color(Color32::YELLOW));
                    }
                    ServerStatus::Running { addr } => {
                        ui.label(RichText::new("⬤ En cours").color(Color32::GREEN));
                        ui.label(format!("  {}  |  {} client(s)", addr, stats.client_count));
                        if stats.client_count > 0 {
                            ui.separator();
                            ui.label(format!("{:.0} fps  {:.0} kbps",
                                stats.fps, stats.bitrate_kbps));
                        }
                    }
                    ServerStatus::Error(e) => {
                        ui.label(RichText::new(format!("⬤ Erreur: {e}"))
                            .color(Color32::RED));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_running() {
                        if ui.button(RichText::new("⏹ Stop").color(Color32::RED)).clicked() {
                            self.stop();
                        }
                    } else {
                        let btn = egui::Button::new(
                            RichText::new("▶ Démarrer").color(Color32::WHITE)
                        ).fill(Color32::from_rgb(0, 140, 60));
                        if ui.add(btn).clicked() {
                            self.start();
                        }
                    }
                });
            });
        });

        // Main panels
        egui::SidePanel::left("info_panel").min_width(210.0).max_width(240.0)
            .show(ctx, |ui| {
                self.draw_info_panel(ui, &status, &config);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Tabs
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, SettingsTab::Simple, "⚙ Simple");
                ui.selectable_value(&mut self.tab, SettingsTab::Advanced, "🔧 Avancé");
            });
            ui.separator();

            let disabled = self.is_running();
            ui.set_enabled(!disabled);

            match self.tab {
                SettingsTab::Simple   => self.draw_simple_tab(ui),
                SettingsTab::Advanced => self.draw_advanced_tab(ui),
            }
        });
    }
}

impl ServerApp {
    fn draw_info_panel(&self, ui: &mut Ui, status: &ServerStatus, config: &ServerConfig) {
        ui.add_space(8.0);
        ui.label(RichText::new("Empreinte TLS").weak());
        ui.add_space(2.0);

        let fp = ServerCert::format_fingerprint(&config.cert_hash);
        let fp_short = if fp.len() > 29 {
            format!("{}…", &fp[..29])
        } else {
            fp.clone()
        };

        egui::Frame::none()
            .fill(Color32::from_gray(30))
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.label(RichText::new(&fp_short).monospace().size(10.5));
            });

        if ui.small_button("📋 Copier").clicked() {
            ui.output_mut(|o| o.copied_text = fp);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        ui.label(RichText::new("Adresses réseau").weak());
        for ip in get_local_ips() {
            ui.label(RichText::new(format!("{ip}:{}", config.port)).monospace().size(11.0));
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if let ServerStatus::Running { .. } = status {
            let stats = &self.state.lock().unwrap().stats;
            ui.label(RichText::new("Session").weak());
            ui.label(format!("Clients : {}", stats.client_count));
            if stats.client_count > 0 {
                ui.label(format!("FPS : {:.0}", stats.fps));
                ui.label(format!("Débit : {:.0} kbps", stats.bitrate_kbps));
            }
        }
    }

    fn draw_simple_tab(&mut self, ui: &mut Ui) {
        let config = &mut self.state.lock().unwrap().config;

        egui::Grid::new("simple_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label("Port :");
                ui.add(egui::TextEdit::singleline(&mut self.port_str).desired_width(80.0));
                ui.end_row();

                ui.label("Mot de passe :");
                ui.horizontal(|ui| {
                    let pw_edit = egui::TextEdit::singleline(&mut config.password)
                        .desired_width(160.0)
                        .password(!self.show_password);
                    ui.add(pw_edit);
                    if ui.small_button(if self.show_password { "🙈" } else { "👁" }).clicked() {
                        self.show_password = !self.show_password;
                    }
                    if ui.small_button("🔄").on_hover_text("Générer un nouveau mot de passe").clicked() {
                        config.password = syber_common::config::ServerConfig::generate_password();
                    }
                });
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.label(RichText::new("Qualité vidéo").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for preset in [QualityPreset::Fast, QualityPreset::Good, QualityPreset::Best] {
                let selected = config.preset == preset;
                let btn = egui::Button::new(preset.label())
                    .fill(if selected { Color32::from_rgb(0, 100, 180) } else { Color32::from_gray(50) });
                if ui.add(btn).clicked() {
                    config.preset = preset.clone();
                    config.apply_preset();
                    self.bitrate_str = config.bitrate_kbps.to_string();
                    self.fps_str     = format!("{:.0}", config.fps);
                }
            }
        });

        ui.add_space(8.0);

        // Monitor selection
        if !self.monitors.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Écran :");
                let current = config.display_index;
                egui::ComboBox::from_id_source("monitor_select")
                    .selected_text(
                        self.monitors.get(current)
                            .map(|(_, name, w, h)| format!("{name} ({w}×{h})"))
                            .unwrap_or_else(|| "Écran 0".to_string())
                    )
                    .show_ui(ui, |ui| {
                        for (i, name, w, h) in &self.monitors {
                            let label = format!("{name} ({w}×{h})");
                            ui.selectable_value(&mut config.display_index, *i, label);
                        }
                    });
            });
        }
    }

    fn draw_advanced_tab(&mut self, ui: &mut Ui) {
        let config = &mut self.state.lock().unwrap().config;

        egui::Grid::new("adv_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                // Codec
                ui.label("Codec :");
                egui::ComboBox::from_id_source("codec")
                    .selected_text(config.codec.label())
                    .show_ui(ui, |ui| {
                        for c in [VideoCodec::H264, VideoCodec::H265] {
                            ui.selectable_value(&mut config.codec, c.clone(), c.label());
                        }
                    });
                ui.end_row();

                // Encoder
                ui.label("Encodeur :");
                egui::ComboBox::from_id_source("encoder")
                    .selected_text(config.encoder.label())
                    .show_ui(ui, |ui| {
                        for e in [
                            VideoEncoder::Auto, VideoEncoder::Software,
                            VideoEncoder::Nvenc, VideoEncoder::Qsv,
                            VideoEncoder::Amf,  VideoEncoder::Vaapi,
                        ] {
                            ui.selectable_value(&mut config.encoder, e.clone(), e.label());
                        }
                    });
                ui.end_row();

                // Bitrate
                ui.label("Débit (kbps) :");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.bitrate_str).desired_width(70.0));
                    ui.add(egui::Slider::new(&mut config.bitrate_kbps, 1_000..=50_000)
                        .suffix(" kbps").clamp_to_range(true));
                });
                self.bitrate_str = config.bitrate_kbps.to_string();
                ui.end_row();

                // FPS
                ui.label("FPS :");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.fps_str).desired_width(50.0));
                    ui.add(egui::Slider::new(&mut config.fps, 1.0..=240.0)
                        .suffix(" fps").clamp_to_range(true));
                });
                self.fps_str = format!("{:.0}", config.fps);
                ui.end_row();

                // Resolution scale
                ui.label("Résolution :");
                ui.add(egui::Slider::new(&mut config.resolution_scale, 0.25..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                ui.end_row();

                // Protocol
                ui.label("Protocole vidéo :");
                egui::ComboBox::from_id_source("vproto")
                    .selected_text(config.video_protocol.label())
                    .show_ui(ui, |ui| {
                        for p in [
                            VideoProtocolChoice::Reliable,
                            VideoProtocolChoice::GopStream,
                            VideoProtocolChoice::UnreliableFec,
                        ] {
                            ui.selectable_value(&mut config.video_protocol, p.clone(), p.label());
                        }
                    });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Cert regeneration
        ui.horizontal(|ui| {
            ui.label(RichText::new("Certificat TLS").weak());
            if ui.small_button("🔄 Régénérer").on_hover_text(
                "Génère un nouveau certificat. Les clients devront reconnecter avec la nouvelle empreinte."
            ).clicked() {
                if let Ok(cert) = ServerCert::generate() {
                    config.cert_pem  = cert.cert_pem;
                    config.key_pem   = cert.key_pem;
                    config.cert_hash = cert.cert_hash;
                }
            }
        });
        ui.label(RichText::new(
            "Le certificat auto-signé est validé par empreinte SHA-256.\nCopiez l'empreinte sur le client."
        ).weak().size(11.0));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Windows: use std::net::UdpSocket trick
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    ips.push(addr.ip().to_string());
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    ips.push(addr.ip().to_string());
                }
            }
        }
    }

    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }
    ips
}


