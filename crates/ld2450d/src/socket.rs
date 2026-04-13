use ld2450_proto::RadarFrame;
use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

#[derive(Serialize)]
struct FrameJson<'a> {
    ts: f64,
    targets: &'a [TargetJson],
}

#[derive(Serialize)]
struct TargetJson {
    x: i16,
    y: i16,
    speed: i16,
    dist_res: u16,
    dist_mm: f32,
    angle_deg: f32,
}

fn timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn frame_to_json(frame: &RadarFrame) -> Vec<u8> {
    let targets: Vec<TargetJson> = frame
        .targets
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| TargetJson {
            x: t.x,
            y: t.y,
            speed: t.speed,
            dist_res: t.distance_resolution,
            dist_mm: t.distance_mm(),
            angle_deg: t.angle_deg(),
        })
        .collect();

    let json = FrameJson {
        ts: timestamp(),
        targets: &targets,
    };

    let mut buf = serde_json::to_vec(&json).expect("serialization cannot fail");
    buf.push(b'\n');
    buf
}

pub async fn run(socket_path: &Path, tx: broadcast::Sender<RadarFrame>) -> anyhow::Result<()> {
    // Remove stale socket file
    let _ = std::fs::remove_file(socket_path);

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(socket_path)?;

    // Make socket world-readable so other services can connect
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o666))?;
    }

    info!(?socket_path, "listening for connections");

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let rx = tx.subscribe();
                tokio::spawn(handle_client(stream, rx));
                debug!("new client connected");
            }
            Err(e) => {
                error!(%e, "accept error");
            }
        }
    }
}

async fn handle_client(
    mut stream: tokio::net::UnixStream,
    mut rx: broadcast::Receiver<RadarFrame>,
) {
    loop {
        match rx.recv().await {
            Ok(frame) => {
                let json = frame_to_json(&frame);
                if let Err(e) = stream.write_all(&json).await {
                    debug!(%e, "client disconnected");
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(skipped = n, "client lagging, dropped frames");
            }
            Err(broadcast::error::RecvError::Closed) => {
                debug!("broadcast channel closed");
                return;
            }
        }
    }
}
