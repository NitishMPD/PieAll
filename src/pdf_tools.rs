//! Bounded local PDF tools. Rewriting is deliberately refused for interactive/signed files.
use crate::{jobs, output};
use lopdf::{Document, Object};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PAGES: usize = 500;

fn load(path: &Path) -> Result<Document, String> {
    jobs::check()?;
    output::plain_path(path)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("PDF input exceeds 64 MiB.".into());
    }
    let doc = Document::load_mem(&bytes).map_err(|e| format!("Cannot read PDF: {e}"))?;
    if doc.is_encrypted() {
        return Err("Password-protected PDFs are not supported yet.".into());
    }
    let count = doc.page_iter().take(MAX_PAGES + 1).count();
    if count == 0 || count > MAX_PAGES {
        return Err("PDF must contain 1–500 pages.".into());
    }
    jobs::check()?;
    Ok(doc)
}

/// Ascending page selections, without silently reordering or accepting invalid pages.
pub fn page_range(value: &str, count: u32) -> Result<BTreeSet<u32>, String> {
    if value.len() > 256 || value.trim().is_empty() {
        return Err("Enter pages such as 1-3,5 or all.".into());
    }
    if value.trim().eq_ignore_ascii_case("all") {
        return Ok((1..=count).collect());
    }
    let mut pages = BTreeSet::new();
    for part in value.split(',') {
        let part = part.trim();
        let parse = |s: &str| {
            s.trim()
                .parse::<u32>()
                .map_err(|_| "Invalid PDF page selection.".to_string())
        };
        let (start, end) = match part.split_once('-') {
            Some((a, b)) => (parse(a)?, parse(b)?),
            None => {
                let n = parse(part)?;
                (n, n)
            }
        };
        if start == 0 || end < start || end > count {
            return Err(format!(
                "Pages must be within 1–{count}, with ascending ranges."
            ));
        }
        pages.extend(start..=end);
    }
    Ok(pages)
}

struct LimitedWriter<'a> {
    file: &'a mut File,
    bytes: u64,
}
impl Write for LimitedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        jobs::check().map_err(std::io::Error::other)?;
        if self.bytes + bytes.len() as u64 > 128 * 1024 * 1024 {
            return Err(std::io::Error::other("PDF output exceeds 128 MiB."));
        }
        let n = self.file.write(bytes)?;
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
fn save(doc: &mut Document, path: &Path, label: &str, shrink: bool) -> Result<String, String> {
    jobs::check()?;
    let mut stage = output::Staged::new(path.parent().ok_or("Missing parent folder")?)?;
    doc.save_to(&mut LimitedWriter {
        file: &mut stage.file,
        bytes: 0,
    })
    .map_err(|e| e.to_string())?;
    if shrink
        && stage.file.metadata().map_err(|e| e.to_string())?.len()
            >= path.metadata().map_err(|e| e.to_string())?.len()
    {
        return Ok("PDF is already compact; no larger output was saved.".into());
    }
    Ok(format!(
        "Saved {}",
        stage.publish(path, label, "pdf")?.display()
    ))
}
fn editable(doc: &Document) -> Result<(), String> {
    let catalog = doc.catalog().map_err(|e| e.to_string())?;
    if catalog.has(b"AcroForm")
        || catalog.has(b"Perms")
        || doc.objects.values().any(|o| {
            o.as_dict().is_ok_and(|d| {
                d.get(b"Type")
                    .and_then(Object::as_name)
                    .is_ok_and(|t| t == b"Sig")
            })
        })
    {
        return Err("PDF rewriting currently supports ordinary documents. Forms and signed PDFs are preserved and refused.".into());
    }
    Ok(())
}
fn rotation(doc: &Document, mut id: lopdf::ObjectId) -> Result<i64, String> {
    for _ in 0..64 {
        let dict = doc.get_dictionary(id).map_err(|e| e.to_string())?;
        if let Ok(value) = dict.get(b"Rotate") {
            return value.as_i64().map_err(|e| e.to_string());
        }
        match dict.get(b"Parent").and_then(Object::as_reference) {
            Ok(parent) => id = parent,
            Err(_) => return Ok(0),
        }
    }
    Err("PDF page inheritance is too deep.".into())
}
pub fn run(id: &str, path: &Path, params: &BTreeMap<String, String>) -> Result<String, String> {
    run_with_progress(id, path, params, 0, 1)
}
pub fn run_with_progress(
    id: &str,
    path: &Path,
    params: &BTreeMap<String, String>,
    completed: usize,
    total: usize,
) -> Result<String, String> {
    let mut doc = load(path)?;
    let pages = doc.get_pages();
    if id == "pdf_details" {
        return Ok(format!("{}: {} pages; PDF {}; {} bytes. Text extraction is not OCR; rewriting does not sanitize active content.", path.display(), pages.len(), doc.version, path.metadata().map_err(|e| e.to_string())?.len()));
    }
    let selected = page_range(
        params.get("pages").map_or("all", String::as_str),
        pages.len() as u32,
    )?;
    if id == "pdf_text" {
        let mut text = String::new();
        for (index, page) in selected.iter().enumerate() {
            jobs::check()?;
            jobs::progress_fraction(
                completed,
                total,
                format!("Reading page {page}"),
                Some(index as f64 / selected.len() as f64),
            );
            let part = doc
                .extract_text(&[*page])
                .map_err(|e| format!("Cannot extract page {page}: {e}"))?;
            if text.len() + part.len() > 8 * 1024 * 1024 {
                return Err("Extracted text exceeds 8 MiB.".into());
            }
            text.push_str(&part);
            text.push('\n');
        }
        if text.trim().is_empty() {
            return Err(
                "No selectable text found. Scanned pages need OCR, which is not installed yet."
                    .into(),
            );
        }
        return Ok(format!(
            "Saved {} (selectable text, without layout preservation)",
            output::write(path, "text", "txt", text.as_bytes())?.display()
        ));
    }
    editable(&doc)?;
    match id {
        "pdf_extract" => {
            doc.delete_pages(
                &pages
                    .keys()
                    .filter(|p| !selected.contains(p))
                    .copied()
                    .collect::<Vec<_>>(),
            );
            // This is page extraction, not secure redaction. Shared objects may remain.
            doc.prune_objects();
            save(&mut doc, path, "extracted", false)
        }
        "pdf_split" => {
            if selected.len() > 100 {
                return Err("Split supports at most 100 output pages per run.".into());
            }
            let mut outputs = Vec::new();
            for (index, page) in selected.iter().enumerate() {
                jobs::check()?;
                jobs::progress_fraction(
                    completed,
                    total,
                    format!("Saving page {page}"),
                    Some(index as f64 / selected.len() as f64),
                );
                let mut single = doc.clone();
                single.delete_pages(
                    &pages
                        .keys()
                        .filter(|p| *p != page)
                        .copied()
                        .collect::<Vec<_>>(),
                );
                single.prune_objects();
                outputs.push(save(&mut single, path, &format!("page-{page}"), false)?);
            }
            Ok(outputs.join("\r\n"))
        }
        "pdf_rotate" => {
            let angle = params
                .get("angle")
                .map_or("90", String::as_str)
                .parse::<i64>()
                .map_err(|_| "Invalid rotation")?;
            if !matches!(angle, 90 | 180 | 270) {
                return Err("Choose 90, 180 or 270 degrees.".into());
            }
            for page in selected {
                jobs::check()?;
                let page_id = pages[&page];
                let old = rotation(&doc, page_id)?.rem_euclid(360);
                doc.get_dictionary_mut(page_id)
                    .map_err(|e| e.to_string())?
                    .set("Rotate", (old + angle).rem_euclid(360));
            }
            save(&mut doc, path, "rotated", false)
        }
        "pdf_compress" => {
            for object in doc.objects.values_mut() {
                jobs::check()?;
                if let Object::Stream(stream) = object {
                    if stream.allows_compression {
                        stream.compress().map_err(|e| e.to_string())?;
                    }
                }
            }
            save(&mut doc, path, "compressed", true)
        }
        "pdf_strip_metadata" => {
            doc.trailer.remove(b"Info");
            for object in doc.objects.values_mut() {
                jobs::check()?;
                match object {
                    Object::Dictionary(dict) => {
                        dict.remove(b"Metadata");
                    }
                    Object::Stream(stream) => {
                        stream.dict.remove(b"Metadata");
                    }
                    _ => {}
                }
            }
            doc.prune_objects();
            save(&mut doc, path, "clean", false)
        }
        _ => Err("Unknown PDF action".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    fn fixture(path: &Path) {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let pages = (0..3).map(|_| Object::Reference(doc.add_object(dictionary! { "Type" => "Page", "Parent" => root, "MediaBox" => vec![0.into(),0.into(),100.into(),200.into()] }))).collect::<Vec<_>>();
        doc.objects.insert(
            root,
            dictionary! { "Type" => "Pages", "Kids" => pages, "Count" => 3, "Rotate" => 90 }.into(),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => root });
        let info = doc.add_object(dictionary! { "Author" => Object::string_literal("Private") });
        doc.trailer.set("Root", catalog);
        doc.trailer.set("Info", info);
        doc.save(path).unwrap();
    }
    #[test]
    fn rejects_bad_ranges_without_expansion() {
        for value in ["0", "4", "2-1", "1-4294967295", "1,,2", ""] {
            assert!(page_range(value, 3).is_err());
        }
        assert_eq!(page_range("1-2,2,3", 3).unwrap(), BTreeSet::from([1, 2, 3]));
    }
    #[test]
    fn pdf_operations_preserve_source_and_handle_inherited_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.pdf");
        fixture(&path);
        let original = std::fs::read(&path).unwrap();
        let params = BTreeMap::from([("pages".into(), "2-3".into())]);
        run("pdf_extract", &path, &params).unwrap();
        assert_eq!(
            load(&dir.path().join("input-extracted.pdf"))
                .unwrap()
                .get_pages()
                .len(),
            2
        );
        run("pdf_rotate", &path, &params).unwrap();
        let rotated = load(&dir.path().join("input-rotated.pdf")).unwrap();
        assert_eq!(rotation(&rotated, rotated.get_pages()[&1]).unwrap(), 90);
        assert_eq!(rotation(&rotated, rotated.get_pages()[&2]).unwrap(), 180);
        run("pdf_split", &path, &params).unwrap();
        assert_eq!(
            load(&dir.path().join("input-page-2.pdf"))
                .unwrap()
                .get_pages()
                .len(),
            1
        );
        run("pdf_strip_metadata", &path, &BTreeMap::new()).unwrap();
        assert!(!load(&dir.path().join("input-clean.pdf"))
            .unwrap()
            .trailer
            .has(b"Info"));
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
    #[test]
    fn refuses_forms_and_leaves_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("form.pdf");
        fixture(&path);
        let mut doc = load(&path).unwrap();
        doc.catalog_mut().unwrap().set("AcroForm", dictionary! {});
        doc.save(&path).unwrap();
        assert!(run("pdf_rotate", &path, &BTreeMap::new()).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
