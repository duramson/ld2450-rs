# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2025-04-13

### Added

- `ld2450-proto`: Zero-allocation frame parser and command builder (`no_std`-compatible)
- `ld2450d`: Async streaming daemon (UART to Unix socket, JSON-Lines at 10 Hz)
- `ld2450-ctl`: CLI tool for sensor configuration (firmware, tracking mode, baud rate, zones, Bluetooth, MAC)
- systemd service unit with security hardening
- Cross-compilation support via `cross` (aarch64-unknown-linux-gnu)
