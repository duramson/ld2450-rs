use crate::types::{TrackingMode, ZoneFilterType};

/// Command frame header
const CMD_HEADER: [u8; 4] = [0xFD, 0xFC, 0xFB, 0xFA];
/// Command frame footer
const CMD_FOOTER: [u8; 4] = [0x04, 0x03, 0x02, 0x01];

/// Commands that can be sent to the LD2450.
#[derive(Debug, Clone)]
pub enum Command {
    EnableConfig,
    EndConfig,
    SingleTargetTracking,
    MultiTargetTracking,
    QueryTrackingMode,
    ReadFirmwareVersion,
    SetBaudRate(BaudRateIndex),
    RestoreFactory,
    Restart,
    SetBluetooth(bool),
    GetMacAddress,
    QueryZoneFilter,
    SetZoneFilter {
        filter_type: ZoneFilterType,
        zones: [ZoneRect; 3],
    },
}

/// A rectangular zone defined by two diagonal vertices (mm).
#[derive(Debug, Clone, Copy, Default)]
pub struct ZoneRect {
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
}

/// Baud rate selection index per protocol spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BaudRateIndex {
    B9600 = 0x0001,
    B19200 = 0x0002,
    B38400 = 0x0003,
    B57600 = 0x0004,
    B115200 = 0x0005,
    B230400 = 0x0006,
    B256000 = 0x0007,
    B460800 = 0x0008,
}

impl BaudRateIndex {
    pub fn from_rate(rate: u32) -> Option<Self> {
        match rate {
            9600 => Some(Self::B9600),
            19200 => Some(Self::B19200),
            38400 => Some(Self::B38400),
            57600 => Some(Self::B57600),
            115200 => Some(Self::B115200),
            230400 => Some(Self::B230400),
            256000 => Some(Self::B256000),
            460800 => Some(Self::B460800),
            _ => None,
        }
    }

    pub fn to_rate(self) -> u32 {
        match self {
            Self::B9600 => 9600,
            Self::B19200 => 19200,
            Self::B38400 => 38400,
            Self::B57600 => 57600,
            Self::B115200 => 115200,
            Self::B230400 => 230400,
            Self::B256000 => 256000,
            Self::B460800 => 460800,
        }
    }
}

/// A serialized command frame ready to send over UART.
#[derive(Debug)]
pub struct CommandFrame {
    buf: [u8; 64],
    len: usize,
}

impl CommandFrame {
    /// Build the wire-format frame for a command.
    pub fn build(cmd: &Command) -> Self {
        let mut frame = Self {
            buf: [0u8; 64],
            len: 0,
        };

        // Header
        frame.push_slice(&CMD_HEADER);

        // Reserve 2 bytes for length, fill in after
        let len_pos = frame.len;
        frame.push_slice(&[0x00, 0x00]);

        let data_start = frame.len;

        match cmd {
            Command::EnableConfig => {
                frame.push_le_u16(0x00FF); // command word
                frame.push_le_u16(0x0001); // command value
            }
            Command::EndConfig => {
                frame.push_le_u16(0x00FE);
            }
            Command::SingleTargetTracking => {
                frame.push_le_u16(0x0080);
            }
            Command::MultiTargetTracking => {
                frame.push_le_u16(0x0090);
            }
            Command::QueryTrackingMode => {
                frame.push_le_u16(0x0091);
            }
            Command::ReadFirmwareVersion => {
                frame.push_le_u16(0x00A0);
            }
            Command::SetBaudRate(idx) => {
                frame.push_le_u16(0x00A1);
                frame.push_le_u16(*idx as u16);
            }
            Command::RestoreFactory => {
                frame.push_le_u16(0x00A2);
            }
            Command::Restart => {
                frame.push_le_u16(0x00A3);
            }
            Command::SetBluetooth(on) => {
                frame.push_le_u16(0x00A4);
                frame.push_le_u16(if *on { 0x0100 } else { 0x0000 });
            }
            Command::GetMacAddress => {
                frame.push_le_u16(0x00A5);
                frame.push_le_u16(0x0001);
            }
            Command::QueryZoneFilter => {
                frame.push_le_u16(0x00C1);
            }
            Command::SetZoneFilter { filter_type, zones } => {
                frame.push_le_u16(0x00C2);
                frame.push_le_u16(*filter_type as u16);
                for zone in zones {
                    frame.push_le_i16(zone.x1);
                    frame.push_le_i16(zone.y1);
                    frame.push_le_i16(zone.x2);
                    frame.push_le_i16(zone.y2);
                }
            }
        }

        // Fill in data length
        let data_len = (frame.len - data_start) as u16;
        frame.buf[len_pos] = data_len as u8;
        frame.buf[len_pos + 1] = (data_len >> 8) as u8;

        // Footer
        frame.push_slice(&CMD_FOOTER);

        frame
    }

    /// The serialized bytes to send over UART.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    fn push_slice(&mut self, data: &[u8]) {
        self.buf[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
    }

    fn push_le_u16(&mut self, val: u16) {
        let bytes = val.to_le_bytes();
        self.push_slice(&bytes);
    }

    fn push_le_i16(&mut self, val: i16) {
        let bytes = val.to_le_bytes();
        self.push_slice(&bytes);
    }
}

/// Status code in ACK responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    Success,
    Failure,
}

/// Parsed ACK data for different commands.
#[derive(Debug, Clone)]
pub enum AckData {
    /// Generic ACK with just a status (EndConfig, SetMode, SetBaud, etc.)
    Simple,
    /// EnableConfig response: protocol version + buffer size
    EnableConfig {
        protocol_version: u16,
        buffer_size: u16,
    },
    /// Query tracking mode response
    TrackingMode(TrackingMode),
    /// Firmware version
    FirmwareVersion {
        fw_type: u16,
        major: u16,
        minor: u32,
    },
    /// MAC address (6 bytes)
    MacAddress([u8; 6]),
    /// Zone filter configuration
    ZoneFilter {
        filter_type: ZoneFilterType,
        zones: [ZoneRect; 3],
    },
}

/// A parsed ACK frame from the radar.
#[derive(Debug, Clone)]
pub struct AckFrame {
    pub command_word: u16,
    pub status: AckStatus,
    pub data: AckData,
}

/// ACK frame parser — scans a byte buffer for a complete ACK frame.
///
/// Returns `Some((ack_frame, bytes_consumed))` if successful.
pub fn parse_ack(buf: &[u8]) -> Option<(AckFrame, usize)> {
    // Find header
    let header_pos = find_sequence(buf, &CMD_HEADER)?;
    let buf = &buf[header_pos..];

    // Need at least header(4) + length(2) + cmd(2) + status(2) + footer(4) = 14
    if buf.len() < 14 {
        return None;
    }

    let data_len = u16::from_le_bytes([buf[4], buf[5]]) as usize;
    let frame_len = 4 + 2 + data_len + 4;

    if buf.len() < frame_len {
        return None;
    }

    // Verify footer
    if buf[frame_len - 4..frame_len] != CMD_FOOTER {
        return None;
    }

    let inframe = &buf[6..6 + data_len];

    // ACK inframe: command_word(2) | status(2) | [extra data...]
    // Note: ACK command word has bit 0 of high byte set (e.g. 0xFF01 for cmd 0x00FF)
    if inframe.len() < 4 {
        return None;
    }

    let ack_cmd_word = u16::from_le_bytes([inframe[0], inframe[1]]);
    let status_val = u16::from_le_bytes([inframe[2], inframe[3]]);
    let status = if status_val == 0 {
        AckStatus::Success
    } else {
        AckStatus::Failure
    };

    // Original command word: clear the ACK bit (bit 8)
    let command_word = ack_cmd_word & !0x0100;
    let extra = &inframe[4..];

    let data = match command_word {
        0x00FF if extra.len() >= 4 => AckData::EnableConfig {
            protocol_version: u16::from_le_bytes([extra[0], extra[1]]),
            buffer_size: u16::from_le_bytes([extra[2], extra[3]]),
        },
        0x0091 if extra.len() >= 2 => {
            let mode_val = u16::from_le_bytes([extra[0], extra[1]]);
            match TrackingMode::from_u16(mode_val) {
                Some(mode) => AckData::TrackingMode(mode),
                None => AckData::Simple,
            }
        }
        0x00A0 if extra.len() >= 8 => AckData::FirmwareVersion {
            fw_type: u16::from_le_bytes([extra[0], extra[1]]),
            major: u16::from_le_bytes([extra[2], extra[3]]),
            minor: u32::from_le_bytes([extra[4], extra[5], extra[6], extra[7]]),
        },
        0x00A5 if extra.len() >= 4 => {
            // 1 byte type (0x00) + 3 bytes MAC... actually protocol says 6 bytes for MAC
            // The doc shows: type(1) + mac(3) but real devices use 6 byte MAC
            // Let's handle both cases
            let mut mac = [0u8; 6];
            let mac_start = 1; // skip type byte
            let available = extra.len().saturating_sub(mac_start).min(6);
            mac[..available].copy_from_slice(&extra[mac_start..mac_start + available]);
            AckData::MacAddress(mac)
        }
        0x00C1 if extra.len() >= 26 => {
            let ft = u16::from_le_bytes([extra[0], extra[1]]);
            let filter_type = ZoneFilterType::from_u16(ft).unwrap_or(ZoneFilterType::Disabled);
            let zones = parse_zones(&extra[2..26]);
            AckData::ZoneFilter { filter_type, zones }
        }
        _ => AckData::Simple,
    };

    Some((
        AckFrame {
            command_word,
            status,
            data,
        },
        header_pos + frame_len,
    ))
}

fn parse_zones(data: &[u8]) -> [ZoneRect; 3] {
    let mut zones = [ZoneRect::default(); 3];
    for (i, zone) in zones.iter_mut().enumerate() {
        let off = i * 8;
        zone.x1 = i16::from_le_bytes([data[off], data[off + 1]]);
        zone.y1 = i16::from_le_bytes([data[off + 2], data[off + 3]]);
        zone.x2 = i16::from_le_bytes([data[off + 4], data[off + 5]]);
        zone.y2 = i16::from_le_bytes([data[off + 6], data[off + 7]]);
    }
    zones
}

fn find_sequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_enable_config() {
        let frame = CommandFrame::build(&Command::EnableConfig);
        let expected: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, // header
            0x04, 0x00, // data length = 4
            0xFF, 0x00, // command word
            0x01, 0x00, // command value
            0x04, 0x03, 0x02, 0x01, // footer
        ];
        assert_eq!(frame.as_bytes(), expected);
    }

    #[test]
    fn build_end_config() {
        let frame = CommandFrame::build(&Command::EndConfig);
        let expected: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, // header
            0x02, 0x00, // data length = 2
            0xFE, 0x00, // command word
            0x04, 0x03, 0x02, 0x01, // footer
        ];
        assert_eq!(frame.as_bytes(), expected);
    }

    #[test]
    fn build_single_target_tracking() {
        let frame = CommandFrame::build(&Command::SingleTargetTracking);
        let expected: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, 0x02, 0x00, 0x80, 0x00, 0x04, 0x03, 0x02, 0x01,
        ];
        assert_eq!(frame.as_bytes(), expected);
    }

    #[test]
    fn build_set_baud_256000() {
        let frame = CommandFrame::build(&Command::SetBaudRate(BaudRateIndex::B256000));
        let expected: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, 0x04, 0x00, 0xA1, 0x00, 0x07, 0x00, 0x04, 0x03, 0x02, 0x01,
        ];
        assert_eq!(frame.as_bytes(), expected);
    }

    #[test]
    fn parse_ack_enable_config() {
        let ack_bytes: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, // header
            0x08, 0x00, // data length = 8
            0xFF, 0x01, // ack command word (0x00FF | 0x0100)
            0x00, 0x00, // status: success
            0x01, 0x00, // protocol version
            0x40, 0x00, // buffer size
            0x04, 0x03, 0x02, 0x01, // footer
        ];

        let (ack, consumed) = parse_ack(ack_bytes).unwrap();
        assert_eq!(consumed, ack_bytes.len());
        assert_eq!(ack.command_word, 0x00FF);
        assert_eq!(ack.status, AckStatus::Success);
        match ack.data {
            AckData::EnableConfig {
                protocol_version,
                buffer_size,
            } => {
                assert_eq!(protocol_version, 0x0001);
                assert_eq!(buffer_size, 0x0040);
            }
            _ => panic!("expected EnableConfig ack data"),
        }
    }

    #[test]
    fn parse_ack_end_config() {
        let ack_bytes: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, 0x04, 0x00, 0xFE, 0x01, 0x00, 0x00, 0x04, 0x03, 0x02, 0x01,
        ];

        let (ack, _) = parse_ack(ack_bytes).unwrap();
        assert_eq!(ack.command_word, 0x00FE);
        assert_eq!(ack.status, AckStatus::Success);
    }

    #[test]
    fn parse_ack_tracking_mode_single() {
        let ack_bytes: &[u8] = &[
            0xFD, 0xFC, 0xFB, 0xFA, 0x06, 0x00, 0x91, 0x01, 0x00, 0x00, 0x01,
            0x00, // single target
            0x04, 0x03, 0x02, 0x01,
        ];

        let (ack, _) = parse_ack(ack_bytes).unwrap();
        assert_eq!(ack.command_word, 0x0091);
        match ack.data {
            AckData::TrackingMode(mode) => assert_eq!(mode, TrackingMode::Single),
            _ => panic!("expected TrackingMode"),
        }
    }

    #[test]
    fn parse_ack_with_garbage_prefix() {
        let mut data = vec![0x00, 0xFF, 0x42]; // garbage
        data.extend_from_slice(&[
            0xFD, 0xFC, 0xFB, 0xFA, 0x04, 0x00, 0xFE, 0x01, 0x00, 0x00, 0x04, 0x03, 0x02, 0x01,
        ]);

        let (ack, _) = parse_ack(&data).unwrap();
        assert_eq!(ack.command_word, 0x00FE);
    }
}
