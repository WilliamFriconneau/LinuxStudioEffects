# Linux Studio Effects

A Linux background effects system similar to macOS Studio Light/Portrait effects, leveraging GStreamer and ONNX Runtime for background segmentation and processing.

## Components

- **Daemon (Rust):** Handles the video processing pipeline, applying effects like background blur or replacement.
- **Gnome Extension:** Provides a system menu integration to control the effects.

## Energy Efficiency & Hardware Acceleration

The project is designed with a "Green" efficiency-first mindset.

- **GPU Acceleration**: Supports Nvidia (`nvvideoconvert`) and Intel (`vaapipostproc`) backends to offload video scaling and processing, drastically reducing CPU usage.
- **Zero-Overhead Idle**: The pipeline automatically destroys itself when no application is consuming the video stream, ensuring 0% CPU/GPU usage when idle.
- **Fail-Safe Mode**: If AI processing is too heavy or fails, the system seamlessly falls back to a "Safety" pass-through path.

## Prerequisites

- Rust (cargo)
- GStreamer development libraries:
  - `gstreamer1.0-plugins-base`
  - `gstreamer1.0-plugins-good`
  - `gstreamer1.0-plugins-bad` (for advanced scalers)
  - `gstreamer1.0-libav`
  - `gstreamer1.0-vaapi` (for Intel GPU)
- GTK4 development libraries

## GPU Configuration

You can configure the GPU backend in `~/.config/linux-studio-effects/state.json`:

```json
{
  "gpu_backend": "nvidia" // Options: "nvidia", "intel", "cpu", "auto"
}
```

- **nvidia**: Uses `nvvideoconvert`. Requires proprietary drivers and GStreamer NVENC/DEC plugins.
- **intel**: Uses `vaapipostproc`. Requires `gstreamer1.0-vaapi`.
- **auto**: Defaults to generic `videoscale` (CPU) if not specified, but will aim to detect hardware in future updates.

## Current Status

- [x] Basic Pipeline (Camera -> Virtual Output)
- [x] "Safety" Path Fallback
- [x] Background Blur / Replacement scaffolding
- [/] AI Model Integration (ONNX Runtime)
- [x] GPU Backend Selection


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
