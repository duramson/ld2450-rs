# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2025-04-18

### Changed

- **BREAKING** `ld2450-proto`: removed `Target::distance_mm()`, replaced with `Target::dist_m()` returning metres
- **BREAKING** `ld2450d`: JSON output now uses SI units throughout — all coordinates, speed, and distance are in metres / m/s (was mm / cm/s)
- **BREAKING** `ld2450d`: JSON field `dist_mm` → `dist`, `angle_deg` → `angle`, `dist_res` removed

### Added

- `ld2450-proto`: new convenience methods `Target::x_m()`, `Target::y_m()`, `Target::speed_ms()` for SI-unit access
- `ld2450-proto`: crates.io metadata (keywords, categories, README)

### Fixed

- Removed unused `SerialPort` trait import in `ld2450d`

## [0.1.0] - 2025-04-13

### Added

- `ld2450-proto`: Zero-allocation frame parser and command builder (`no_std`-compatible)
- `ld2450d`: Async streaming daemon (UART to Unix socket, JSON-Lines at 10 Hz)
- `ld2450-ctl`: CLI tool for sensor configuration (firmware, tracking mode, baud rate, zones, Bluetooth, MAC)
- systemd service unit with security hardening
- Cross-compilation support via `cross` (aarch64-unknown-linux-gnu)
