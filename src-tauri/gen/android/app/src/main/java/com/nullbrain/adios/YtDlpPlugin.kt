package com.nullbrain.adios

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.youtubedl_android.YoutubeDL
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// P19/D33: in-process yt-dlp (python + lazy extractors bundled by the
// youtubedl-android library). Commands run off the main thread; responses
// map 1:1 onto the Rust Candidate shape in commands/mobile.rs.
@TauriPlugin
class YtDlpPlugin(private val activity: Activity) : Plugin(activity) {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    @Volatile
    private var initialized = false

    /** 19.1 bridge proof: lazy-init the library once, report version. */
    @Command
    fun ping(invoke: Invoke) {
        scope.launch {
            val ret = JSObject()
            try {
                if (!initialized) {
                    YoutubeDL.getInstance().init(activity.applicationContext)
                    initialized = true
                }
                ret.put("ok", true)
                ret.put("version", YoutubeDL.getInstance().versionName(activity))
            } catch (e: Exception) {
                ret.put("ok", false)
                ret.put("error", e.message ?: e.javaClass.simpleName)
            }
            invoke.resolve(ret)
        }
    }
}
