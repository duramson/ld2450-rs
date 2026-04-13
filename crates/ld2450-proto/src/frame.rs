use crate::types::RadarFrame;

/// Data frame header: AA FF 03 00
const DATA_HEADER: [u8; 4] = [0xAA, 0xFF, 0x03, 0x00];
/// Data frame footer: 55 CC
const DATA_FOOTER: [u8; 2] = [0x55, 0xCC];

/// Total payload size: 3 targets × 8 bytes each
const PAYLOAD_LEN: usize = 24;
/// Events emitted by the frame parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseEvent {
    /// A complete, valid radar data frame was parsed.
    Frame(RadarFrame),
}

/// Parser states.
#[derive(Debug, Clone, Copy)]
enum State {
    /// Scanning for the header sequence.
    SearchHeader { matched: u8 },
    /// Reading payload bytes.
    ReadPayload { pos: u8 },
    /// Reading footer bytes.
    ReadFooter { matched: u8 },
}

/// Streaming frame parser for LD2450 radar data output.
///
/// Feed bytes one at a time or in chunks. The parser emits `ParseEvent::Frame`
/// for each valid frame found in the byte stream.
///
/// This is a zero-allocation state machine suitable for `no_std` environments.
#[derive(Debug)]
pub struct FrameParser {
    state: State,
    buf: [u8; PAYLOAD_LEN],
}

impl FrameParser {
    pub fn new() -> Self {
        Self {
            state: State::SearchHeader { matched: 0 },
            buf: [0u8; PAYLOAD_LEN],
        }
    }

    /// Feed a single byte to the parser. Returns `Some(ParseEvent)` if a
    /// complete frame was parsed.
    #[inline]
    pub fn feed(&mut self, byte: u8) -> Option<ParseEvent> {
        match self.state {
            State::SearchHeader { matched } => {
                if byte == DATA_HEADER[matched as usize] {
                    let next = matched + 1;
                    if next as usize == DATA_HEADER.len() {
                        self.state = State::ReadPayload { pos: 0 };
                    } else {
                        self.state = State::SearchHeader { matched: next };
                    }
                } else if byte == DATA_HEADER[0] {
                    // Could be start of a new header
                    self.state = State::SearchHeader { matched: 1 };
                } else {
                    self.state = State::SearchHeader { matched: 0 };
                }
                None
            }
            State::ReadPayload { pos } => {
                self.buf[pos as usize] = byte;
                let next = pos + 1;
                if next as usize == PAYLOAD_LEN {
                    self.state = State::ReadFooter { matched: 0 };
                } else {
                    self.state = State::ReadPayload { pos: next };
                }
                None
            }
            State::ReadFooter { matched } => {
                if byte == DATA_FOOTER[matched as usize] {
                    let next = matched + 1;
                    if next as usize == DATA_FOOTER.len() {
                        // Complete frame!
                        self.state = State::SearchHeader { matched: 0 };
                        let frame = RadarFrame::from_bytes(&self.buf);
                        return Some(ParseEvent::Frame(frame));
                    } else {
                        self.state = State::ReadFooter { matched: next };
                    }
                } else {
                    // Invalid footer — discard and resync
                    self.state = State::SearchHeader { matched: 0 };
                }
                None
            }
        }
    }

    /// Feed a slice of bytes, collecting all parsed events.
    /// For performance-critical paths, prefer calling `feed()` in a loop
    /// and handling events inline.
    #[cfg(feature = "std")]
    pub fn feed_slice(&mut self, data: &[u8]) -> Vec<ParseEvent> {
        let mut events = Vec::new();
        for &b in data {
            if let Some(ev) = self.feed(b) {
                events.push(ev);
            }
        }
        events
    }
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(payload: &[u8; 24]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(30);
        frame.extend_from_slice(&DATA_HEADER);
        frame.extend_from_slice(payload);
        frame.extend_from_slice(&DATA_FOOTER);
        frame
    }

    #[test]
    fn parse_datasheet_example() {
        // From protocol doc example
        let raw: [u8; 30] = [
            0xAA, 0xFF, 0x03, 0x00, // header
            0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01, // target 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // target 2 (empty)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // target 3 (empty)
            0x55, 0xCC, // footer
        ];

        let mut parser = FrameParser::new();
        let events = parser.feed_slice(&raw);
        assert_eq!(events.len(), 1);

        let ParseEvent::Frame(frame) = &events[0];
        assert_eq!(frame.targets[0].x, -782);
        assert_eq!(frame.targets[0].y, 1713);
        assert_eq!(frame.targets[0].speed, -16);
        assert_eq!(frame.targets[0].distance_resolution, 320);
        assert!(frame.targets[1].is_empty());
        assert!(frame.targets[2].is_empty());
        assert_eq!(frame.active_count(), 1);
    }

    #[test]
    fn parse_with_garbage_prefix() {
        let mut data = vec![0xFF, 0x00, 0x42, 0x13]; // garbage
        let mut payload = [0u8; 24];
        payload[..8].copy_from_slice(&[0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01]);
        data.extend_from_slice(&make_frame(&payload));

        let mut parser = FrameParser::new();
        let events = parser.feed_slice(&data);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn parse_multiple_frames() {
        let mut payload = [0u8; 24];
        payload[..8].copy_from_slice(&[0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01]);
        let frame = make_frame(&payload);

        let mut data = Vec::new();
        data.extend_from_slice(&frame);
        data.extend_from_slice(&frame);
        data.extend_from_slice(&frame);

        let mut parser = FrameParser::new();
        let events = parser.feed_slice(&data);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn bad_footer_resyncs() {
        let mut data = Vec::new();
        // Frame with bad footer
        data.extend_from_slice(&DATA_HEADER);
        data.extend_from_slice(&[0u8; 24]);
        data.extend_from_slice(&[0x55, 0xDD]); // wrong footer

        // Followed by a good frame
        let mut payload = [0u8; 24];
        payload[..8].copy_from_slice(&[0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01]);
        data.extend_from_slice(&make_frame(&payload));

        let mut parser = FrameParser::new();
        let events = parser.feed_slice(&data);
        assert_eq!(events.len(), 1); // only the good frame
    }

    #[test]
    fn byte_by_byte_feeding() {
        let mut payload = [0u8; 24];
        payload[..8].copy_from_slice(&[0x0E, 0x03, 0xB1, 0x86, 0x10, 0x00, 0x40, 0x01]);
        let data = make_frame(&payload);

        let mut parser = FrameParser::new();
        let mut events = Vec::new();
        for &b in &data {
            if let Some(ev) = parser.feed(b) {
                events.push(ev);
            }
        }
        assert_eq!(events.len(), 1);
    }
}
