set dotenv-load

target := env_var_or_default("TARGET", "aarch64-unknown-linux-gnu")
host := env_var_or_default("DEPLOY_HOST", "root@10.0.90.240")

# Build release for target architecture
build:
    cross build --target {{target}} --release

# Run tests
test:
    cargo test --all

# Clippy lint
lint:
    cargo clippy --all-targets -- -D warnings

# Format check
fmt:
    cargo fmt --check

# Deploy binaries + config to remote host via scp
deploy: build
    scp target/{{target}}/release/ld2450d {{host}}:/usr/local/bin/
    scp target/{{target}}/release/ld2450-ctl {{host}}:/usr/local/bin/
    scp config/ld2450d.toml {{host}}:/etc/ld2450d.toml
    scp deploy/ld2450d.service {{host}}:/etc/systemd/system/
    scp deploy/ld2450d.tmpfiles {{host}}:/etc/tmpfiles.d/ld2450d.conf
    ssh {{host}} "systemd-tmpfiles --create && systemctl daemon-reload && systemctl enable --now ld2450d"

# Stream radar data from remote host (for testing)
stream:
    ssh {{host}} "socat - UNIX-CONNECT:/run/ld2450/radar.sock"
