package com.nullbrain.adios

import app.tauri.plugin.JSObject
import com.yausername.youtubedl_android.YoutubeDLRequest
import org.json.JSONArray
import org.json.JSONObject

internal data class BatchQuery(val artist: String, val title: String, val durationMs: Long?)

/** P19.4: one request per chunk — mirror of build_batch_args (ytdlp_batch.rs).
 *  YoutubeDLRequest(urls: List) emits shared options once before the urls;
 *  yt-dlp options are global so per-input repetition (desktop's flattened
 *  argv) is semantically identical. `--ignore-errors` is ANDROID-ONLY: the
 *  library throws and discards partial stdout on non-zero exit (YoutubeDL.kt
 *  executeImpl), this keeps one failed search from aborting the chunk. */
internal fun buildBatchRequest(queries: List<BatchQuery>): YoutubeDLRequest {
    val terms = queries.map { "ytsearch$SEARCH_COUNT:${searchTerm(it.artist, it.title)}" }
    return YoutubeDLRequest(terms)
        .addOption("--no-playlist")
        .addOption("--no-warnings")
        .addOption("--skip-download")
        .addOption("--ignore-errors")
        .addOption("--socket-timeout", "15")
        .addOption("--dump-single-json")
        .addOption("-f", FORMAT_SELECTOR)
}

/** Newline-separated compact JSON docs; invalid lines skipped (the desktop
 *  serde stream stops at first error — line-skip is strictly more lenient
 *  and safer against stray non-JSON output on device). */
private fun parseDocs(out: String): List<JSONObject> =
    out.lineSequence().mapNotNull { line ->
        val t = line.trim()
        if (t.isEmpty()) null else try { JSONObject(t) } catch (_: Exception) { null }
    }.toList()

/** Per-query items array — port of parse_batch (ytdlp_batch.rs): match docs
 *  to queries by the echoed search term (LIFO pop for duplicate terms),
 *  positional fallback when unmatched doc/query counts match, then score the
 *  matched doc's entries with the shared pickFromDoc (first accept wins). */
internal fun parseBatchOut(out: String, queries: List<BatchQuery>): JSONArray {
    val docs = parseDocs(out)
    val byKey = HashMap<String, ArrayDeque<Int>>()
    docs.forEachIndexed { di, doc ->
        val id = doc.optString("id", "")
        if (id.isNotEmpty()) byKey.getOrPut(id) { ArrayDeque() }.addLast(di)
    }
    val docFor = arrayOfNulls<Int>(queries.size)
    val leftovers = mutableListOf<Int>()
    queries.forEachIndexed { qi, q ->
        val dq = byKey[searchTerm(q.artist, q.title)]
        if (dq != null && dq.isNotEmpty()) docFor[qi] = dq.removeLast() else leftovers.add(qi)
    }
    val used = docFor.filterNotNull().toHashSet()
    val unmatchedDocs = docs.indices.filter { it !in used }
    if (unmatchedDocs.size == leftovers.size) {
        unmatchedDocs.forEachIndexed { i, di -> docFor[leftovers[i]] = di }
        leftovers.clear()
    }

    val items = JSONArray()
    queries.forEachIndexed { qi, q ->
        val item = JSObject()
        val di = docFor[qi]
        if (di == null) {
            item.put("ok", false)
            item.put("error", "batch: no result doc for \"${searchTerm(q.artist, q.title)}\"")
        } else {
            try {
                val hit = pickFromDoc(docs[di], q.artist, q.title, q.durationMs)
                item.put("ok", true)
                item.put("url", hit.url)
                if (hit.mime != null) item.put("mime", hit.mime)
                item.put("confidence", hit.confidence.toDouble())
                item.put("source", "ytdlp")
            } catch (e: Exception) {
                item.put("ok", false)
                item.put("error", e.message ?: "batch miss")
            }
        }
        items.put(item)
    }
    return items
}

/** Catch-path items: index-aligned misses carrying the plugin error. */
internal fun missItems(queries: List<BatchQuery>, error: String): JSONArray {
    val items = JSONArray()
    for (q in queries) {
        val item = JSObject()
        item.put("ok", false)
        item.put("error", error)
        items.put(item)
    }
    return items
}
