use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box, Orientation, Scale, Label, Button, Adjustment, Entry};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
// For preview
// We need gstreamer to push to gtksink.
// But for simplicity, we might just spawn a gst-launch window or embed it if possible.
// Embedding gst in gtk4 requires `gst-plugin-gtk4`.
// A "Petite application" can just have controls for now, or use a separate Viewport.
// Implementing full embedded GStreamer GTK4 sink in a single file without crate setup for complex build is tricky.
// We will focus on the CONFIG EDITOR first, as requested ("gère aussi toutes ces fonctions d'édition du fichier de configuration").
// The user also asked for "rendu en temps réel".
// We can try to use `Picture` with a `Paintable` from GStreamer, but let's stick to the controls first to ensure stability.

const CONFIG_PATH: &str = "/Users/wiwi/.config/linux-studio-effects/state.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppConfig {
    active: bool,
    camera_priority: Vec<String>,
    blur_strength: f32,
    sabotage: String,
    lighting_boost: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active: true,
            camera_priority: vec!["/dev/video0".to_string()],
            blur_strength: 0.5,
            sabotage: "none".to_string(),
            lighting_boost: false,
        }
    }
}

fn load_config(path: &Path) -> AppConfig {
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return cfg;
        }
    }
    AppConfig::default()
}

fn save_config(path: &Path, cfg: &AppConfig) {
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, s);
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.wiwi.linuxstudioeffects")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let config_path = PathBuf::from(&home).join(".config/linux-studio-effects/state.json");
    
    // Ensure dir exists
    if let Some(p) = config_path.parent() {
        fs::create_dir_all(p).ok();
    }

    let config = Rc::new(RefCell::new(load_config(&config_path)));
    
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Studio Effects Control")
        .default_width(400)
        .default_height(500)
        .build();

    let vbox = Box::new(Orientation::Vertical, 10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    
    // Header
    let header = Label::new(Some("<b>Webcam Control Panel</b>"));
    header.set_use_markup(true);
    vbox.append(&header);
    
    // 1. Active Switch
    let active_switch = gtk::Switch::new();
    active_switch.set_active(config.borrow().active);
    let active_box = Box::new(Orientation::Horizontal, 10);
    active_box.append(&Label::new(Some("Master Switch:")));
    active_box.append(&active_switch);
    vbox.append(&active_box);
    
    let cfg_clone = config.clone();
    let path_clone = config_path.clone();
    active_switch.connect_state_set(move |_, state| {
        cfg_clone.borrow_mut().active = state;
        save_config(&path_clone, &cfg_clone.borrow());
        gtk::Inhibit(false)
    });
    
    // 2. Blur Strength
    let blur_label = Label::new(Some("Blur Strength"));
    vbox.append(&blur_label);
    
    let adjustment = Adjustment::new(config.borrow().blur_strength as f64, 0.0, 1.0, 0.01, 0.1, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adjustment));
    scale.set_digits(2);
    scale.set_hexpand(true);
    vbox.append(&scale);
    
    let cfg_clone = config.clone();
    let path_clone = config_path.clone();
    scale.connect_value_changed(move |s| {
        cfg_clone.borrow_mut().blur_strength = s.value() as f32;
        save_config(&path_clone, &cfg_clone.borrow());
    });
    
    // 3. Sabotage
    let sab_label = Label::new(Some("Sabotage Mode"));
    vbox.append(&sab_label);
    
    let sab_combo = gtk::ComboBoxText::new();
    sab_combo.append(Some("none"), "None");
    sab_combo.append(Some("freeze"), "Freeze");
    sab_combo.append(Some("glitch"), "Glitch");
    
    let current_sab = config.borrow().sabotage.clone();
    sab_combo.set_active_id(Some(&current_sab));
    vbox.append(&sab_combo);
    
    let cfg_clone = config.clone();
    let path_clone = config_path.clone();
    sab_combo.connect_changed(move |c| {
        if let Some(id) = c.active_id() {
            cfg_clone.borrow_mut().sabotage = id.to_string();
            save_config(&path_clone, &cfg_clone.borrow());
        }
    });

    // 4. Priority List (Simple Text Entry for now, comma separated?)
    // Or just a label showing current list
    let prio_label = Label::new(Some("Camera Priority (Top = First)"));
    vbox.append(&prio_label);
    
    let prio_entry = Entry::new();
    prio_entry.set_text(&config.borrow().camera_priority.join(", "));
    vbox.append(&prio_entry);
    
    let save_prio_btn = Button::with_label("Update Priority");
    vbox.append(&save_prio_btn);
    
    let cfg_clone = config.clone();
    let path_clone = config_path.clone();
    save_prio_btn.connect_clicked(move |_| {
        let text = prio_entry.text();
        let list: Vec<String> = text.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        cfg_clone.borrow_mut().camera_priority = list;
        save_config(&path_clone, &cfg_clone.borrow());
    });
    
    // 5. Preview Placeholder
    // Implementing a real GStreamer GTK Paintable sink requires `gstreamer-video` crate with `v1_20` features and `gtk4` crate integration.
    // For this prototype, we'll add a placeholder button to launch a separate preview window if needed, or simply state it's running.
    // "Rendu en temps réel" -> User likely wants to see it inside the app.
    // Currently, without complex build.rs setup for GStreamer plugins, embedding video is hard in a single file submission.
    // I will add a note or button to launch `gst-launch` for preview.
    
    let preview_btn = Button::with_label("Launch External Preview");
    vbox.append(&preview_btn);
    
    preview_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("gst-launch-1.0")
            .args(&["v4l2src", "device=/dev/video1", "!", "videoconvert", "!", "autovideosink"])
            .spawn();
    });

    window.set_child(Some(&vbox));
    window.present();
}
