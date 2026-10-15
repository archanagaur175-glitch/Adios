//! P5.2 palette tests: block fixture proves dominance order, solid fill
//! stays uniform, garbage input errors cleanly.

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

use crate::meta::palette::extract;

/// Red 40 % / green 35 % / blue 25 % horizontal blocks → PNG bytes.
fn block_png() -> Vec<u8> {
    let img = RgbImage::from_fn(80, 40, |x, _| {
        if x < 32 {
            Rgb([255, 0, 0])
        } else if x < 60 {
            Rgb([0, 255, 0])
        } else {
            Rgb([0, 0, 255])
        }
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, ImageFormat::Png)
        .expect("encode fixture png");
    buf.into_inner()
}

fn near(hex: &str, want: [u8; 3], tol: i32) -> bool {
    let h = hex.trim_start_matches('#');
    let parse = |s: &str| i32::from_str_radix(s, 16).unwrap_or(-1);
    let got = [parse(&h[0..2]), parse(&h[2..4]), parse(&h[4..6])];
    (0..3).all(|i| (got[i] - want[i] as i32).abs() <= tol)
}

#[test]
fn three_blocks_yield_dominance_ordered_colors() {
    let pal = extract(&block_png()).expect("palette");
    assert_eq!(pal.accent, pal.colors[0], "accent is the dominant color");
    assert_eq!(pal.accent2, pal.colors[1], "accent2 is the runner-up");
    assert!(near(&pal.colors[0], [255, 0, 0], 48), "accent: {}", pal.colors[0]);
    assert!(near(&pal.colors[1], [0, 255, 0], 48), "second: {}", pal.colors[1]);
    assert!(near(&pal.colors[2], [0, 0, 255], 48), "third: {}", pal.colors[2]);
}

#[test]
fn solid_fill_returns_uniform_palette() {
    let img = RgbImage::from_pixel(16, 16, Rgb([128, 128, 128]));
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, ImageFormat::Png)
        .expect("encode fixture png");
    let pal = extract(&buf.into_inner()).expect("palette");
    for color in &pal.colors {
        assert!(near(color, [128, 128, 128], 12), "uniform: {color}");
    }
    assert_eq!(pal.accent, pal.colors[0]);
}

#[test]
fn garbage_bytes_error_mentions_palette() {
    let err = extract(b"definitely not an image").unwrap_err();
    assert!(err.contains("palette"), "{err}");
}

#[test]
fn alpha_png_decodes_without_panicking() {
    let rgba = image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 20, 30, 128]));
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut buf, ImageFormat::Png)
        .expect("encode alpha png");
    let pal = extract(&buf.into_inner()).expect("alpha png palette");
    assert!(near(&pal.accent, [10, 20, 30], 6), "{}", pal.accent);
}
