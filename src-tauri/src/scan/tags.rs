//! Tag + embedded-art reading via lofty (P3.2 folder scan).
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::MimeType;
use lofty::tag::{Accessor, Tag};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Embedded cover art ready to be written to `covers/`.
pub struct CoverArt {
    pub ext: &'static str,
    pub data: Vec<u8>,
}

/// Metadata extracted from one audio file.
pub struct FileMeta {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub file_size: i64,
    pub mtime: Option<i64>,
    pub cover: Option<CoverArt>,
}

/// Read tags, properties, and embedded art from an audio file.
/// Fallbacks: title → file stem, artist → "Unknown Artist".
pub fn read_meta(path: &Path) -> Result<FileMeta, String> {
    let fs_meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let file_size = fs_meta.len() as i64;
    let mtime = fs_meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    let tagged = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let duration_ms = tagged.properties().duration().as_millis() as i64;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let stem = |p: &Path| {
        p.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".to_owned())
    };

    let (title, artist, album, year, track_no, disc_no) = match tag {
        Some(t) => (
            t.title().map(|v| v.into_owned()).unwrap_or_else(|| stem(path)),
            t.artist()
                .map(|v| v.into_owned())
                .unwrap_or_else(|| "Unknown Artist".to_owned()),
            t.album().map(|v| v.into_owned()),
            t.date().map(|d| i64::from(d.year)),
            t.track().map(i64::from),
            t.disk().map(i64::from),
        ),
        None => (stem(path), "Unknown Artist".to_owned(), None, None, None, None),
    };

    Ok(FileMeta {
        title,
        artist,
        album,
        duration_ms,
        year,
        track_no,
        disc_no,
        file_size,
        mtime,
        cover: tag.and_then(pick_cover),
    })
}

/// First Jpeg/Png picture (covers are the common case across formats).
fn pick_cover(tag: &Tag) -> Option<CoverArt> {
    let pic = tag
        .pictures()
        .iter()
        .find(|p| matches!(p.mime_type(), Some(MimeType::Jpeg | MimeType::Png)))?;
    let ext = match pic.mime_type() {
        Some(MimeType::Png) => "png",
        _ => "jpg",
    };
    Some(CoverArt {
        ext,
        data: pic.data().to_vec(),
    })
}
