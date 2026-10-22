//! Bounded vertical screenshots using the page's Windows Scroll control pattern.
use super::*;
use std::io::{Read, Seek, SeekFrom, Write};

const MAX_PIXELS: usize = 256_000_000;
const CLIPBOARD_PIXELS: usize = 32_000_000;
const MAX_HEIGHT: usize = 262_144;
const MAX_TILES: usize = 512;

#[derive(Debug)]
struct Plan {
    width: usize,
    viewport: usize,
    height: usize,
    offsets: Vec<usize>,
}

impl Plan {
    fn new(width: usize, viewport: usize, view_percent: f64) -> Result<Self, String> {
        if width == 0
            || viewport < 4
            || !view_percent.is_finite()
            || view_percent <= 0.0
            || view_percent > 100.0
        {
            return Err("The page did not report valid scrolling dimensions.".into());
        }
        let height = (viewport as f64 * 100.0 / view_percent).round();
        if height > MAX_HEIGHT as f64
            || height < viewport as f64
            || width > 16_384
            || height * width as f64 > MAX_PIXELS as f64
        {
            return Err(format!("Page size is {width} x {height:.0} pixels. Full-page export supports up to 256 megapixels and 262144 pixels in height."));
        }
        let height = height as usize;
        let mut offsets = vec![0];
        let step = (viewport * 3 / 4).max(1);
        let bottom = height - viewport;
        while *offsets.last().unwrap() < bottom {
            if offsets.len() >= MAX_TILES {
                return Err("Full-page capture exceeds the 512-frame limit.".into());
            }
            offsets.push((offsets.last().unwrap() + step).min(bottom));
        }
        Ok(Self {
            width,
            viewport,
            height,
            offsets,
        })
    }

    fn percent(&self, offset: usize) -> f64 {
        if self.height == self.viewport {
            0.0
        } else {
            offset as f64 * 100.0 / (self.height - self.viewport) as f64
        }
    }

    fn offset(&self, percent: f64) -> Result<usize, String> {
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return Err("The page did not report its scroll position.".into());
        }
        Ok(((self.height - self.viewport) as f64 * percent / 100.0).round() as usize)
    }

    fn merge(
        &self,
        output: &mut (impl Write + Seek),
        frame: &[u8],
        y: usize,
        previous_end: usize,
    ) -> Result<usize, String> {
        let stride = self.width * 4;
        if frame.len() != self.viewport * stride
            || y > self.height - self.viewport
            || y > previous_end
            || (previous_end != 0 && y + self.viewport <= previous_end)
        {
            return Err("The page moved inconsistently; screenshot stitching stopped.".into());
        }
        // Split the overlap at its midpoint. This replaces the old frame's
        // bottom edge and discards the new frame's top edge, reducing repeated
        // small sticky headers/footers without changing page styles.
        let seam = if previous_end == 0 {
            0
        } else {
            y + (previous_end - y) / 2
        };
        let end = y + self.viewport;
        output
            .seek(SeekFrom::Start((seam * stride) as u64))
            .map_err(|e| e.to_string())?;
        output
            .write_all(&frame[(seam - y) * stride..])
            .map_err(|e| e.to_string())?;
        Ok(end)
    }
}

fn check_input() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    // Escape cancels, and all other physical keys/buttons also stop capture.
    if (1..=254).any(|key| unsafe { GetAsyncKeyState(key) } < 0) {
        Err(
            "Capture stopped because keyboard or mouse input changed. The clipboard was preserved."
                .into(),
        )
    } else {
        Ok(())
    }
}

unsafe fn same_page(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
    document: &IUIAutomationElement,
    rect: Rect,
    source: usize,
    pid: u32,
    action: &Shortcut,
) -> Result<(), String> {
    validate(source, pid, action)?;
    check_input()?;
    let (current_rect, current_document) =
        page_document(automation, root, source)?.ok_or("The page disappeared during capture.")?;
    if current_rect != rect
        || !automation
            .CompareElements(document, &current_document)
            .map_err(|e| e.to_string())?
            .as_bool()
    {
        return Err("The browser page or viewport changed. Capture stopped without replacing the clipboard.".into());
    }
    Ok(())
}

pub(super) unsafe fn capture(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
    source: usize,
    pid: u32,
    action: &Shortcut,
) -> Result<String, String> {
    let (rect, document) = page_document(automation, root, source)?
        .ok_or("The visible page area is unavailable. Focus a normal browser tab and try again.")?;
    let (width, viewport, _) = rect.size()?;
    let scroll = document.GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)
        .map_err(|_| "This page does not expose native scrolling controls. Full-page capture is unavailable on this page.")?;
    if scroll
        .CurrentHorizontallyScrollable()
        .map_err(|e| e.to_string())?
        .as_bool()
    {
        return Err(
            "Full-page capture currently supports vertical pages without horizontal overflow."
                .into(),
        );
    }
    if !scroll
        .CurrentVerticallyScrollable()
        .map_err(|e| e.to_string())?
        .as_bool()
    {
        return copy_viewport(source, pid, action, rect)
            .map(|()| "Full page copied to clipboard.".into());
    }
    let start = scroll
        .CurrentVerticalScrollPercent()
        .map_err(|e| e.to_string())?;
    let plan = Plan::new(
        width as usize,
        viewport as usize,
        scroll
            .CurrentVerticalViewSize()
            .map_err(|e| e.to_string())?,
    )?;
    plan.offset(start)?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let result = (|| -> Result<std::fs::File, String> {
        let mut output =
            tempfile::tempfile().map_err(|e| format!("Cannot stage screenshot: {e}"))?;
        let mut covered = 0;
        for target in &plan.offsets {
            same_page(automation, root, &document, rect, source, pid, action)?;
            scroll
                .SetScrollPercent(-1.0, plan.percent(*target))
                .map_err(|e| e.to_string())?;
            let settle = Instant::now() + Duration::from_secs(2);
            let began = Instant::now();
            let mut stable = 0;
            let actual = loop {
                std::thread::sleep(Duration::from_millis(75));
                same_page(automation, root, &document, rect, source, pid, action)?;
                let current_plan = Plan::new(
                    plan.width,
                    plan.viewport,
                    scroll
                        .CurrentVerticalViewSize()
                        .map_err(|e| e.to_string())?,
                )?;
                if current_plan.height.abs_diff(plan.height) > 2 {
                    return Err("The page height changed while scrolling. Let the page finish loading, then try again.".into());
                }
                let actual = plan.offset(
                    scroll
                        .CurrentVerticalScrollPercent()
                        .map_err(|e| e.to_string())?,
                )?;
                if actual.abs_diff(*target) <= 1 {
                    stable += 1;
                } else {
                    stable = 0;
                }
                if stable >= 2 && began.elapsed() >= Duration::from_millis(300) {
                    break actual;
                }
                if Instant::now() >= settle || Instant::now() >= deadline {
                    return Err(
                        "The page did not finish scrolling within the capture deadline.".into(),
                    );
                }
            };
            let frame = read_viewport(source, pid, action, rect)?;
            same_page(automation, root, &document, rect, source, pid, action)?;
            // Reject a user scroll that occurred while pixels were being read.
            if plan
                .offset(
                    scroll
                        .CurrentVerticalScrollPercent()
                        .map_err(|e| e.to_string())?,
                )?
                .abs_diff(actual)
                > 1
            {
                return Err(
                    "The page was scrolled during capture. The clipboard was preserved.".into(),
                );
            }
            let after = Plan::new(
                plan.width,
                plan.viewport,
                scroll
                    .CurrentVerticalViewSize()
                    .map_err(|e| e.to_string())?,
            )?;
            if after.height.abs_diff(plan.height) > 2 {
                return Err(
                    "The page height changed during capture. No partial screenshot was copied."
                        .into(),
                );
            }
            covered = plan.merge(&mut output, &frame, actual, covered)?;
            if Instant::now() >= deadline {
                return Err("Full-page capture reached its 180-second deadline.".into());
            }
        }
        if covered != plan.height {
            return Err(
                "The page bottom was not reached; no partial screenshot was copied.".into(),
            );
        }
        Ok(output)
    })();

    // Use the same element's Scroll pattern, with no foreground input. Restore
    // after errors/focus loss too, but never target a replacement tab/document.
    let restore = (|| -> Result<(), String> {
        if platform::window_profile(source, pid)?.is_none_or(|p| p.id != action.profile) {
            return Err("The original browser window is no longer available.".into());
        }
        let (_, current_document) = page_document(automation, root, source)?
            .ok_or("The original page is no longer available.")?;
        if !automation
            .CompareElements(&document, &current_document)
            .map_err(|e| e.to_string())?
            .as_bool()
        {
            return Err(
                "The original page changed; its scroll position could not be restored.".into(),
            );
        }
        scroll
            .SetScrollPercent(-1.0, start)
            .map_err(|e| e.to_string())?;
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            if plan
                .offset(
                    scroll
                        .CurrentVerticalScrollPercent()
                        .map_err(|e| e.to_string())?,
                )?
                .abs_diff(plan.offset(start)?)
                <= 1
            {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err("The starting scroll position could not be restored.".into());
            }
            std::thread::sleep(Duration::from_millis(75));
        }
    })();
    let mut output = match (result, restore) {
        (Ok(output), Ok(())) => output,
        (Err(error), Ok(())) => return Err(error),
        (result, Err(restore)) => {
            return Err(format!(
                "{} {restore}",
                result
                    .err()
                    .unwrap_or_else(|| "Capture was not copied.".into())
            ))
        }
    };
    same_page(automation, root, &document, rect, source, pid, action)?;
    output.rewind().map_err(|e| e.to_string())?;
    if plan.width * plan.height <= CLIPBOARD_PIXELS && plan.height <= 32_768 {
        let mut data = Vec::with_capacity(plan.width * plan.height * 4);
        output.read_to_end(&mut data).map_err(|e| e.to_string())?;
        platform::set_clipboard_image(plan.width as u32, plan.height as u32, &data)?;
        Ok("Full page copied to clipboard.".into())
    } else {
        let folder = screenshot_folder()?;
        let mut staged = crate::output::Staged::new(&folder)?;
        encode_png(&mut output, &mut staged.file, plan.width, plan.height)?;
        let path = staged.publish_named(&folder, "Full-page screenshot", "png")?;
        Ok(format!("Saved {}", path.display()))
    }
}

fn encode_png(
    input: &mut impl Read,
    output: impl Write,
    width: usize,
    height: usize,
) -> Result<(), String> {
    let mut encoder = png::Encoder::new(output, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let mut stream = writer.stream_writer().map_err(|e| e.to_string())?;
    let mut bgra = vec![0; width * 4];
    let mut rgb = vec![0; width * 3];
    for _ in 0..height {
        input.read_exact(&mut bgra).map_err(|e| e.to_string())?;
        for (source, destination) in bgra.chunks_exact(4).zip(rgb.chunks_exact_mut(3)) {
            destination.copy_from_slice(&[source[2], source[1], source[0]]);
        }
        stream.write_all(&rgb).map_err(|e| e.to_string())?;
    }
    stream.finish().map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

unsafe fn screenshot_folder() -> Result<std::path::PathBuf, String> {
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    let path = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None)
        .map_err(|e| e.to_string())?;
    let text = path.to_string().map_err(|e| e.to_string());
    CoTaskMemFree(Some(path.0 as _));
    let downloads = std::path::PathBuf::from(text?);
    crate::output::plain_path(&downloads)?;
    let folder = downloads.join("PieAll Screenshots");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streams_png_rows_in_rgb_order_and_rejects_incomplete_input() {
        let mut bytes = Vec::new();
        encode_png(
            &mut &b"\x00\x00\xff\xff\x00\xff\x00\xff\xff\x00\x00\xff\xff\xff\xff\xff"[..],
            &mut bytes,
            2,
            2,
        )
        .unwrap();
        let image = image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (2, 2));
        assert_eq!(
            image.into_raw(),
            [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
        );
        assert!(encode_png(&mut &b"short"[..], Vec::new(), 2, 2).is_err());
    }

    #[test]
    fn plans_overlaps_and_an_exact_bottom_frame_with_bounded_allocations() {
        let plan = Plan::new(1200, 800, 32.0).unwrap();
        assert_eq!(plan.height, 2500);
        assert_eq!(plan.offsets, [0, 600, 1200, 1700]);
        assert_eq!(plan.percent(1700), 100.0);
        assert_eq!(plan.offset(100.0).unwrap(), 1700);
        assert_eq!(Plan::new(1200, 800, 100.0).unwrap().offsets, [0]);
        for percent in [f64::NAN, f64::INFINITY, 0.0, -1.0, 101.0, 0.01] {
            assert!(Plan::new(1200, 800, percent).is_err());
        }
        assert!(Plan::new(16000, 2000, 5.0).is_err());
        assert!(Plan::new(1, 4, 0.01).is_err());
        assert_eq!(Plan::new(1920, 1000, 2.0).unwrap().height, 50000);
    }
    #[test]
    fn stitches_document_rows_without_duplicates_including_a_short_final_scroll() {
        let plan = Plan::new(3, 8, 32.0).unwrap(); // 25 rows, offsets 0,6,12,17
        let mut output = std::io::Cursor::new(vec![0; plan.width * plan.height * 4]);
        let mut end = 0;
        for &y in &plan.offsets {
            let frame: Vec<u8> = (y..y + plan.viewport)
                .flat_map(|row| vec![row as u8; plan.width * 4])
                .collect();
            end = plan.merge(&mut output, &frame, y, end).unwrap();
        }
        assert_eq!(end, 25);
        for (row, bytes) in output.get_ref().chunks_exact(plan.width * 4).enumerate() {
            assert!(bytes.iter().all(|b| *b == row as u8));
        }
        assert!(plan.merge(&mut output, &[], 0, 0).is_err());
        assert!(plan.merge(&mut output, &[0; 3 * 8 * 4], 10, 8).is_err());
    }
}
