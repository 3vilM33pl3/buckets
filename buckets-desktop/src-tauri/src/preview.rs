// SPDX-License-Identifier: MIT
//! File previews and diffs: textures as images (with a difference map), text
//! as a line diff, and `.blend` files through their embedded thumbnail.

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use base64::Engine;
use buckets::utils::object_store::{is_safe_relative, ObjectStore};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

use crate::model::{DiffLine, FileDiff, ImageDiff, Preview, Source};
use crate::repo::RepoBucket;

/// Longest edge of a preview. Diffs are measured at this size too.
const MAX_EDGE: u32 = 2048;
/// Files larger than this are not decoded.
const MAX_DECODE_BYTES: u64 = 768 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp", "webp", "exr", "hdr", "gif",
];
const TEXT_EXTS: &[&str] = &[
    "txt", "md", "json", "ini", "cfg", "toml", "yaml", "yml", "xml", "csv", "py", "lua", "usda",
    "obj", "mtl", "glsl", "hlsl", "usf", "ush", "cpp", "h", "hpp", "cs", "rs", "js", "ts", "html",
    "css", "uproject", "uplugin", "vex", "vfl", "sh", "bat", "gltf",
];

fn ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Bytes of one version of a file, or None if it does not exist.
fn read_source(
    root: &Path,
    bucket: &RepoBucket,
    path: &str,
    source: &Source,
) -> Result<Option<(Vec<u8>, String)>, String> {
    if !is_safe_relative(path) {
        return Err(format!("Refusing unsafe path '{}'", path));
    }
    match source {
        Source::Working => {
            let full = bucket.abs.join(path);
            if !full.is_file() {
                return Ok(None);
            }
            let size = full.metadata().map(|m| m.len()).unwrap_or(0);
            if size > MAX_DECODE_BYTES {
                return Err(format!("{} is too large to preview", path));
            }
            let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
            let hash = blake3::hash(&bytes).to_hex().to_string();
            Ok(Some((bytes, hash)))
        }
        Source::Object { hash } => {
            let store = ObjectStore::for_repo(root);
            let Some(object) = store.locate(hash, Some(&bucket.abs)) else {
                return Err(format!(
                    "Object {} is not in the local store. Pull from the remote first.",
                    &hash[..hash.len().min(12)]
                ));
            };
            let input = File::open(object).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            zstd::stream::read::Decoder::new(BufReader::new(input))
                .map_err(|e| e.to_string())?
                .take(MAX_DECODE_BYTES)
                .read_to_end(&mut out)
                .map_err(|e| e.to_string())?;
            Ok(Some((out, hash.clone())))
        }
    }
}

fn png_data_url(img: &RgbaImage) -> Result<String, String> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    ))
}

fn fit(img: &DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= MAX_EDGE {
        img.clone()
    } else {
        img.resize(MAX_EDGE, MAX_EDGE, image::imageops::FilterType::Triangle)
    }
}

struct Decoded {
    preview: Preview,
    pixels: Option<RgbaImage>,
}

fn decode_image(bytes: &[u8], path: &str) -> Result<(DynamicImage, String, Option<String>), String> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let format = reader
        .format()
        .map(|f| format!("{:?}", f).to_uppercase())
        .unwrap_or_else(|| ext(path).to_uppercase());
    let img = match reader.decode() {
        Ok(img) => img,
        Err(_) if ext(path) == "tga" => {
            image::load_from_memory_with_format(bytes, ImageFormat::Tga).map_err(|e| e.to_string())?
        }
        Err(e) => return Err(e.to_string()),
    };
    let note = match img.color() {
        image::ColorType::Rgb32F | image::ColorType::Rgba32F => {
            Some("HDR values clamped to 0–1 for display".to_string())
        }
        image::ColorType::L16 | image::ColorType::La16 | image::ColorType::Rgb16 | image::ColorType::Rgba16 => {
            Some("16-bit, shown at 8-bit".to_string())
        }
        _ => None,
    };
    Ok((img, format, note))
}

fn build(bytes: Option<(Vec<u8>, String)>, path: &str) -> Decoded {
    let Some((bytes, hash)) = bytes else {
        return Decoded {
            preview: Preview {
                kind: "missing".into(),
                ..Default::default()
            },
            pixels: None,
        };
    };
    let mut preview = Preview {
        kind: "binary".into(),
        size: Some(bytes.len() as u64),
        hash: Some(hash),
        ..Default::default()
    };
    let e = ext(path);

    if IMAGE_EXTS.contains(&e.as_str()) {
        match decode_image(&bytes, path) {
            Ok((img, format, note)) => {
                let (w, h) = img.dimensions();
                let shown = fit(&img).to_rgba8();
                preview.kind = "image".into();
                preview.width = Some(w);
                preview.height = Some(h);
                preview.format = Some(format);
                preview.note = note;
                preview.data_url = png_data_url(&shown).ok();
                return Decoded {
                    preview,
                    pixels: Some(shown),
                };
            }
            Err(err) => preview.note = Some(format!("Could not decode: {}", err)),
        }
    } else if e == "blend" {
        if let Some(thumb) = blend_thumbnail(&bytes) {
            preview.kind = "image".into();
            preview.width = Some(thumb.width());
            preview.height = Some(thumb.height());
            preview.format = Some("BLEND".into());
            preview.note = Some("Thumbnail saved by Blender".into());
            preview.data_url = png_data_url(&thumb).ok();
            return Decoded {
                preview,
                pixels: Some(thumb),
            };
        }
    }

    if bytes.len() <= MAX_TEXT_BYTES && (TEXT_EXTS.contains(&e.as_str()) || looks_like_text(&bytes)) {
        if let Ok(text) = String::from_utf8(bytes) {
            preview.kind = "text".into();
            preview.text = Some(text);
        }
    }
    Decoded {
        preview,
        pixels: None,
    }
}

fn looks_like_text(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(8192)];
    !head.is_empty() && !head.contains(&0) && std::str::from_utf8(head).is_ok()
}

pub fn preview(root: &Path, bucket: &RepoBucket, path: &str, source: &Source) -> Result<Preview, String> {
    Ok(build(read_source(root, bucket, path, source)?, path).preview)
}

pub fn diff(
    root: &Path,
    bucket: &RepoBucket,
    path: &str,
    left: &Source,
    right: &Source,
) -> Result<FileDiff, String> {
    let l = build(read_source(root, bucket, path, left)?, path);
    let r = build(read_source(root, bucket, path, right)?, path);
    let image = match (&l.pixels, &r.pixels) {
        (Some(a), Some(b)) => image_diff(
            a,
            b,
            l.preview.width == r.preview.width && l.preview.height == r.preview.height,
        )
        .ok(),
        _ => None,
    };
    let text = match (&l.preview.text, &r.preview.text) {
        (Some(a), Some(b)) => Some(text_diff(a, b)),
        (None, Some(b)) if l.preview.kind == "missing" => Some(text_diff("", b)),
        (Some(a), None) if r.preview.kind == "missing" => Some(text_diff(a, "")),
        _ => None,
    };
    Ok(FileDiff {
        left: l.preview,
        right: r.preview,
        image,
        text,
    })
}

/// Per-pixel difference, shown as a heat map: dark where equal, through
/// magenta to yellow-white where the change is largest.
fn image_diff(a: &RgbaImage, b: &RgbaImage, same_size: bool) -> Result<ImageDiff, String> {
    let b_resized;
    let b = if a.dimensions() == b.dimensions() {
        b
    } else {
        b_resized = image::imageops::resize(b, a.width(), a.height(), image::imageops::FilterType::Triangle);
        &b_resized
    };
    let mut out = RgbaImage::new(a.width(), a.height());
    let mut changed = 0u64;
    let mut total = 0u64;
    let mut max = 0u8;
    for ((pa, pb), po) in a.pixels().zip(b.pixels()).zip(out.pixels_mut()) {
        let d = (0..4)
            .map(|i| (pa[i] as i16 - pb[i] as i16).unsigned_abs() as u8)
            .max()
            .unwrap_or(0);
        total += d as u64;
        max = max.max(d);
        if d > 2 {
            changed += 1;
        }
        // Boost small differences so they are visible.
        let t = ((d as f32 / 255.0).sqrt()).min(1.0);
        let lum = (pa[0] as f32 * 0.2126 + pa[1] as f32 * 0.7152 + pa[2] as f32 * 0.0722) * 0.18;
        let (r, g, bl) = heat(t);
        let mix = |c: f32| -> u8 { (lum * (1.0 - t) + c * t).clamp(0.0, 255.0) as u8 };
        *po = image::Rgba([mix(r), mix(g), mix(bl), 255]);
    }
    let pixels = (a.width() as u64 * a.height() as u64).max(1);
    Ok(ImageDiff {
        data_url: png_data_url(&out)?,
        changed_percent: changed as f64 * 100.0 / pixels as f64,
        mean_delta: total as f64 / pixels as f64,
        max_delta: max,
        same_size,
    })
}

fn heat(t: f32) -> (f32, f32, f32) {
    // 0 -> deep violet, 0.5 -> magenta, 1 -> warm yellow.
    if t < 0.5 {
        let k = t / 0.5;
        (60.0 + 180.0 * k, 20.0 + 20.0 * k, 120.0 + 80.0 * k)
    } else {
        let k = (t - 0.5) / 0.5;
        (240.0 + 15.0 * k, 40.0 + 200.0 * k, 200.0 - 120.0 * k)
    }
}

fn text_diff(old: &str, new: &str) -> Vec<DiffLine> {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();
    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Insert => "insert",
            ChangeTag::Delete => "delete",
        };
        lines.push(DiffLine {
            tag: tag.into(),
            old_line: change.old_index().map(|i| i + 1),
            new_line: change.new_index().map(|i| i + 1),
            text: change.value().trim_end_matches(['\n', '\r']).to_string(),
        });
    }
    lines
}

/// The thumbnail Blender embeds in a saved `.blend` (the `TEST` block).
/// Handles plain and zstd-compressed files, and both the classic header
/// (`BLENDER-v` / `BLENDER_v`) and the newer `BLENDER17-01v` one.
pub fn blend_thumbnail(bytes: &[u8]) -> Option<RgbaImage> {
    let owned;
    let data: &[u8] = if bytes.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        owned = zstd::stream::decode_all(Cursor::new(bytes)).ok()?;
        &owned
    } else {
        bytes
    };
    if !data.starts_with(b"BLENDER") || data.len() < 12 {
        return None;
    }
    // Classic: "BLENDER" + ptr ('_' 4 / '-' 8) + endian ('v'/'V') + 3-digit version.
    // New:     "BLENDER" + header size (2 digits) + '-' + format (2 digits) + endian + 4-digit version.
    let (mut pos, big_endian, layout) = if data[7] == b'_' || data[7] == b'-' {
        let ptr = if data[7] == b'_' { 4 } else { 8 };
        (12usize, data[8] == b'V', Layout::Classic { ptr })
    } else {
        let size: usize = std::str::from_utf8(data.get(7..9)?).ok()?.parse().ok()?;
        let endian = *data.get(12)?;
        (size, endian == b'V', Layout::Large)
    };
    let u32_at = |p: usize| -> Option<u32> {
        let b: [u8; 4] = data.get(p..p + 4)?.try_into().ok()?;
        Some(if big_endian { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    };
    let i64_at = |p: usize| -> Option<i64> {
        let b: [u8; 8] = data.get(p..p + 8)?.try_into().ok()?;
        Some(if big_endian { i64::from_be_bytes(b) } else { i64::from_le_bytes(b) })
    };
    for _ in 0..64 {
        let code = data.get(pos..pos + 4)?;
        let (len, header) = match layout {
            Layout::Classic { ptr } => (u32_at(pos + 4)? as usize, 16 + ptr),
            // code, SDNA index (4), old pointer (8), length (8), count (8).
            Layout::Large => (usize::try_from(i64_at(pos + 16)?).ok()?, 32),
        };
        let body = pos + header;
        if code == b"TEST" {
            let w = u32_at(body)? as usize;
            let h = u32_at(body + 4)? as usize;
            if w == 0 || h == 0 || w > 4096 || h > 4096 {
                return None;
            }
            let pixels = data.get(body + 8..body + 8 + w * h * 4)?;
            let mut img = RgbaImage::from_raw(w as u32, h as u32, pixels.to_vec())?;
            image::imageops::flip_vertical_in_place(&mut img);
            return Some(img);
        }
        if code == b"ENDB" || code == b"DNA1" {
            return None;
        }
        pos = body.checked_add(len)?;
    }
    None
}

#[derive(Clone, Copy)]
enum Layout {
    Classic { ptr: usize },
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classic_blend_with_thumb(w: u32, h: u32) -> Vec<u8> {
        let mut out = b"BLENDER-v405".to_vec();
        // REND block first (skipped), then TEST.
        out.extend(b"REND");
        out.extend(&8u32.to_le_bytes());
        out.extend(&[0u8; 8]); // old pointer
        out.extend(&0u32.to_le_bytes());
        out.extend(&1u32.to_le_bytes());
        out.extend(&[0u8; 8]);
        let len = 8 + w * h * 4;
        out.extend(b"TEST");
        out.extend(&len.to_le_bytes());
        out.extend(&[0u8; 8]);
        out.extend(&0u32.to_le_bytes());
        out.extend(&1u32.to_le_bytes());
        out.extend(&w.to_le_bytes());
        out.extend(&h.to_le_bytes());
        for y in 0..h {
            for _ in 0..w {
                out.extend(&[if y == 0 { 255 } else { 0 }, 0, 0, 255]);
            }
        }
        out.extend(b"ENDB");
        out
    }

    #[test]
    fn reads_classic_blend_thumbnail() {
        let bytes = classic_blend_with_thumb(4, 2);
        let img = blend_thumbnail(&bytes).expect("thumbnail");
        assert_eq!(img.dimensions(), (4, 2));
        // Blender stores rows bottom-up; the red first row ends up at the bottom.
        assert_eq!(img.get_pixel(0, 1)[0], 255);
        assert_eq!(img.get_pixel(0, 0)[0], 0);
    }

    #[test]
    fn reads_zstd_blend_thumbnail() {
        let bytes = classic_blend_with_thumb(3, 3);
        let packed = zstd::stream::encode_all(Cursor::new(bytes), 3).expect("zstd");
        assert!(blend_thumbnail(&packed).is_some());
    }

    #[test]
    fn reads_large_header_blend_thumbnail() {
        let mut out = b"BLENDER17-01v0500".to_vec();
        let (w, h) = (2u32, 2u32);
        out.extend(b"TEST");
        out.extend(&0u32.to_le_bytes());
        out.extend(&[0u8; 8]);
        out.extend(&((8 + w * h * 4) as i64).to_le_bytes());
        out.extend(&1i64.to_le_bytes());
        out.extend(&w.to_le_bytes());
        out.extend(&h.to_le_bytes());
        out.extend(std::iter::repeat_n(200u8, (w * h * 4) as usize));
        assert_eq!(blend_thumbnail(&out).expect("thumb").dimensions(), (2, 2));
    }

    #[test]
    fn rejects_non_blend() {
        assert!(blend_thumbnail(b"not a blend file").is_none());
    }

    #[test]
    fn image_diff_measures_change() {
        let a = RgbaImage::from_pixel(10, 10, image::Rgba([10, 10, 10, 255]));
        let mut b = a.clone();
        for x in 0..10 {
            b.put_pixel(x, 0, image::Rgba([200, 10, 10, 255]));
        }
        let d = image_diff(&a, &b, true).expect("diff");
        assert!((d.changed_percent - 10.0).abs() < 0.01);
        assert_eq!(d.max_delta, 190);
    }

    #[test]
    fn text_diff_marks_lines() {
        let lines = text_diff("a\nb\n", "a\nc\n");
        let tags: Vec<&str> = lines.iter().map(|l| l.tag.as_str()).collect();
        assert_eq!(tags, vec!["equal", "delete", "insert"]);
    }
}
