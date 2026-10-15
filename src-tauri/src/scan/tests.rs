use super::*;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adios-scan-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Minimal valid PCM16 mono WAV that lofty can parse.
fn write_wav(path: &Path, sample_rate: u32, samples: &[i16]) {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend(&16u32.to_le_bytes());
    bytes.extend(&1u16.to_le_bytes()); // PCM
    bytes.extend(&1u16.to_le_bytes()); // mono
    bytes.extend(&sample_rate.to_le_bytes());
    bytes.extend(&(sample_rate * 2).to_le_bytes()); // byte rate
    bytes.extend(&2u16.to_le_bytes()); // block align
    bytes.extend(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend(&sample.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn collect_audio_files_recurses_filters_and_skips_hidden() {
    let dir = temp_dir("collect");
    std::fs::write(dir.join("a.mp3"), b"x").unwrap();
    std::fs::write(dir.join("b.TXT"), b"x").unwrap();
    std::fs::write(dir.join("c.MP3"), b"x").unwrap(); // case-insensitive
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/d.flac"), b"x").unwrap();
    std::fs::create_dir_all(dir.join(".hidden")).unwrap();
    std::fs::write(dir.join(".hidden/secret.mp3"), b"x").unwrap();

    let found = collect_audio_files(&dir);
    let names: Vec<String> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a.mp3", "c.MP3", "d.flac"]);
}

#[test]
fn read_meta_parses_wav_with_fallbacks() {
    let dir = temp_dir("meta");
    let path = dir.join("My Song.wav");
    let samples: Vec<i16> = (0..4000).map(|i| (i % 100) as i16).collect();
    write_wav(&path, 8000, &samples);

    let meta = tags::read_meta(&path).expect("lofty should parse PCM16 wav");
    assert!(meta.duration_ms >= 450 && meta.duration_ms <= 550, "{}", meta.duration_ms);
    assert_eq!(meta.title, "My Song"); // file-stem fallback
    assert_eq!(meta.artist, "Unknown Artist");
    assert_eq!(meta.album, None);
    assert_eq!(meta.file_size, std::fs::metadata(&path).unwrap().len() as i64);
    assert!(meta.cover.is_none());
}

#[test]
fn read_meta_missing_file_is_err() {
    let err = tags::read_meta(Path::new("C:\\definitely\\missing\\x.mp3"));
    assert!(err.is_err());
}

#[test]
fn import_files_counts_added_then_skips_duplicates() {
    let dir = temp_dir("import");
    let wav = dir.join("track one.wav");
    write_wav(&wav, 8000, &[0i16; 800]);
    let covers = dir.join("covers");
    std::fs::create_dir_all(&covers).unwrap();

    let conn = crate::db::open_memory().unwrap();
    let first = import_files(&conn, "fixture", &[wav.clone()], &covers);
    assert_eq!((first.added, first.skipped, first.failed), (1, 0, 0));
    assert_eq!(first.errors.len(), 0);
    assert!(first.elapsed_ms < 5000);

    let second = import_files(&conn, "fixture", &[wav.clone()], &covers);
    assert_eq!((second.added, second.skipped, second.failed), (0, 1, 0));

    let all = queries::tracks_all(&conn).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].title, "track one");
    assert_eq!(all[0].source, "local");

    // unreadable file lands in errors, not a panic
    let third = import_files(&conn, "fixture", &[dir.join("ghost.mp3")], &covers);
    assert_eq!(third.failed, 1);
    assert_eq!(third.errors.len(), 1);
    assert!(third.errors[0].contains("ghost.mp3"));
}

#[test]
fn cover_name_is_deterministic_and_sanitized() {
    let a = cover_name(Path::new("C:\\Music\\My Song (live).mp3"));
    assert_eq!(a, cover_name(Path::new("C:\\Music\\My Song (live).mp3")));
    assert!(a.starts_with("My_Song__live_-"));
    assert_ne!(a, cover_name(Path::new("C:\\Music\\Other.mp3")));
}
