//! 3-color palette extraction (P5.2): deterministic k-means over a 32×32
//! downscale (docs/ARCHITECTURE.md §"Cover Art Archive": "palette = 3
//! dominant colors from a 32×32 downscale, cached on the track row").
//! Consumes encoded bytes — embedded scan art or CAA downloads — and lands
//! in `tracks.accent` / `accent2` / `dominant_colors_json`.

use std::collections::HashMap;

/// Three dominant colors, most common first; `colors[0]` is the accent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub accent: String,
    pub accent2: String,
    pub colors: [String; 3],
}

/// Extract three dominant colors from encoded image bytes.
pub fn extract(bytes: &[u8]) -> Result<Palette, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("palette: decode failed: {e}"))?
        .to_rgb8();
    let small = image::imageops::thumbnail(&img, 32, 32);
    let points: Vec<[f32; 3]> = small
        .pixels()
        .map(|p| {
            let [r, g, b] = p.0;
            [r as f32, g as f32, b as f32]
        })
        .collect();
    if points.is_empty() {
        return Err("palette: no pixels".to_owned());
    }
    let mut clusters = kmeans(&points, 3);
    clusters.sort_by(|a, b| b.count.cmp(&a.count));
    while clusters.len() < 3 {
        clusters.push(Cluster {
            center: [0.0; 3],
            count: 0,
        });
    }
    let colors = [hex(clusters[0].center), hex(clusters[1].center), hex(clusters[2].center)];
    Ok(Palette {
        accent: colors[0].clone(),
        accent2: colors[1].clone(),
        colors,
    })
}

struct Cluster {
    center: [f32; 3],
    count: usize,
}

/// Lloyd iterations seeded from the most common 4-bits-per-channel buckets;
/// always yields `k` clusters (duplicated seeds pad out flat images).
fn kmeans(points: &[[f32; 3]], k: usize) -> Vec<Cluster> {
    let mut centers = seeds(points, k);
    for _ in 0..16 {
        let (sums, counts) = assign(points, &centers);
        let mut moved = false;
        for i in 0..k {
            if counts[i] == 0 {
                continue;
            }
            let next = [
                sums[i][0] / counts[i] as f32,
                sums[i][1] / counts[i] as f32,
                sums[i][2] / counts[i] as f32,
            ];
            if dist2(&next, &centers[i]) > 0.05 {
                moved = true;
            }
            centers[i] = next;
        }
        if !moved {
            break;
        }
    }
    let (_sums, counts) = assign(points, &centers);
    counts
        .iter()
        .enumerate()
        .map(|(i, &count)| Cluster {
            center: centers[i],
            count,
        })
        .collect()
}

/// Per-center sums + member counts for one assignment pass.
fn assign(points: &[[f32; 3]], centers: &[[f32; 3]]) -> (Vec<[f32; 3]>, Vec<usize>) {
    let mut sums = vec![[0.0f32; 3]; centers.len()];
    let mut counts = vec![0usize; centers.len()];
    for p in points {
        let mut best = 0;
        let mut best_d = f32::MAX;
        for (i, c) in centers.iter().enumerate() {
            let d = dist2(p, c);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        counts[best] += 1;
        for ch in 0..3 {
            sums[best][ch] += p[ch];
        }
    }
    (sums, counts)
}

fn dist2(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

/// Top-`k` bucket averages (bucket = 16 levels per channel).
fn seeds(points: &[[f32; 3]], k: usize) -> Vec<[f32; 3]> {
    let mut buckets: HashMap<[u16; 3], (usize, [f64; 3])> = HashMap::new();
    for p in points {
        let key = [
            (p[0] as u32 >> 4) as u16,
            (p[1] as u32 >> 4) as u16,
            (p[2] as u32 >> 4) as u16,
        ];
        let entry = buckets.entry(key).or_insert((0, [0.0; 3]));
        entry.0 += 1;
        for ch in 0..3 {
            entry.1[ch] += p[ch] as f64;
        }
    }
    let mut common: Vec<_> = buckets.into_values().collect();
    common.sort_by(|a, b| b.0.cmp(&a.0));
    let mut centers: Vec<[f32; 3]> = common
        .iter()
        .take(k)
        .map(|(n, sum)| {
            let n = *n as f64;
            [(sum[0] / n) as f32, (sum[1] / n) as f32, (sum[2] / n) as f32]
        })
        .collect();
    let pad = centers.last().copied().unwrap_or([128.0; 3]);
    centers.resize(k, pad);
    centers
}

fn hex(c: [f32; 3]) -> String {
    let ch = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", ch(c[0]), ch(c[1]), ch(c[2]))
}
