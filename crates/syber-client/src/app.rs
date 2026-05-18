//! Client egui application.
//! Two views: Connect page and Stream page (with video + stats overlay).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use egui::{Color32, Key, RichText, TextureHandle, TextureOptions, Ui, Vec2};
use tokio::sync::mpsc;

use syber_common::config::ClientConfig;
use crate::session::{
    ClientCmd, ClientStatus, ConnectParams, SharedClientState, SharedClientStateArc,
    start_client,
};
use crate::input;
use kyproto::VideoProtocol;

// ── App ───────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Page { Connect, Settings, Stream }

pub struct ClientApp {
    config:  ClientConfig,
    state:   SharedClientStateArc,
    cmd_tx:  Option<mpsc::Sender<ClientCmd>>,

    page:  Page,

    // Connect form buffers
    host_buf:  String,
    port_buf:  String,
    pass_buf:  String,
    hash_buf:  String,
    show_pass: bool,

    // Video texture
    texture:          Option<TextureHandle>,
    last_texture_size: (u32, u32),

    // Hotkey state
    ctrl_pressed:   bool,
    shift_pressed:  bool,
    last_frame_time: Instant,
}

impl ClientApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let config = ClientConfig::load();

        let host_buf = config.last_host.clone();
        let port_buf = config.last_port.to_string();
        let hash_buf = config.last_cert_hash.clone();
        let state    = Arc::new(Mutex::new(SharedClientState::new()));

        Self {
            config,
            state,
            cmd_tx:           None,
            page:             Page::Connect,
            host_buf,
            port_buf,
            pass_buf:         String::new(),
            hash_buf,
            show_pass:        false,
            texture:          None,
            last_texture_size: (0, 0),
            ctrl_pressed:     false,
            shift_pressed:    false,
            last_frame_time:  Instant::now(),
        }
    }

    // ── Connection control ─────────────────────────────────────────────────

    fn connect(&mut self) {
        let host = self.host_buf.trim().to_string();
        let port = self.port_buf.trim().parse::<u16>().unwrap_or(8080);
        let pass = self.pass_buf.clone();
        let hash = self.hash_buf.trim().to_string();

        if host.is_empty() { return; }

        // Save connection info
        self.config.last_host      = host.clone();
        self.config.last_port      = port;
        self.config.last_cert_hash = hash.clone();
        self.config.add_recent(&host, port, &hash);
        let _ = self.config.save();

        let (cmd_tx, cmd_rx) = mpsc::channel::<ClientCmd>(256);
        self.cmd_tx = Some(cmd_tx);

        let params = ConnectParams {
            host,
            port,
            password:  pass,
            cert_hash: hash,
            video_protocol: VideoProtocol::GopStream,
        };

        start_client(self.state.clone(), params, cmd_rx);
        self.page = Page::Stream;
    }

    fn disconnect(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.try_send(ClientCmd::Disconnect);
        }
        self.page = Page::Connect;
        self.texture = None;
    }

    fn send_input(&mut self, pkt: kymux_types::input::InputPacket) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.try_send(ClientCmd::SendInput(pkt));
        }
    }

    fn is_connected(&self) -> bool {
        matches!(self.state.lock().unwrap().status, ClientStatus::Connected)
    }
}

// ── eframe App ────────────────────────────────────────────────────────────────

impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Request repaint at ~60fps when streaming
        let repaint_delay = if self.page == Page::Stream {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        ctx.request_repaint_after(repaint_delay);

        let status = self.state.lock().unwrap().status.clone();

        // Auto-navigate on disconnect
        if self.page == Page::Stream {
            match &status {
                ClientStatus::Disconnected if self.cmd_tx.is_none() => {}
                ClientStatus::Error(_) => {
                    self.page = Page::Connect;
                    self.cmd_tx = None;
                }
                _ => {}
            }
        }

        // Hotkey: Ctrl+Shift+Q → disconnect
        ctx.input(|input| {
            self.ctrl_pressed  = input.modifiers.ctrl;
            self.shift_pressed = input.modifiers.shift;

            if input.modifiers.ctrl && input.modifiers.shift
                && input.key_pressed(Key::Q)
                && self.page == Page::Stream
            {
                self.disconnect();
            }
        });

        match self.page {
            Page::Connect  => self.draw_connect(ctx),
            Page::Settings => self.draw_settings(ctx),
            Page::Stream   => self.draw_stream(ctx, frame),
        }
    }
}

impl ClientApp {
    // ── Connect page ───────────────────────────────────────────────────────

    fn draw_connect(&mut self, ctx: &egui::Context) {
        let status = self.state.lock().unwrap().status.clone();

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("⬡ SYBER").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Paramètres").clicked() {
                        self.page = Page::Settings;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(30.0);

            ui.vertical_centered(|ui| {
                ui.set_max_width(380.0);

                // Status message
                match &status {
                    ClientStatus::Connecting => {
                        ui.label(RichText::new("⏳ Connexion en cours…").color(Color32::YELLOW));
                        ui.add_space(8.0);
                    }
                    ClientStatus::Error(e) => {
                        ui.label(RichText::new(format!("✗ Erreur : {e}"))
                            .color(Color32::RED));
                        ui.add_space(8.0);
                    }
                    _ => {}
                }

                egui::Frame::none()
                    .fill(Color32::from_gray(25))
                    .inner_margin(20.0)
                    .rounding(8.0)
                    .show(ui, |ui| {
                        ui.set_width(360.0);

                        egui::Grid::new("connect_grid")
                            .num_columns(2)
                            .spacing([12.0, 10.0])
                            .show(ui, |ui| {
                                ui.label("Hôte :");
                                ui.add(egui::TextEdit::singleline(&mut self.host_buf)
                                    .hint_text("192.168.1.100")
                                    .desired_width(220.0));
                                ui.end_row();

                                ui.label("Port :");
                                ui.add(egui::TextEdit::singleline(&mut self.port_buf)
                                    .desired_width(70.0));
                                ui.end_row();

                                ui.label("Mot de passe :");
                                ui.horizontal(|ui| {
                                    ui.add(egui::TextEdit::singleline(&mut self.pass_buf)
                                        .password(!self.show_pass)
                                        .desired_width(180.0));
                                    if ui.small_button(if self.show_pass { "🙈" } else { "👁" })
                                        .clicked() { self.show_pass = !self.show_pass; }
                                });
                                ui.end_row();

                                ui.label("Empreinte TLS :")
                                    .on_hover_text("SHA-256 affiché dans le serveur. Laissez vide pour désactiver la vérification (LAN de confiance uniquement).");
                                ui.add(egui::TextEdit::singleline(&mut self.hash_buf)
                                    .hint_text("a3b2… (optionnel)")
                                    .desired_width(220.0));
                                ui.end_row();
                            });

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.config.fullscreen_on_connect, "Plein écran");
                        });

                        ui.add_space(12.0);

                        let connecting = matches!(status, ClientStatus::Connecting);
                        if connecting { ui.disable(); }

                        let btn = egui::Button::new(
                            RichText::new(if connecting { "  Connexion…  " } else { "  Connecter  " })
                                .size(16.0)
                        ).min_size(Vec2::new(200.0, 36.0))
                         .fill(Color32::from_rgb(0, 120, 200));

                        if ui.add(btn).clicked() && !connecting {
                            self.connect();
                        }
                    });

                // Recent connections
                if !self.config.recents.is_empty() {
                    ui.add_space(16.0);
                    ui.label(RichText::new("Récents").weak());
                    for recent in self.config.recents.clone() {
                        let label = format!("{}:{}", recent.host, recent.port);
                        if ui.link(&label).clicked() {
                            self.host_buf = recent.host.clone();
                            self.port_buf = recent.port.to_string();
                            self.hash_buf = recent.cert_hash.clone();
                        }
                    }
                }
            });
        });
    }

    // ── Settings page ──────────────────────────────────────────────────────

    fn draw_settings(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("← Retour").clicked() {
                    self.page = Page::Connect;
                }
                ui.heading("Paramètres");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Capture clavier :");
                    ui.checkbox(&mut self.config.capture_keyboard, "");
                    ui.end_row();

                    ui.label("Capture souris :");
                    ui.checkbox(&mut self.config.capture_mouse, "");
                    ui.end_row();

                    ui.label("Capturer le curseur :");
                    ui.checkbox(&mut self.config.grab_cursor, "");
                    ui.end_row();

                    ui.label("Overlay stats :");
                    ui.checkbox(&mut self.config.show_stats_overlay, "");
                    ui.end_row();

                    ui.label("Plein écran auto :");
                    ui.checkbox(&mut self.config.fullscreen_on_connect, "");
                    ui.end_row();

                    ui.label("Raccourci déconnexion :");
                    ui.label(RichText::new("Ctrl+Shift+Q").monospace());
                    ui.end_row();
                });

            ui.add_space(16.0);
            if ui.button("💾 Sauvegarder").clicked() {
                let _ = self.config.save();
                self.page = Page::Connect;
            }
        });
    }

    // ── Stream page ────────────────────────────────────────────────────────

    fn draw_stream(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let status = self.state.lock().unwrap().status.clone();
        let stats  = self.state.lock().unwrap().stats.clone();

        // ── Handle input events and forward to server ─────────────────────
        if self.is_connected() && self.config.capture_keyboard {
            ctx.input(|input| {
                for event in &input.events {
                    match event {
                        egui::Event::Key { key, pressed, modifiers, .. } => {
                            // Skip Ctrl+Shift+Q (disconnect hotkey)
                            if modifiers.ctrl && modifiers.shift && *key == Key::Q {
                                return;
                            }
                            if let Some(pkt) = input::egui_key_to_wire(*key, *pressed, modifiers) {
                                self.send_input(pkt);
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        if self.is_connected() && self.config.capture_mouse {
            ctx.input(|input| {
                // Mouse move
                let delta = input.pointer.delta();
                if delta.length() > 0.1 {
                    let pkt = input::egui_mouse_move_to_wire(delta);
                    self.send_input(pkt);
                }
                // Mouse buttons
                for btn in [
                    egui::PointerButton::Primary,
                    egui::PointerButton::Secondary,
                    egui::PointerButton::Middle,
                ] {
                    if input.pointer.button_pressed(btn) {
                        if let Some(pkt) = input::egui_mouse_button_to_wire(btn, true) {
                            self.send_input(pkt);
                        }
                    }
                    if input.pointer.button_released(btn) {
                        if let Some(pkt) = input::egui_mouse_button_to_wire(btn, false) {
                            self.send_input(pkt);
                        }
                    }
                }
                // Scroll
                let scroll = input.smooth_scroll_delta;
                for pkt in input::egui_scroll_to_wire(scroll) {
                    self.send_input(pkt);
                }
            });
        }

        // ── Update texture from latest frame ──────────────────────────────
        let (new_frame, new_size) = {
            let mut s = self.state.lock().unwrap();
            if s.frame_dirty {
                s.frame_dirty = false;
                let frame_ref = s.latest_frame.as_ref();
                match frame_ref {
                    Some(f) => (
                        Some((f.rgba.clone(), f.width, f.height)),
                        (f.width, f.height),
                    ),
                    None => (None, self.last_texture_size),
                }
            } else {
                (None, self.last_texture_size)
            }
        };

        if let Some((rgba, w, h)) = new_frame {
            self.last_texture_size = (w, h);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize], &rgba,
            );
            match self.texture.as_mut() {
                Some(tex) if self.last_texture_size == (w, h) => {
                    tex.set(image, TextureOptions::LINEAR);
                }
                _ => {
                    self.texture = Some(ctx.load_texture(
                        "video_frame", image, TextureOptions::LINEAR,
                    ));
                }
            }
        }

        // ── Draw video ────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let avail = ui.available_size();

                if let Some(texture) = &self.texture {
                    let tex_size = texture.size_vec2();
                    // Letterbox: fit within available area
                    let scale = (avail.x / tex_size.x).min(avail.y / tex_size.y);
                    let display_size = tex_size * scale;
                    let offset = (avail - display_size) * 0.5;

                    let rect = egui::Rect::from_min_size(
                        ui.cursor().min + offset,
                        display_size,
                    );
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.centered_and_justified(|ui| {
                        match &status {
                            ClientStatus::Connecting => {
                                ui.label(RichText::new("⏳ Connexion…").color(Color32::YELLOW).size(22.0));
                            }
                            ClientStatus::Error(e) => {
                                ui.label(RichText::new(format!("✗ {e}")).color(Color32::RED).size(18.0));
                            }
                            _ => {
                                ui.label(RichText::new("En attente du flux vidéo…")
                                    .color(Color32::GRAY).size(16.0));
                            }
                        }
                    });
                }

                // ── Stats overlay ─────────────────────────────────────────
                if self.config.show_stats_overlay {
                    let stats_text = format!(
                        "{:.0} fps  |  {:.0} kbps  |  {:.0} ms",
                        stats.fps, stats.bitrate_kbps, stats.rtt_ms
                    );
                    let painter = ui.painter();
                    let pos     = ui.min_rect().min + egui::vec2(10.0, avail.y - 28.0);
                    painter.text(
                        pos,
                        egui::Align2::LEFT_CENTER,
                        &stats_text,
                        egui::FontId::monospace(12.0),
                        Color32::from_rgba_premultiplied(255, 255, 255, 180),
                    );
                }

                // ── Disconnect button ─────────────────────────────────────
                let btn_pos = ui.min_rect().max - egui::vec2(110.0, 34.0);
                let btn_rect = egui::Rect::from_min_size(btn_pos, egui::vec2(100.0, 24.0));
                let response = ui.put(btn_rect, egui::Button::new("✕ Déconnecter")
                    .fill(Color32::from_rgba_premultiplied(180, 0, 0, 200)));
                if response.clicked() {
                    self.disconnect();
                }

                // Ctrl+Shift+Q hint
                let hint = "Ctrl+Shift+Q pour déconnecter";
                ui.painter().text(
                    ui.min_rect().min + egui::vec2(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    hint,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgba_premultiplied(200, 200, 200, 80),
                );
            });
    }
}
