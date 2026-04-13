# ld2450-proto

Zero-allocation protocol parser and command builder for the **HLK-LD2450** 24 GHz mmWave radar sensor.

Part of the [ld2450-rs](https://github.com/duramson/ld2450-rs) project.

## Features

- Streaming frame parser (state machine, no heap allocation)
- Command frame builder for all sensor configuration commands
- ACK response parser with typed data extraction
- `no_std`-compatible (uses `libm` for math)
- Optional `std` feature for `feed_slice()` convenience method

## Usage

```rust
use ld2450_proto::{FrameParser, ParseEvent};

let mut parser = FrameParser::new();

// Feed bytes from UART one at a time or in slices
for &byte in uart_data {
    if let Some(ParseEvent::Frame(frame)) = parser.feed(byte) {
        for target in frame.active_targets() {
            println!(
                "x={}mm y={}mm speed={}cm/s dist={:.1}mm",
                target.x(),
                target.y(),
                target.speed(),
                target.distance_mm(),
            );
        }
    }
}
```

## Protocol

Each radar data frame is 30 bytes:

```
Header:  AA FF 03 00   (4 bytes)
Target1: [8 bytes]     x(2) + y(2) + speed(2) + resolution(2)
Target2: [8 bytes]
Target3: [8 bytes]
Footer:  55 CC         (2 bytes)
```

Up to 3 targets tracked simultaneously at 10 Hz.

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE)
or [MIT License](../../LICENSE-MIT) at your option.
