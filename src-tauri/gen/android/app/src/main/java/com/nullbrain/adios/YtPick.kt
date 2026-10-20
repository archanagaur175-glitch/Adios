package com.nullbrain.adios

import android.util.Log
import com.yausername.youtubedl_android.YoutubeDLException
import org.json.JSONObject

private const val TAG = "YtPick"

internal data class Hit(val url: String, val mime: String?, val title: String, val confidence: Float)

/** Mirrors pick_candidate (ytdlp.rs): relevance order, first accept wins.
 *  Shared by the single `resolve` command (19.2) and the batch parse (19.4)
 *  so both paths accept the same candidates at the same confidence. */
internal fun pickFromDoc(
    root: JSONObject,
    queryArtist: String,
    queryTitle: String,
    queryDurationMs: Long?,
): Hit {
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
