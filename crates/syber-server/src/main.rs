mod app;
mod cert;
mod session;
mod capture;
mod encode;
mod input;

use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("syber_server=debug".parse().unwrap())
            .add_directive("warn".parse().unwrap()))
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Syber Server")
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(eframe::icon_data::from_png_bytes(
                include_bytes!("../assets/icon.png")
            ).unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "Syber Server",
        options,
        Box::new(|cc| Ok(Box::new(app::ServerApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("UI error: {e}"))?;

    Ok(())
}
