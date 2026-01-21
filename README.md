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
  "effects": ["replace", "blur"], // Options: "blur", "replace"
  "gpu_backend": "nvidia" // Options: "nvidia", "intel", "cpu", "auto"
}
```

- **effects**: A list of active effects. Combined to define the pipeline behavior.
- **gpu_backend**:
    - **nvidia**: Uses `nvvideoconvert` (CUDA/NVENC).
    - **intel/amd**: Uses `vaapipostproc` (VA-API).
    - **npu**: Accelerates AI inference via ONNX Runtime.
    - **auto**: Defaults to generic `videoscale` (CPU).
    
    *Note: The Gnome Extension menu will automatically filter these options to show only what is detected on your system.*

### Troubleshooting Drivers

If **AMD** or **Intel** options do not appear in the menu, you are likely missing the GStreamer VA-API plugins.

**Debian/Ubuntu:**
```bash
sudo apt install gstreamer1.0-vaapi mesa-va-drivers
```

**Fedora:**
```bash
sudo dnf install gstreamer1-vaapi libva-utils
```

**Arch:**
```bash
sudo pacman -S gstreamer-vaapi libva-mesa-driver
```

## Status & Performance

The Extension now displays real-time status of the pipeline, including:
- **Active Backend**: Shows which technology is properly loaded (e.g., "Nvidia CUDA", "AMD VA-API").
- **Latency**: Real-time processing frame measurements in ms/microseconds.
- **ONNX Runtime**: Ensuring AI models run on the best available provider (TensorRT, OpenVINO, ROCm).


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
