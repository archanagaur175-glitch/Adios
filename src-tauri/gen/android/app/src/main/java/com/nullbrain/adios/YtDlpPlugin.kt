package com.nullbrain.adios

import android.app.Activity
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

private const val TAG = "YtDlpPlugin"

// P19/D33: in-process yt-dlp (python + lazy extractors bundled by the
// youtubedl-android library). Commands run off the main thread; responses
// map 1:1 onto the Rust Candidate shape in commands/mobile.rs.
@TauriPlugin
class YtDlpPlugin(private val activity: Activity) : Plugin(activity) {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    @Volatile
    private var initialized = false

    @Volatile
    private var updateAttempted = false

    private val prefs by lazy {
        activity.getSharedPreferences("adios_ytdlp", Activity.MODE_PRIVATE)
    }

    private fun ensureInit() {
        if (initialized) return
        YoutubeDL.getInstance().init(activity.applicationContext)
        initialized = true
    }

    /** Risk ①: one-time yt-dlp self-update — persisted only on success,
     *  at most one attempt per launch (failed launches retry next time). */
    private fun ensureUpdated() {
        if (prefs.getBoolean("updated", false) || updateAttempted) return
        updateAttempted = true
        try {
            YoutubeDL.getInstance().updateYoutubeDL(activity, YoutubeDL.UpdateChannel.STABLE)
            prefs.edit().putBoolean("updated", true).apply()
            Log.i(TAG, "yt-dlp updated -> ${YoutubeDL.getInstance().versionName(activity)}")
        } catch (e: Exception) {
            Log.w(TAG, "yt-dlp update failed (retried next launch): ${e.message}")
        }
    }

    /** 19.1 bridge proof: lazy-init the library once, report version.
     *  catch Throwable: reflective bootstrap failures are Errors
     *  (ExceptionInInitializerError/NoClassDefFoundError), not Exceptions —
     *  they must reject the invoke, never kill the process (gate195). */
    @Command
    fun ping(invoke: Invoke) {
        scope.launch {
            val ret = JSObject()
            try {
                ensureInit()
                ret.put("ok", true)
                ret.put("version", YoutubeDL.getInstance().versionName(activity))
            } catch (t: Throwable) {
                Log.e(TAG, "ping failed", t)
                ret.put("ok", false)
                ret.put("error", t.message ?: t.javaClass.simpleName)
            }
            invoke.resolve(ret)
        }
    }

    /** P19.2: desktop resolve pipeline in-process — ytsearch3 → parse
     *  entries → score() port → first accept wins → bestaudio URL. */
    @Command
    fun resolve(invoke: Invoke) {
        scope.launch {
            try {
                ensureInit()
                ensureUpdated()
                val args = invoke.getArgs()
                val artist = args.optString("artist", "")
                val title = args.optString("title", "")
                val durationMs = args.optLong("durationMs", -1L).takeIf { it > 0 }

                val request = YoutubeDLRequest("ytsearch$SEARCH_COUNT:${searchTerm(artist, title)}")
                    .addOption("--no-playlist")
                    .addOption("--no-warnings")
                    .addOption("--skip-download")
                    .addOption("--socket-timeout", "15")
                    .addOption("--dump-single-json")
                    .addOption("-f", FORMAT_SELECTOR)
                val response = YoutubeDL.getInstance().execute(request)
                val hit = pickFromDoc(JSONObject(response.out), artist, title, durationMs)
                val ret = JSObject()
                ret.put("url", hit.url)
                hit.mime?.let { ret.put("mime", it) }
                ret.put("confidence", hit.confidence.toDouble())
                ret.put("source", "ytdlp")
                Log.i(TAG, "resolve hit conf=${hit.confidence} \"${hit.title}\"")
                invoke.resolve(ret)
            } catch (t: Throwable) {
                Log.e(TAG, "resolve failed", t)
                invoke.reject(t.message ?: "resolve failed")
            }
        }
    }

    /** P19.4: ONE python process per chunk — desktop resolve_batch parity
     *  (multi-doc stdout → doc↔query mapping → shared score loop per query).
     *  `latched: true` = the plugin or process could not run; the FE then
     *  stops batching and every pick falls back to the sequential single
     *  query path (today's behavior, preserved). */
    @Command
    fun resolveBatch(invoke: Invoke) {
        val arr = invoke.getArgs().optJSONArray("queries") ?: JSONArray()
        val queries = (0 until arr.length()).map { i ->
            val o = arr.optJSONObject(i) ?: JSONObject()
            BatchQuery(
                o.optString("artist", ""),
                o.optString("title", ""),
                o.optLong("durationMs", -1L).takeIf { it > 0 },
            )
        }
        scope.launch {
            val ret = JSObject()
            if (queries.isEmpty()) {
                ret.put("latched", false)
                ret.put("items", JSONArray())
                invoke.resolve(ret)
                return@launch
            }
            try {
                ensureInit()
                ensureUpdated()
                val response = YoutubeDL.getInstance().execute(buildBatchRequest(queries))
                if (response.exitCode != 0) {
                    Log.w(TAG, "batch exited ${response.exitCode} with ${response.out.length} out bytes (kept)")
                }
                ret.put("latched", false)
                ret.put("items", parseBatchOut(response.out, queries))
                Log.i(TAG, "resolveBatch: ${queries.size} queries -> ${response.out.length} out bytes")
            } catch (t: Throwable) {
                Log.e(TAG, "resolveBatch failed", t)
                ret.put("latched", true)
                ret.put("items", missItems(queries, t.message ?: "batch failed"))
            }
            invoke.resolve(ret)
        }
    }
}
