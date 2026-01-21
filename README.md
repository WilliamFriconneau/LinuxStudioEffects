# Linux Studio Effects

A Linux background effects system similar to macOS Studio Light/Portrait effects, leveraging GStreamer and ONNX Runtime for background segmentation and processing.

## Components

- **Daemon (Rust):** Handles the video processing pipeline, applying effects like background blur or replacement.
- **Gnome Extension:** Provides a system menu integration to control the effects.

## Prerequisites

- Rust (cargo)
- GStreamer development libraries (`gstreamer1.0-plugins-base`, `gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-bad`, `gstreamer1.0-libav`)
- GTK4 development libraries

## Build & Install

A helper script is provided to build the project:

```bash
chmod +x build.sh
./build.sh
```

This script will:
1. Build the Rust binary in release mode.
2. (Optional) Zip the Gnome extension for installation.

## Manual Usage

Run the daemon:

```bash
cargo run --release
```
