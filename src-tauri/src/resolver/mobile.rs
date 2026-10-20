//! P19/D33: Android in-process yt-dlp bridge (youtubedl-android 0.18.1).
//! Registers the Kotlin `YtDlpPlugin` on Android; desktop compiles a
//! no-op plugin + placeholder state so shared command signatures typecheck.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Wry};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

/// Managed handle to the Kotlin plugin (Android). Desktop keeps a
/// placeholder so `commands::mobile` compiles on every target.
#[cfg(target_os = "android")]
pub struct YtDlpMobile(pub PluginHandle<Wry>);

#[cfg(not(target_os = "android"))]
pub struct YtDlpMobile;

/// Plugin registration — wired into the builder in lib.rs.
pub fn plugin() -> TauriPlugin<Wry> {
    Builder::<Wry, ()>::new("ytdlp")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api
                    .register_android_plugin("com.nullbrain.adios", "YtDlpPlugin")
                    .expect("register YtDlpPlugin");
                println!("[ytdlp] android plugin registered");
                app.manage(YtDlpMobile(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = api;
                app.manage(YtDlpMobile);
            }
            Ok(())
        })
        .build()
}
