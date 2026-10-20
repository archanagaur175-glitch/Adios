package com.nullbrain.adios

import kotlin.math.abs
import kotlin.math.min
import kotlin.math.roundToLong

// P19.2 (D33): pure Kotlin port of the desktop score/args pipeline
// (src-tauri/src/resolver/{score.rs,ytdlp_args.rs}). Constants, weights,
// reject order and boost rules mirror byte-for-byte so desktop and
// Android accept the same candidates at the same confidence.

const val DURATION_HARD_BOUND_MS = 3000L
const val MIN_TITLE_OVERLAP = 0.34f
const val FORMAT_SELECTOR = "bestaudio[ext=m4a]/bestaudio"
const val SEARCH_COUNT = 3

private const val TITLE_WEIGHT = 0.65f
private const val ARTIST_WEIGHT = 0.35f
private const val OFFICIAL_AUDIO_BOOST = 0.05f
private const val TOPIC_BOOST = 0.08f

data class ScoreOutcome(val accepted: Boolean, val confidence: Float = 0f, val reason: String = "")

/** P9.7: drop parenthetical/bracket segments — "Song (Live)" searched
 *  literally finds nothing; "Song" finds the track. Mirrors ytdlp_args.rs. */
fun stripParentheticals(s: String): String {
    val out = StringBuilder()
    var depth = 0
    for (ch in s) {
        when (ch) {
            '(', '[' -> depth++
            ')', ']' -> depth = (depth - 1).coerceAtLeast(0)
            else -> if (depth == 0) out.append(ch)
        }
    }
    return out.toString().split(Regex("\\s+")).filter { it.isNotEmpty() }.joinToString(" ")
}

/** The term yt-dlp echoes back in batch doc ids: "<artist> - <title>". */
fun searchTerm(artist: String, title: String): String =
    "${stripParentheticals(artist)} - ${stripParentheticals(title)}"

/** Lowercase, strip punctuation to spaces, split into tokens. */
fun normalizeTokens(text: String): List<String> =
    text.lowercase()
        .map { if (it.isLetterOrDigit()) it else ' ' }
        .joinToString("")
        .split(Regex("\\s+"))
        .filter { it.isNotEmpty() }

/** Overlap coefficient over token sets; neutral 0.5 when either side is empty. */
private fun tokenOverlap(a: List<String>, b: List<String>): Float {
    if (a.isEmpty() || b.isEmpty()) return 0.5f
    val setA = a.toSet()
    val setB = b.toSet()
    val intersection = setA.intersect(setB).size
    return intersection.toFloat() / min(setA.size, setB.size)
}

/** Banned flavor words: rejected only when the QUERY lacks the word. */
private fun flavorReject(candTokens: List<String>, queryTokens: List<String>): String? {
    val banned = mapOf(
        "live" to "live version rejected",
        "cover" to "cover version rejected",
        "covers" to "cover version rejected",
        "remix" to "remix rejected",
        "remixed" to "remix rejected",
    )
    for ((word, reason) in banned) {
        if (word in candTokens && word !in queryTokens) return reason
    }
    return null
}

/** Score one candidate — reject order: flavor, duration bound, title overlap. */
fun score(
    queryArtist: String,
    queryTitle: String,
    queryDurationMs: Long?,
    candTitle: String,
    candArtist: String,
    candDurationMs: Long?,
): ScoreOutcome {
    val qTitle = normalizeTokens(queryTitle)
    val qArtist = normalizeTokens(queryArtist)
    val cTitle = normalizeTokens(candTitle)
    val cArtist = normalizeTokens(candArtist)

    flavorReject(cTitle, qTitle)?.let {
        return ScoreOutcome(accepted = false, reason = it)
    }

    if (queryDurationMs != null && candDurationMs != null) {
        val delta = abs(candDurationMs - queryDurationMs)
        if (delta > DURATION_HARD_BOUND_MS) {
            return ScoreOutcome(accepted = false, reason = "duration delta ${delta}ms exceeds bound")
        }
    }

    val titleOverlap = tokenOverlap(qTitle, cTitle)
    if (titleOverlap < MIN_TITLE_OVERLAP) {
        return ScoreOutcome(accepted = false, reason = "title tokens do not overlap")
    }

    val artistOverlap = tokenOverlap(qArtist, cArtist)
    var confidence = TITLE_WEIGHT * titleOverlap + ARTIST_WEIGHT * artistOverlap
    val raw = "$candTitle $candArtist".lowercase()
    if (raw.contains("official audio")) confidence += OFFICIAL_AUDIO_BOOST
    if (cArtist.lastOrNull() == "topic") confidence += TOPIC_BOOST
    return ScoreOutcome(accepted = true, confidence = confidence.coerceIn(0f, 1f))
}

/** yt-dlp seconds → ms like Rust `(secs * 1000.0).round() as i64`. */
fun durationMsOf(entrySeconds: Double?): Long? = entrySeconds?.let { (it * 1000).roundToLong() }

fun mimeForAudioExt(ext: String): String? = when (ext) {
    "m4a", "mp4" -> "audio/mp4"
    "webm" -> "audio/webm"
    "mp3" -> "audio/mpeg"
    "" -> null
    else -> ext
}
