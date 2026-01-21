const St = imports.gi.St;
const Main = imports.ui.main;
const PanelMenu = imports.ui.panelMenu;
const PopupMenu = imports.ui.popupMenu;
const GObject = imports.gi.GObject;
const Gio = imports.gi.Gio;
const GLib = imports.gi.GLib;

const CONFIG_PATH = GLib.get_home_dir() + "/.config/linux-studio-effects/state.json";

const StudioIndicator = GObject.registerClass(
    class StudioIndicator extends PanelMenu.Button {
        _init() {
            super._init(0.0, "Studio Effects");

            // Icon
            let icon = new St.Icon({
                icon_name: 'camera-video-symbolic',
                style_class: 'system-status-icon'
            });
            this.add_child(icon);

            // Load Config
            this._config = this._loadConfig();

            // 1. Master Switch
            this._switchItem = new PopupMenu.PopupSwitchMenuItem("Active", this._config.active);
            this._switchItem.connect('toggled', (item, state) => {
                this._config.active = state;
                this._saveConfig();
            });
            this.menu.addMenuItem(this._switchItem);

            this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

            // 2. Blur Strength (Slider not standard in simple API, using Submenu or Steps?)
            // Standard extensions use SliderMenuItem (custom).
            // For simplicity: "Blur Strength" submenu with 25%, 50%, 75%, 100%
            let blurItem = new PopupMenu.PopupSubMenuMenuItem("Blur Strength");
            [0.0, 0.25, 0.5, 0.75, 1.0].forEach(val => {
                let label = (val * 100) + "%";
                let item = new PopupMenu.PopupMenuItem(label);
                item.connect('activate', () => {
                    this._config.blur_strength = val;
                    this._saveConfig();
                });
                blurItem.menu.addMenuItem(item);
            });
            this.menu.addMenuItem(blurItem);

            // 3. Effects (Presets)
            let modeItem = new PopupMenu.PopupSubMenuMenuItem("Effects");

            let effectsPresets = [
                { label: "Blur Background", val: ["blur"] },
                { label: "Replace Background", val: ["replace"] },
                { label: "Replace & Blur", val: ["replace", "blur"] }
            ];

            effectsPresets.forEach(preset => {
                let item = new PopupMenu.PopupMenuItem(preset.label);
                item.connect('activate', () => {
                    this._config.effects = preset.val;
                    this._saveConfig();
                });
                modeItem.menu.addMenuItem(item);
            });
            this.menu.addMenuItem(modeItem);

            // 4. GPU Backend
            let gpuItem = new PopupMenu.PopupSubMenuMenuItem("GPU Backend");
            ["auto", "nvidia", "intel", "cpu"].forEach(backend => {
                let item = new PopupMenu.PopupMenuItem(backend);
                item.connect('activate', () => {
                    this._config.gpu_backend = backend;
                    this._saveConfig();
                });
                gpuItem.menu.addMenuItem(item);
            });
            this.menu.addMenuItem(gpuItem);

            // 5. Sabotage
            let sabItem = new PopupMenu.PopupSubMenuMenuItem("Sabotage");
            ["none", "freeze", "glitch"].forEach(m => {
                let item = new PopupMenu.PopupMenuItem(m);
                item.connect('activate', () => {
                    this._config.sabotage = m;
                    this._saveConfig();
                });
                sabItem.menu.addMenuItem(item);
            });
            this.menu.addMenuItem(sabItem);

            this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

            // 6. Status Indicator (Detailed)
            this._statusItem = new PopupMenu.PopupMenuItem("Status: Checking...");
            this.menu.addMenuItem(this._statusItem);

            // Poll status every 1 second (faster updates for metrics)
            GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
                this._updateStatus();
                return true; // repeat
            });
        }

        _updateStatus() {
            try {
                let file = Gio.File.new_for_path(GLib.get_home_dir() + "/.config/linux-studio-effects/state_report.json");
                let [success, content] = file.load_contents(null);
                if (success) {
                    let status = JSON.parse(new TextDecoder().decode(content));
                    let ms = (status.latency_us / 1000.0).toFixed(2);
                    this._statusItem.label.text = `Tech: ${status.tech}\nLatency: ${ms}ms`;
                }
            } catch (e) {
                // ignore
            }
        }

        _loadConfig() {
            try {
                let file = Gio.File.new_for_path(CONFIG_PATH);
                let [success, content] = file.load_contents(null);
                if (success) {
                    return JSON.parse(new TextDecoder().decode(content));
                }
            } catch (e) {
                log("StudioEffects: Failed to load config " + e);
            }
            return { active: true, blur_strength: 0.5, sabotage: "none", effects: ["blur"], gpu_backend: "auto" };
        }

        _saveConfig() {
            try {
                let file = Gio.File.new_for_path(CONFIG_PATH);
                let json = JSON.stringify(this._config, null, 2);
                file.replace_contents(json, null, false, Gio.FileCreateFlags.NONE, null);
            } catch (e) {
                log("StudioEffects: Failed to save config " + e);
            }
        }
    });

let _indicator;

function init() {
}

function enable() {
    _indicator = new StudioIndicator();
    Main.panel.addToStatusArea('studio-effects', _indicator);
}

function disable() {
    _indicator.destroy();
    _indicator = null;
}
