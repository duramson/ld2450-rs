/// A single tracked target reported by the radar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Target {
    /// X coordinate in mm. Positive = right of sensor, negative = left.
    pub x: i16,
    /// Y coordinate in mm. Always positive (in front of sensor).
    pub y: i16,
    /// Speed in cm/s. Positive = approaching, negative = receding.
    pub speed: i16,
    /// Distance gate resolution in mm.
    pub distance_resolution: u16,
}

impl Target {
    /// Returns true if this target slot is empty (all zeros).
    pub fn is_empty(&self) -> bool {
        self.x == 0 && self.y == 0 && self.speed == 0 && self.distance_resolution == 0
    }

    /// Distance from sensor in mm.
    pub fn distance_mm(&self) -> f32 {
        let x = self.x as f32;
        let y = self.y as f32;
        libm::sqrtf(x * x + y * y)
    }

    /// Angle in degrees from sensor boresight (-90 to +90).
    pub fn angle_deg(&self) -> f32 {
        libm::atan2f(self.x as f32, self.y as f32) * (180.0 / core::f32::consts::PI)
    }

    /// Parse a target from 8 bytes of in-frame data.
    ///
    /// Byte layout (little-endian):
    /// [0..2] x coordinate (signed, bit15: 1=positive, 0=negative)
    /// [2..4] y coordinate (signed, bit15: 1=positive, 0=negative)
    /// [4..6] speed (signed, bit15: 1=positive/approaching, 0=negative/receding)
    /// [6..8] distance resolution (unsigned)
    pub fn from_bytes(data: &[u8; 8]) -> Self {
        Self {
            x: decode_coord(u16::from_le_bytes([data[0], data[1]])),
            y: decode_coord(u16::from_le_bytes([data[2], data[3]])),
            speed: decode_coord(u16::from_le_bytes([data[4], data[5]])),
            distance_resolution: u16::from_le_bytes([data[6], data[7]]),
        }
    }
}

/// Decode the LD2450's signed format:
/// Bit 15 = 1 → positive value (bits 0-14)
/// Bit 15 = 0 → negative value (negate bits 0-14)
fn decode_coord(raw: u16) -> i16 {
    let magnitude = (raw & 0x7FFF) as i16;
    if raw & 0x8000 != 0 {
        magnitude
    } else {
        -magnitude
    }
}

/// A complete radar data frame containing up to 3 targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RadarFrame {
    pub targets: [Target; 3],
}

impl RadarFrame {
    /// Number of active (non-empty) targets.
    pub fn active_count(&self) -> usize {
        self.targets.iter().filter(|t| !t.is_empty()).count()
    }

    /// Parse from 24 bytes of payload (3 × 8 bytes per target).
    pub fn from_bytes(data: &[u8; 24]) -> Self {
        Self {
            targets: [
                Target::from_bytes(data[0..8].try_into().unwrap()),
                Target::from_bytes(data[8..16].try_into().unwrap()),
                Target::from_bytes(data[16..24].try_into().unwrap()),
            ],
        }
    }
}

/// Tracking mode of the sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TrackingMode {
    Single = 0x0001,
    Multi = 0x0002,
}

impl TrackingMode {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0x0001 => Some(Self::Single),
            0x0002 => Some(Self::Multi),
            _ => None,
        }
    }
}

/// Zone filtering type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ZoneFilterType {
    Disabled = 0x0000,
    DetectOnly = 0x0001,
    Exclude = 0x0002,
}

impl ZoneFilterType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0x0000 => Some(Self::Disabled),
            0x0001 => Some(Self::DetectOnly),
            0x0002 => Some(Self::Exclude),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_coord_positive() {
        // 0x8000 | 782 = 0x830E → bit15 set → positive 782
        assert_eq!(decode_coord(0x830E), 782);
    }

    #[test]
    fn decode_coord_negative() {
        // 782 without bit15 → negative
        assert_eq!(decode_coord(0x030E), -782);
    }

    #[test]
    fn target_from_datasheet_example() {
        // From protocol doc: 0E 03 B1 86 10 00 40 01
        // x: 0x030E = 782, bit15=0 → -782mm
        // y: 0x86B1 = 34481, bit15=1 → 34481-32768 = 1713mm
        // speed: 0x0010 = 16, bit15=0 → -16 cm/s
        // resolution: 0x0140 = 320mm
        let data = [0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01];
        let t = Target::from_bytes(&data);
        assert_eq!(t.x, -782);
        assert_eq!(t.y, 1713);
        assert_eq!(t.speed, -16);
        assert_eq!(t.distance_resolution, 320);
    }

    #[test]
    fn empty_target() {
        let t = Target::from_bytes(&[0; 8]);
        assert!(t.is_empty());
    }

    #[test]
    fn radar_frame_active_count() {
        let mut payload = [0u8; 24];
        // Only target 1 has data
        payload[..8].copy_from_slice(&[0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01]);
        let frame = RadarFrame::from_bytes(&payload);
        assert_eq!(frame.active_count(), 1);
        assert!(!frame.targets[0].is_empty());
        assert!(frame.targets[1].is_empty());
        assert!(frame.targets[2].is_empty());
    }
}
