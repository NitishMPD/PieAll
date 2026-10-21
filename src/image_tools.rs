use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::collections::BTreeMap;

fn number(p: &BTreeMap<String, String>, key: &str, default: u32, max: u32) -> Result<u32, String> {
    let value = p.get(key).map_or(Ok(default), |s| {
        s.parse::<u32>()
            .map_err(|_| format!("{key} must be a whole number"))
    })?;
    if value > max {
        Err(format!("{key} must be at most {max}"))
    } else {
        Ok(value)
    }
}
fn color(p: &BTreeMap<String, String>, key: &str, default: &str) -> Result<[u8; 3], String> {
    let value = p
        .get(key)
        .map_or(default, String::as_str)
        .trim_start_matches('#');
    if value.len() != 6 || !value.is_ascii() {
        return Err(format!("{key} must have six HEX digits"));
    }
    let value = u32::from_str_radix(value, 16).map_err(|_| format!("Invalid {key} color"))?;
    Ok([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}

pub fn dimensions(
    width: u32,
    height: u32,
    padding: u32,
    ratio: &str,
) -> Result<(u32, u32), String> {
    if width == 0 || height == 0 || width > 16384 || height > 16384 || padding > 2048 {
        return Err("Invalid frame dimensions or padding.".into());
    }
    let mut w = width
        .checked_add(padding * 2)
        .ok_or("Frame dimensions overflow")?;
    let mut h = height
        .checked_add(padding * 2)
        .ok_or("Frame dimensions overflow")?;
    let aspect = match ratio {
        "natural" => None,
        "16:9" => Some((16, 9)),
        "4:5" => Some((4, 5)),
        "1:1" => Some((1, 1)),
        _ => return Err("Unsupported canvas ratio".into()),
    };
    if let Some((x, y)) = aspect {
        w = w.max((h as u64 * x).div_ceil(y) as u32);
        h = h.max((w as u64 * y).div_ceil(x) as u32);
    }
    if w == 0 || h == 0 || w > 16384 || h > 16384 || w as u64 * h as u64 > 40_000_000 {
        return Err("Framed output exceeds dimension or 40-megapixel limit.".into());
    }
    Ok((w, h))
}

pub fn frame(img: DynamicImage, p: &BTreeMap<String, String>) -> Result<DynamicImage, String> {
    let (w, h) = img.dimensions();
    let padding = number(p, "padding", 64, 2048)?;
    let radius = number(p, "corners", 24, 8192)?.min(w / 2).min(h / 2);
    let shadow = number(p, "shadow", 0, 64)?;
    let (nw, nh) = dimensions(
        w,
        h,
        padding,
        p.get("ratio").map_or("natural", String::as_str),
    )?;
    let gradient = match p.get("style").map_or("solid", String::as_str) {
        "solid" => false,
        "gradient" => true,
        _ => return Err("Choose solid or gradient background.".into()),
    };
    let start = color(p, "background", "F3EBDD")?;
    let end = if gradient {
        color(p, "secondary", "D3B8A0")?
    } else {
        start
    };
    let mut canvas = RgbaImage::new(nw, nh);
    for (y, row) in canvas.rows_mut().enumerate() {
        if y % 128 == 0 {
            crate::jobs::check()?;
        }
        for (x, pixel) in row.enumerate() {
            let t = (x + y) as f64 / (nw + nh - 2).max(1) as f64;
            *pixel = Rgba([0, 1, 2, 3].map(|i| {
                if i == 3 {
                    255
                } else {
                    (start[i] as f64 * (1.0 - t) + end[i] as f64 * t).round() as u8
                }
            }));
        }
    }
    let mut source = img.to_rgba8();
    for (y, row) in source.rows_mut().enumerate() {
        if y % 128 == 0 {
            crate::jobs::check()?;
        }
        for (x, px) in row.enumerate() {
            let dx = if (x as u32) < radius {
                radius - x as u32
            } else {
                (x as u32).saturating_sub(w - radius - 1)
            };
            let dy = if (y as u32) < radius {
                radius - y as u32
            } else {
                (y as u32).saturating_sub(h - radius - 1)
            };
            if dx * dx + dy * dy > radius * radius {
                px[3] = 0;
            }
        }
    }
    let (left, top) = ((nw - w) / 2, (nh - h) / 2);
    if shadow > 0 {
        let mut mask = image::GrayImage::new(nw, nh);
        for (x, y, px) in source.enumerate_pixels() {
            if let Some(target) = mask.get_pixel_mut_checked(left + x, top + y + shadow / 2) {
                target[0] = px[3] / 3;
            }
        }
        let mask = image::imageops::blur(&mask, shadow as f32 / 2.0);
        crate::jobs::check()?;
        for (px, a) in canvas.pixels_mut().zip(mask.pixels()) {
            for c in &mut px.0[..3] {
                *c = (*c as u32 * (255 - a[0] as u32) / 255) as u8;
            }
        }
    }
    image::imageops::overlay(&mut canvas, &source, left as i64, top as i64);
    Ok(DynamicImage::ImageRgba8(canvas))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_preserve_pixels_and_apply_ratio_gradient_and_shadow() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(40, 20, Rgba([255, 0, 0, 255])));
        let p = BTreeMap::from([
            ("padding".into(), "10".into()),
            ("corners".into(), "0".into()),
            ("ratio".into(), "1:1".into()),
            ("style".into(), "gradient".into()),
            ("background".into(), "FFFFFF".into()),
            ("secondary".into(), "000000".into()),
            ("shadow".into(), "4".into()),
        ]);
        let out = frame(img, &p).unwrap();
        assert_eq!(out.dimensions(), (60, 60));
        assert_eq!(out.get_pixel(30, 30), Rgba([255, 0, 0, 255]));
        assert!(out.get_pixel(0, 0)[0] > out.get_pixel(59, 59)[0]);
        assert!(dimensions(16384, 16384, 64, "16:9").is_err());
        assert!(dimensions(10, 10, 0, "bad").is_err());
        assert!(dimensions(10, 10, u32::MAX, "natural").is_err());
    }
}
