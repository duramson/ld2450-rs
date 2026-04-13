mod config;
mod socket;
mod uart;

use config::Config;
use ld2450_proto::RadarFrame;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tracing::info;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Load config
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/ld2450d.toml"));

    let config = if config_path.exists() {
        Config::load(&config_path)?
    } else {
        info!("no config file found, using defaults");
        Config::default()
    };

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .with_target(false)
        .init();

    info!(
        device = %config.device.display(),
        baud_rate = config.baud_rate,
        socket = %config.socket_path.display(),
        "ld2450d starting"
    );

    // Broadcast channel — 16 frames buffer (10Hz → ~1.6s)
    let (tx, _rx) = broadcast::channel::<RadarFrame>(16);

    // Spawn UART reader
    let uart_tx = tx.clone();
    let uart_device = config.device.clone();
    let uart_baud = config.baud_rate;
    let uart_handle = tokio::spawn(async move {
        if let Err(e) = uart::run(uart_device, uart_baud, uart_tx).await {
            tracing::error!(%e, "UART reader failed");
        }
    });

    // Spawn socket server
    let socket_path = config.socket_path.clone();
    let socket_tx = tx.clone();
    let socket_handle = tokio::spawn(async move {
        if let Err(e) = socket::run(&socket_path, socket_tx).await {
            tracing::error!(%e, "socket server failed");
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("shutdown signal received");

    // Cleanup socket file
    let _ = std::fs::remove_file(&config.socket_path);

    // Abort tasks
    uart_handle.abort();
    socket_handle.abort();

    info!("ld2450d stopped");
    Ok(())
}
