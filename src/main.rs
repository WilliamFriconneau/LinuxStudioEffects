use anyhow::{Context, Result};
use gstreamer::{MessageView, State};
use gstreamer::prelude::*;
use log::{info, error, warn, debug};
use notify::{Watcher, RecursiveMode, Event};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod pipeline;
use pipeline::StudioPipeline;

const CONFIG_PATH: &str = "/Users/wiwi/.config/linux-studio-effects/state.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AppConfig {
    active: bool,
    #[serde(default = "default_camera_priority")]
    pub camera_priority: Vec<String>,
    blur_strength: f32,
    sabotage: String,
    lighting_boost: bool,
    #[serde(default = "default_effects")]
    pub effects: Vec<String>, // Replaces single string mode. e.g. ["replace", "blur"]
    #[serde(default)]
    background_image: String,
    #[serde(default = "default_gpu_backend")]
    pub gpu_backend: String,
}

#[derive(Debug, Serialize)]
struct StatusReport {
    backend: String,
    tech: String, // e.g. "Nvidia CUDA", "Intel QuickSync", "CPU"
    active_effects: Vec<String>,
    fps: u32,
    latency_us: u64,
}

fn default_effects() -> Vec<String> {
    vec!["blur".to_string()]
}

fn default_gpu_backend() -> String {
    "auto".to_string()
}

fn default_camera_priority() -> Vec<String> {
    vec!["/dev/video0".to_string(), "/dev/video2".to_string()]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active: true,
            camera_priority: default_camera_priority(),
            blur_strength: 0.5,
            sabotage: "none".to_string(),
            lighting_boost: false,
            effects: default_effects(),
            background_image: "".to_string(),
            gpu_backend: default_gpu_backend(),
        }
    }
}

fn load_config(path: &Path) -> AppConfig {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return cfg;
        } else {
             warn!("Failed to parse config, using defaults.");
        }
    }
    AppConfig::default()
}

fn write_status_report(path: &Path, report: &StatusReport) {
    if let Ok(json) = serde_json::to_string_pretty(report) {
        let _ = fs::write(path, json);
    }
}

fn is_sink_active(device_path: &str) -> bool {
    // Quick check using lsof, optimized
    let output = Command::new("lsof")
        .arg(device_path)
        .output();
        
    match output {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => true, 
    }
}

// Helper to find the first available camera from priority list
fn find_best_camera(priority: &[String]) -> Option<String> {
    for cam in priority {
        if Path::new(cam).exists() {
            return Some(cam.clone());
        }
    }
    None
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting LinuxStudioEffects Daemon (Priority Mode)");

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let config_dir = Path::new(&home).join(".config/linux-studio-effects");
    let config_path = config_dir.join("state.json");
    let status_path = config_dir.join("state_report.json");
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if !config_path.exists() {
        let def = AppConfig::default();
        let _ = fs::write(&config_path, serde_json::to_string_pretty(&def)?);
    }

    let current_config = Arc::new(Mutex::new(load_config(&config_path)));
    
    // Channel for config updates
    let (tx_config, rx_config) = std::sync::mpsc::channel();
    let tx_clone = tx_config.clone();
    
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        match res {
            Ok(_) => { let _ = tx_clone.send(()); },
            Err(e) => error!("Watch error: {:?}", e),
        }
    })?;
    
    watcher.watch(&config_path, RecursiveMode::NonRecursive)?;


    // We need a loop that rebuilds the pipeline if it fails or if scans change
    // Using an Option<Pipeline> wrapper
    
    // NOTE: std::sync::mpsc::Receiver cannot be cloned easily for unrelated threads.
    // The "Idle Monitor" thread needs access to the pipeline.
    // The "Main Loop" needs to handle Bus messages.
    // We'll wrap the pipeline in Arc<Mutex<Option<StudioPipeline>>> so both can access/rebuild. 
    // Actually, rebuilding in main thread is safer. Idle thread just reports or requests state.
    
    // Let's stick to: Main thread manages pipeline lifecycle (create/destroy).
    // Idle thread just checks sink status and sends a "ShouldRun" command? 
    // Or Idle thread just updates a shared atomic bool "sink_active" and main thread applies it?
    
    // New design for robustness:
    // Shared State: AppState { config, sink_active }
    // Main Loop: 
    //   - select! (pseudo)
    //     - Config Update -> update shared state, maybe reconfigure pipeline
    //     - Bus Message -> handle error -> trigger REBUILD
    //     - Idle Check Timer -> update shared state "sink_active"
    //     - Maintenance Timer -> Apply state (Start/Stop/Reconfigure)
    
    let pipeline_store: Arc<Mutex<Option<StudioPipeline>>> = Arc::new(Mutex::new(None));
    let pipeline_store_clone = pipeline_store.clone();
    
    // Idle Monitor Thread
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            let active = is_sink_active("/dev/video1");
            
            // Access pipeline to update valid state if running
            // If active changed, main loop logic handles start/stop? 
            // Currently pipeline has logic `start` / `stop`.
            // We can just call it here if we have part ownership?
            // Safer to just set a flag, but for simplicity let's try to acquire lock and act.
            
            // To properly coordinate, let's just make `is_sink_active` available to main loop or Main handles it.
            // But main loop is blocked on Bus.
            
            // Actually, GStreamer bus poll can have a timeout.
        }
    });

    // We will do everything in ONE main loop with short timeouts.
    // This avoids thread contention on the pipeline.
    
    let mut last_cam: Option<String> = None;
    
    // Shared metrics
    let latency_us = Arc::new(std::sync::atomic::AtomicU64::new(0));

    loop {
        // 1. Load Config (if changed)
        if let Ok(_) = rx_config.try_recv() {
            info!("Config file changed. Reloading...");
            *current_config.lock().unwrap() = load_config(&config_path);
        }
        
        // 2. Determine "Active" Requirement
        let cfg = current_config.lock().unwrap().clone();
        let sink_active = is_sink_active("/dev/video1");
        let should_run = cfg.active && sink_active;

        // 3. Manage Pipeline Existence
        {
            let mut pl_opt = pipeline_store.lock().unwrap();
            
            // If we should run but have no pipeline, try creating one
            if should_run && pl_opt.is_none() {
                if let Some(cam) = find_best_camera(&cfg.camera_priority) {
                    info!("Constructing pipeline with camera: {} (GPU: {})", cam, cfg.gpu_backend);
                    // Pass metrics to pipeline
                    match StudioPipeline::new(&cam, &cfg.gpu_backend, latency_us.clone()) {
                        Ok(p) => {
                            let _ = p.start(); // Start immediately if created
                            *pl_opt = Some(p);
                            last_cam = Some(cam);
                        }
                        Err(e) => {
                            error!("Failed to create pipeline: {}", e);
                            thread::sleep(Duration::from_millis(500)); // Backoff
                        }
                    }
                } else {
                    warn!("No cameras found from priority list! Waiting...");
                }
            }
            // If we should NOT run, but have pipeline -> Stop/Destroy? Or just Stop?
            // "Pipeline must enter NULL state (releasing physical camera)"
            // Destroying is safest for zero-overhead.
            else if !should_run && pl_opt.is_some() {
                 info!("Idle or Disabled. Destroying pipeline to release resources.");
                 if let Some(p) = pl_opt.take() {
                     let _ = p.stop();
                 }
                 // pl_opt is now None.
            }
            
            // If running, apply dynamic config
            if let Some(p) = pl_opt.as_ref() {
                let _ = p.apply_config(
                    cfg.active, 
                    cfg.blur_strength, 
                    &cfg.sabotage,
                    &cfg.effects,
                    &cfg.background_image
                );
            }
        }

        // 3.5 Status Reporting
        let tech_desc = match cfg.gpu_backend.as_str() {
            "nvidia" => "Nvidia CUDA (nvvideoconvert)",
            "intel" => "Intel QuickSync (vaapipostproc)",
            "amd" => "AMD VA-API (vaapipostproc)",
            "npu" => "NPU (Accelerated)",
            "cpu" => "CPU (videoscale)",
            "auto" | _ => "Auto/CPU",
        };
        
        // Read metrics
        let current_latency = latency_us.load(std::sync::atomic::Ordering::Relaxed);

        // Active effects desc
        let report = StatusReport {
            backend: cfg.gpu_backend.clone(),
            tech: tech_desc.to_string(),
            active_effects: if sink_active { cfg.effects.clone() } else { vec!["idle".to_string()] },
            fps: 30, // Dummy for now
            latency_us: current_latency,
        };
        write_status_report(&status_path, &report);
        
        // 4. Handle GStreamer Bus (if pipeline exists)
        let mut error_needs_rebuild = false;
        
        if let Some(lock) = pipeline_store.lock().unwrap().as_ref() {
            if let Some(bus) = lock.pipeline.bus() { // pipeline.bus() return is cheap
                match bus.timed_pop(Some(Duration::from_millis(100))) {
                     Some(msg) => {
                         match msg.view() {
                             MessageView::Error(err) => {
                                 error!("Pipeline Error: {} ({:?})", err.error(), err.debug());
                                 error_needs_rebuild = true;
                             }
                             MessageView::Eos(..) => {
                                 info!("EOS encountered. Restarting...");
                                 error_needs_rebuild = true;
                             }
                             _ => (),
                         }
                     }
                     None => (),
                }
            }
        } else {
            // No pipeline, sleep a bit to avoid CPU spin
            thread::sleep(Duration::from_millis(500));
        }
        
    // 5. Device Monitor for Hotplug (Upgrading)
    // We want to detect if a higher priority camera was plugged in.
    // GstDeviceMonitor is useful, but simple polling of /dev/video* existence matches our "Zero-Overhead" manual Priority check philosophy 
    // without complex GObject signals in the main loop (which might require a MainLoop/GLib context).
    // Given the constraints and the loop structure, we can check for "better" cameras periodically.
    
    // Check for upgrade every 2 seconds?
    // "higher priority" means lower index in `cfg.camera_priority`.
    
    if let Some(current_cam) = last_cam.clone() {
        if let Some(current_idx) = cfg.camera_priority.iter().position(|x| x == &current_cam) {
            // Check if any camera with index < current_idx is now available
            for i in 0..current_idx {
                let candidate = &cfg.camera_priority[i];
                if Path::new(candidate).exists() {
                    info!("Higher priority camera found: {}. Switching...", candidate);
                    // Force rebuild
                    error_needs_rebuild = true;
                    break;
                }
            }
        } else {
             // Current cam not in priority list (maybe config changed?), scanning might pick a better one.
             // Or maybe we just stick with it until it fails? 
             // Let's re-scan to enforce priority.
             if let Some(best) = find_best_camera(&cfg.camera_priority) {
                 if best != current_cam {
                      info!("Config change or Better camera found: {}. Switching...", best);
                      error_needs_rebuild = true;
                 }
             }
        }
    }

    if error_needs_rebuild {
        info!("Triggering Pipeline Rebuild/Rescan...");
        let mut pl_opt = pipeline_store.lock().unwrap();
        if let Some(p) = pl_opt.take() {
            let _ = p.stop();
        }
        last_cam = None; // Force full scan next loop
    }
}
