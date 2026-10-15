use tauri::window::{Effect, EffectsBuilder};
use tauri::WebviewWindow;

/// Apply a Windows system backdrop to the window.
/// Tries Mica (dark) first, falls back to Mica, then Acrylic.
/// Returns the label of the applied effect (or "none") for logging.
/// API source: tauri-2.12.4 `src/window/mod.rs` (EffectsBuilder doc example)
/// and tauri-utils `WindowEffect` enum variants.
pub fn apply(window: &WebviewWindow) -> &'static str {
    let attempts: [(&'static str, Effect); 3] = [
        ("mica-dark", Effect::MicaDark),
        ("mica", Effect::Mica),
        ("acrylic", Effect::Acrylic),
    ];
    for (label, effect) in attempts {
        match window.set_effects(EffectsBuilder::new().effect(effect).build()) {
            Ok(()) => return label,
            Err(err) => eprintln!("[vibrancy] {label} failed: {err}"),
        }
    }
    "none"
}
