//! PDF pages, rasterised for the window.
//!
//! The client has no webview (X-6) and embeds no PDF library, so a page is
//! rasterised by poppler's `pdftoppm` — the same poppler the career-ops core
//! already depends on for `pdftotext` — and painted into egui as a texture.
//! Nothing here renders PDF *semantics*: poppler does, and this module reads
//! the pixels it produced.
//!
//! The format chosen is PPM, deliberately. It is `P6\n<w> <h>\n<max>\n` and then
//! three bytes per pixel, so decoding it is thirty lines and NO dependency —
//! where a PNG or JPEG would pull a decoder crate in for a preview.
//!
//! The failure this module refuses to hide: `pdftoppm` missing, a PDF poppler
//! cannot read, and a PPM that is not what its header claims are three
//! different outcomes. Each says which one it is, because "no preview" with no
//! reason is the failure mode a user cannot act on.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One rasterised page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfPage {
    pub width: usize,
    pub height: usize,
    /// `width * height * 3` bytes, RGB, no padding.
    pub rgb: Vec<u8>,
}

impl PdfPage {
    /// The page as egui's image, one `Color32` per pixel.
    pub fn to_color_image(&self) -> egui::ColorImage {
        let mut pixels = Vec::with_capacity(self.width * self.height);
        for chunk in self.rgb.chunks_exact(3) {
            pixels.push(egui::Color32::from_rgb(chunk[0], chunk[1], chunk[2]));
        }
        egui::ColorImage::new([self.width, self.height], pixels)
    }
}

/// Why a preview could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// The rasteriser is not on this machine. One actionable sentence.
    ToolMissing,
    /// The rasteriser ran and refused the file, with its own words.
    Failed(String),
    /// The file is not there.
    NotFound(String),
    /// The bytes handed back were not a P6 PPM this module can read.
    Malformed(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::ToolMissing => write!(
                f,
                "No PDF preview is possible on this machine: install poppler-utils (pdftoppm), or open the file in your own viewer."
            ),
            PdfError::Failed(why) => write!(f, "The PDF could not be read: {why}"),
            PdfError::NotFound(path) => write!(f, "There is no file at {path}"),
            PdfError::Malformed(why) => {
                write!(f, "The rasteriser returned something unreadable: {why}")
            }
        }
    }
}

impl std::error::Error for PdfError {}

/// The resolution a preview is rasterised at.
///
/// 110 DPI is legible at the width a page gets in this window, and it is a
/// quarter of the memory a 220 DPI page would need for a preview nobody zooms
/// into.
pub const PREVIEW_DPI: u32 = 110;

/// How many pages a PDF has.
///
/// `pdfinfo` is asked first; when it is missing the count falls back to 1, and
/// the viewer's "next page" simply finds nothing more. A wrong count here is a
/// missing button, never a wrong page.
pub fn page_count(path: &Path) -> usize {
    if !path.is_file() {
        return 0;
    }
    let out = Command::new("pdfinfo").arg(path).output();
    let Ok(out) = out else { return 1 };
    if !out.status.success() {
        return 1;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Pages:") {
            if let Ok(pages) = rest.trim().parse::<usize>() {
                return pages.max(1);
            }
        }
    }
    1
}

/// A number no other render in this process will use.
fn next_scratch_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    // The stamp separates two PROCESSES; the counter separates two calls in
    // one. Neither needs a random-number dependency.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    (stamp << 16) | (NEXT.fetch_add(1, Ordering::Relaxed) & 0xffff)
}

/// Rasterise one page of a PDF.
///
/// `page` is 1-based, as poppler's `-f`/`-l` are.
pub fn render_page(path: &Path, page: usize, dpi: u32) -> Result<PdfPage, PdfError> {
    if !path.is_file() {
        return Err(PdfError::NotFound(path.display().to_string()));
    }
    // A private temporary directory per CALL. `pdftoppm` writes to a prefix
    // rather than to a file, so two renders must never share one: a page number
    // and a process id are not enough, because two previews of two documents
    // (and two tests) run at the same time in one process.
    let dir = std::env::temp_dir().join(format!(
        "career-ops-pdf-{}-{}",
        std::process::id(),
        next_scratch_id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| PdfError::Failed(format!("could not make a scratch directory: {e}")))?;
    let prefix: PathBuf = dir.join("page");

    let out = Command::new("pdftoppm")
        .arg("-r")
        .arg(dpi.to_string())
        .arg("-f")
        .arg(page.max(1).to_string())
        .arg("-l")
        .arg(page.max(1).to_string())
        .arg("-singlefile")
        .arg(path)
        .arg(&prefix)
        .output();

    let out = match out {
        Ok(out) => out,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(PdfError::ToolMissing);
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(PdfError::Failed(err.to_string()));
        }
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let why = stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("pdftoppm refused the file")
            .to_string();
        let _ = std::fs::remove_dir_all(&dir);
        return Err(PdfError::Failed(why));
    }

    let ppm = std::fs::read(prefix.with_extension("ppm")).map_err(|e| {
        PdfError::Failed(format!("pdftoppm wrote no page: {e}"))
    });
    let _ = std::fs::remove_dir_all(&dir);
    let bytes = ppm?;
    decode_ppm(&bytes)
}

/// Decode a binary PPM (`P6`).
///
/// The header is `P6`, whitespace, width, height, maxval, then a single
/// whitespace byte and the pixels. Comments (`#` to end of line) are allowed
/// between header fields — `pdftoppm` does not write them, but the format does,
/// and a decoder that quietly mis-reads three bytes because of one would be
/// producing a shifted image rather than an error.
pub fn decode_ppm(bytes: &[u8]) -> Result<PdfPage, PdfError> {
    if bytes.len() < 2 || &bytes[0..2] != b"P6" {
        return Err(PdfError::Malformed("not a binary PPM".into()));
    }
    let mut at = 2usize;
    let width = ppm_field(bytes, &mut at)? as usize;
    let height = ppm_field(bytes, &mut at)? as usize;
    let maxval = ppm_field(bytes, &mut at)?;
    if maxval != 255 {
        return Err(PdfError::Malformed(format!(
            "only 8-bit PPM is read (maxval {maxval})"
        )));
    }
    // Exactly ONE whitespace byte separates the header from the pixels. Skipping
    // a run of them would eat a leading 0x20 or 0x0a pixel byte.
    if at >= bytes.len() || !(bytes[at] as char).is_ascii_whitespace() {
        return Err(PdfError::Malformed("no header terminator".into()));
    }
    at += 1;

    let expected = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(3))
        .ok_or_else(|| PdfError::Malformed("the pixel count overflows".into()))?;
    let available = bytes.len() - at;
    if available < expected {
        return Err(PdfError::Malformed(format!(
            "{available} pixel bytes for a {width}x{height} page"
        )));
    }
    Ok(PdfPage {
        width,
        height,
        rgb: bytes[at..at + expected].to_vec(),
    })
}

/// One numeric header field, skipping the whitespace and comments the format
/// allows between them.
fn ppm_field(bytes: &[u8], at: &mut usize) -> Result<u32, PdfError> {
    loop {
        while *at < bytes.len() && (bytes[*at] as char).is_ascii_whitespace() {
            *at += 1;
        }
        if *at < bytes.len() && bytes[*at] == b'#' {
            while *at < bytes.len() && bytes[*at] != b'\n' {
                *at += 1;
            }
            continue;
        }
        break;
    }
    let start = *at;
    while *at < bytes.len() && bytes[*at].is_ascii_digit() {
        *at += 1;
    }
    if start == *at {
        return Err(PdfError::Malformed("a header field is missing".into()));
    }
    std::str::from_utf8(&bytes[start..*at])
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| PdfError::Malformed("a header field is not a number".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ppm_is_read_with_its_header_off() {
        let mut bytes = b"P6\n2 1\n255\n".to_vec();
        bytes.extend_from_slice(&[255, 0, 0, 0, 0, 255]);
        let page = decode_ppm(&bytes).expect("a plain PPM must decode");
        assert_eq!(page.width, 2);
        assert_eq!(page.height, 1);
        assert_eq!(page.rgb, vec![255, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn a_comment_in_the_header_does_not_shift_the_pixels() {
        let mut bytes = b"P6\n# written by a tool\n1 1\n255\n".to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        let page = decode_ppm(&bytes).expect("a commented PPM must decode");
        assert_eq!(page.rgb, vec![1, 2, 3]);
    }

    #[test]
    fn a_whitespace_pixel_value_is_not_eaten_by_the_header() {
        // The first pixel byte is a newline (0x0a): a decoder that skipped a
        // RUN of whitespace after maxval would shift the whole image.
        let mut bytes = b"P6\n1 1\n255\n".to_vec();
        bytes.extend_from_slice(&[0x0a, 0x20, 0x09]);
        let page = decode_ppm(&bytes).expect("a PPM starting with whitespace must decode");
        assert_eq!(page.rgb, vec![0x0a, 0x20, 0x09]);
    }

    #[test]
    fn a_truncated_page_is_an_error_not_a_short_image() {
        let mut bytes = b"P6\n2 2\n255\n".to_vec();
        bytes.extend_from_slice(&[1, 2, 3]);
        let err = decode_ppm(&bytes).expect_err("a short PPM must be refused");
        assert!(matches!(err, PdfError::Malformed(_)));
        assert!(err.to_string().contains("pixel bytes"));
    }

    #[test]
    fn something_that_is_not_a_ppm_says_so() {
        let err = decode_ppm(b"\x89PNG\r\n").expect_err("a PNG is not a PPM");
        assert!(matches!(err, PdfError::Malformed(_)));
    }

    #[test]
    fn one_page_is_painted_as_one_pixel_per_colour() {
        let page = PdfPage {
            width: 2,
            height: 1,
            rgb: vec![10, 20, 30, 40, 50, 60],
        };
        let image = page.to_color_image();
        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels[0], egui::Color32::from_rgb(10, 20, 30));
        assert_eq!(image.pixels[1], egui::Color32::from_rgb(40, 50, 60));
    }
}
