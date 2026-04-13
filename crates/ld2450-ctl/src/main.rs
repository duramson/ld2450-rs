use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ld2450_proto::{
    command::{
        parse_ack, AckData, AckFrame, AckStatus, BaudRateIndex, Command, CommandFrame, ZoneRect,
    },
    ZoneFilterType,
};
use serialport::SerialPort;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ld2450-ctl", about = "HLK-LD2450 radar sensor configuration")]
struct Cli {
    /// Serial device path
    #[arg(short, long, default_value = "/dev/ttyAMA0")]
    device: PathBuf,

    /// Baud rate
    #[arg(short, long, default_value_t = 256000)]
    baud_rate: u32,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Read firmware version
    Firmware,
    /// Query current tracking mode
    GetMode,
    /// Set tracking mode
    SetMode {
        #[arg(value_parser = ["single", "multi"])]
        mode: String,
    },
    /// Set baud rate
    SetBaud {
        /// Baud rate (9600, 19200, 38400, 57600, 115200, 230400, 256000, 460800)
        rate: u32,
    },
    /// Restore factory settings
    FactoryReset,
    /// Restart the module
    Restart,
    /// Enable or disable Bluetooth
    Bluetooth {
        #[arg(value_parser = ["on", "off"])]
        state: String,
    },
    /// Get module MAC address
    GetMac,
    /// Query zone filter configuration
    GetZone,
    /// Set zone filter configuration
    SetZone {
        /// Filter type: disable, detect-only, exclude
        #[arg(long, value_parser = ["disable", "detect-only", "exclude"])]
        filter: String,
        /// Zone 1 as x1,y1,x2,y2 in mm (e.g. -1000,0,1000,3000)
        #[arg(long)]
        zone1: Option<String>,
        /// Zone 2 as x1,y1,x2,y2 in mm
        #[arg(long)]
        zone2: Option<String>,
        /// Zone 3 as x1,y1,x2,y2 in mm
        #[arg(long)]
        zone3: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut port = serialport::new(cli.device.to_string_lossy(), cli.baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .stop_bits(serialport::StopBits::One)
        .parity(serialport::Parity::None)
        .timeout(Duration::from_secs(2))
        .open()
        .context("failed to open serial port")?;

    // Enable configuration mode
    send_and_ack(&mut port, &Command::EnableConfig)?;
    eprintln!("configuration mode enabled");

    // Execute the command
    let result = match &cli.command {
        Cmd::Firmware => {
            let ack = send_and_ack(&mut port, &Command::ReadFirmwareVersion)?;
            match ack.data {
                AckData::FirmwareVersion {
                    fw_type,
                    major,
                    minor,
                } => {
                    println!("Firmware: V{major}.{minor:08X} (type {fw_type})");
                }
                _ => println!("Firmware version read (no detail)"),
            }
            Ok(())
        }
        Cmd::GetMode => {
            let ack = send_and_ack(&mut port, &Command::QueryTrackingMode)?;
            match ack.data {
                AckData::TrackingMode(mode) => {
                    println!("Tracking mode: {mode:?}");
                }
                _ => println!("Could not determine tracking mode"),
            }
            Ok(())
        }
        Cmd::SetMode { mode } => {
            let cmd = match mode.as_str() {
                "single" => Command::SingleTargetTracking,
                "multi" => Command::MultiTargetTracking,
                _ => unreachable!(),
            };
            send_and_ack(&mut port, &cmd)?;
            println!("Tracking mode set to: {mode}");
            Ok(())
        }
        Cmd::SetBaud { rate } => {
            let idx = BaudRateIndex::from_rate(*rate).context("unsupported baud rate")?;
            send_and_ack(&mut port, &Command::SetBaudRate(idx))?;
            println!("Baud rate set to: {rate} (effective after restart)");
            Ok(())
        }
        Cmd::FactoryReset => {
            send_and_ack(&mut port, &Command::RestoreFactory)?;
            println!("Factory settings restored (effective after restart)");
            Ok(())
        }
        Cmd::Restart => {
            send_and_ack(&mut port, &Command::Restart)?;
            println!("Module restarting...");
            Ok(())
        }
        Cmd::Bluetooth { state } => {
            let on = state == "on";
            send_and_ack(&mut port, &Command::SetBluetooth(on))?;
            println!("Bluetooth: {} (effective after restart)", state);
            Ok(())
        }
        Cmd::GetMac => {
            let ack = send_and_ack(&mut port, &Command::GetMacAddress)?;
            match ack.data {
                AckData::MacAddress(mac) => {
                    println!(
                        "MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                    );
                }
                _ => println!("MAC address not available"),
            }
            Ok(())
        }
        Cmd::GetZone => {
            let ack = send_and_ack(&mut port, &Command::QueryZoneFilter)?;
            match ack.data {
                AckData::ZoneFilter { filter_type, zones } => {
                    println!("Filter type: {filter_type:?}");
                    for (i, z) in zones.iter().enumerate() {
                        if z.x1 != 0 || z.y1 != 0 || z.x2 != 0 || z.y2 != 0 {
                            println!("Zone {}: ({},{}) → ({},{})", i + 1, z.x1, z.y1, z.x2, z.y2);
                        }
                    }
                }
                _ => println!("Zone filter config not available"),
            }
            Ok(())
        }
        Cmd::SetZone {
            filter,
            zone1,
            zone2,
            zone3,
        } => {
            let filter_type = match filter.as_str() {
                "disable" => ZoneFilterType::Disabled,
                "detect-only" => ZoneFilterType::DetectOnly,
                "exclude" => ZoneFilterType::Exclude,
                _ => unreachable!(),
            };
            let zones = [
                parse_zone(zone1.as_deref())?,
                parse_zone(zone2.as_deref())?,
                parse_zone(zone3.as_deref())?,
            ];
            send_and_ack(&mut port, &Command::SetZoneFilter { filter_type, zones })?;
            println!("Zone filter configured");
            Ok(())
        }
    };

    // End configuration mode (skip if we just restarted)
    if !matches!(cli.command, Cmd::Restart) {
        let _ = send_and_ack(&mut port, &Command::EndConfig);
        eprintln!("configuration mode ended");
    }

    result
}

fn parse_zone(s: Option<&str>) -> Result<ZoneRect> {
    let Some(s) = s else {
        return Ok(ZoneRect::default());
    };
    let parts: Vec<i16> = s
        .split(',')
        .map(|p| {
            p.trim()
                .parse::<i16>()
                .context("zone coordinate must be an integer (mm)")
        })
        .collect::<Result<_>>()?;
    anyhow::ensure!(parts.len() == 4, "zone must be x1,y1,x2,y2 (4 values)");
    Ok(ZoneRect {
        x1: parts[0],
        y1: parts[1],
        x2: parts[2],
        y2: parts[3],
    })
}

fn send_and_ack(port: &mut Box<dyn SerialPort>, cmd: &Command) -> Result<AckFrame> {
    let frame = CommandFrame::build(cmd);
    port.write_all(frame.as_bytes())
        .context("failed to write command")?;
    port.flush().context("failed to flush")?;

    // Read ACK response with timeout
    let mut buf = [0u8; 128];
    let mut total = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);

    loop {
        if std::time::Instant::now() > deadline {
            bail!("timeout waiting for ACK");
        }

        match port.read(&mut buf[total..]) {
            Ok(n) if n > 0 => {
                total += n;
                if let Some((ack, _)) = parse_ack(&buf[..total]) {
                    if ack.status == AckStatus::Failure {
                        bail!("command failed (ACK status: failure)");
                    }
                    return Ok(ack);
                }
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
