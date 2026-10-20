package com.nullbrain.adios

import android.app.Activity
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLException
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
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

    /** 19.1 bridge proof: lazy-init the library once, report version. */
    @Command
    fun ping(invoke: Invoke) {
        scope.launch {
            val ret = JSObject()
            try {
                ensureInit()
                ret.put("ok", true)
                ret.put("version", YoutubeDL.getInstance().versionName(activity))
            } catch (e: Exception) {
                ret.put("ok", false)
                ret.put("error", e.message ?: e.javaClass.simpleName)
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
                val hit = pickCandidate(response.out, artist, title, durationMs)
                val ret = JSObject()
                ret.put("url", hit.url)
                hit.mime?.let { ret.put("mime", it) }
                ret.put("confidence", hit.confidence.toDouble())
                ret.put("source", "ytdlp")
                Log.i(TAG, "resolve hit conf=${hit.confidence} \"${hit.title}\"")
                invoke.resolve(ret)
            } catch (e: Exception) {
                Log.w(TAG, "resolve failed: ${e.message}")
                invoke.reject(e.message ?: "resolve failed")
            }
        }
    }

    private data class Hit(val url: String, val mime: String?, val title: String, val confidence: Float)

    /** Mirrors pick_candidate (ytdlp.rs): relevance order, first accept wins. */
    private fun pickCandidate(
        outJson: String,
        queryArtist: String,
        queryTitle: String,
        queryDurationMs: Long?,
    ): Hit {
        val root = JSONObject(outJson)
        val entriesArr = root.optJSONArray("entries")
        val entries = if (entriesArr != null) {
            (0 until entriesArr.length()).mapNotNull { entriesArr.optJSONObject(it) }
        } else {
            listOf(root)
        }
        if (entries.isEmpty()) throw YoutubeDLException("yt-dlp returned no results")
        var firstReject: String? = null
        for (entry in entries) {
            val entryTitle = entry.optString("title", "")
            val entryArtist = entry.optString("channel", "").ifEmpty { entry.optString("uploader", "") }
            val seconds = if (entry.isNull("duration")) null else entry.optDouble("duration").takeIf { !it.isNaN() }
            val outcome = score(queryArtist, queryTitle, queryDurationMs, entryTitle, entryArtist, durationMsOf(seconds))
            if (!outcome.accepted) {
                if (firstReject == null) firstReject = "${outcome.reason} (\"$entryTitle\")"
                Log.i(TAG, "reject: ${outcome.reason} (\"$entryTitle\")")
                continue
            }
            val url = entry.optString("url", "").ifEmpty { entry.optString("webpage_url", "") }
            if (url.isEmpty()) throw YoutubeDLException("accepted result has no url")
            return Hit(url, mimeForAudioExt(entry.optString("ext", "")), entryTitle, outcome.confidence)
        }
        throw YoutubeDLException("no usable YouTube video (${firstReject ?: "unrated results"})")
    }
}
