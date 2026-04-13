use ld2450_proto::{FrameParser, ParseEvent, RadarFrame};
use tokio::sync::broadcast;
use tokio_serial::SerialPortBuilderExt;
use tracing::{debug, info, warn};

pub async fn run(
    device: std::path::PathBuf,
    baud_rate: u32,
    tx: broadcast::Sender<RadarFrame>,
) -> anyhow::Result<()> {
    info!(?device, baud_rate, "opening serial port");

    let mut port = tokio_serial::new(device.to_string_lossy(), baud_rate)
        .data_bits(tokio_serial::DataBits::Eight)
        .stop_bits(tokio_serial::StopBits::One)
        .parity(tokio_serial::Parity::None)
        .open_native_async()?;

    #[cfg(target_os = "linux")]
    {
        use tokio_serial::SerialPort;
        port.set_exclusive(false)?;
    }

    info!("serial port opened, starting frame parser");

    let mut parser = FrameParser::new();
    let mut buf = [0u8; 256];

    use tokio::io::AsyncReadExt;
    loop {
        let n = port.read(&mut buf).await?;
        if n == 0 {
            warn!("serial port returned 0 bytes, EOF?");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        debug!(bytes = n, "read from serial");

        for &byte in &buf[..n] {
            if let Some(ParseEvent::Frame(frame)) = parser.feed(byte) {
                debug!(active = frame.active_count(), "parsed radar frame");
                // Ignore send errors — means no receivers connected
                let _ = tx.send(frame);
            }
        }
    }
}
