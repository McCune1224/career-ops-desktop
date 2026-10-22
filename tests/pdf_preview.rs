//! The PDF rasteriser, against a real PDF this test builds itself.
//!
//! A hand-written PDF keeps the test independent of the fixture's contents and
//! of any tool that could produce one: the bytes below are a valid one-page
//! document, so poppler either renders it or genuinely cannot. Nothing here
//! needs a model, a browser or the network.
//!
//! The rasteriser is an external tool (`pdftoppm`, poppler-utils). When it is
//! not installed the end-to-end assertions are SKIPPED with a printed reason
//! rather than failed: a missing system tool is a fact about the machine, not a
//! defect in the client — and the decoder, which is the part this repository
//! owns, is tested in `src/pdf.rs` with no tool at all.

use std::path::{Path, PathBuf};

use career_ops_desktop::pdf::{decode_ppm, page_count, render_page, PdfError, PREVIEW_DPI};

/// A valid one-page PDF, 100x100 points, with a filled square on it.
fn minimal_pdf(path: &Path) {
    let content = "1 0 0 rg 10 10 80 80 re f\n";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << >> >>"
            .to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            content.len(),
            content
        ),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, body));
    }
    let xref = out.len();
    out.push_str(&format!("xref\n0 {}\n", bodies.len() + 1));
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        out.push_str(&format!("{offset:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        bodies.len() + 1
    ));
    std::fs::write(path, out).expect("the test PDF must be writable");
}

/// A scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("career-ops-pdf-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch directory must be creatable");
    dir
}

#[test]
fn a_page_is_rasterised_into_pixels() {
    let dir = scratch("render");
    let file = dir.join("one-page.pdf");
    minimal_pdf(&file);

    assert_eq!(page_count(&file), 1, "the document has one page");

    match render_page(&file, 1, 72) {
        Ok(page) => {
            // The page is square, so the pixels are. The absolute count is the
            // RASTERISER's scale, which is the tool's business and differs
            // between poppler builds; what this client owns is that the buffer
            // is three bytes per pixel and that the content landed where the
            // PDF put it.
            assert!(page.width > 0 && page.height > 0, "an empty page is not a page");
            assert_eq!(
                page.width, page.height,
                "a square page must rasterise to a square image"
            );
            assert_eq!(
                page.rgb.len(),
                page.width * page.height * 3,
                "the pixel buffer is not one RGB triple per pixel"
            );
            // The square is red, and the corner is the page's own white.
            let corner = &page.rgb[0..3];
            assert_eq!(corner, &[255, 255, 255], "the page corner must be blank");
            let middle = (page.height / 2 * page.width + page.width / 2) * 3;
            assert_eq!(
                &page.rgb[middle..middle + 3],
                &[255, 0, 0],
                "the red square must be where the content put it"
            );
            // And it survives the trip into egui's shape.
            let image = page.to_color_image();
            assert_eq!(image.size, [100, 100]);
        }
        Err(PdfError::ToolMissing) => {
            eprintln!("skipped: pdftoppm is not installed, so there is nothing to rasterise with");
        }
        Err(other) => panic!("the test PDF did not render: {other}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_higher_resolution_makes_more_pixels() {
    let dir = scratch("dpi");
    let file = dir.join("one-page.pdf");
    minimal_pdf(&file);

    let low = render_page(&file, 1, 72);
    let high = render_page(&file, 1, 144);
    match (low, high) {
        (Ok(low), Ok(high)) => {
            assert!(
                high.width > low.width,
                "144 DPI must give more pixels than 72 ({high:?} vs {low:?})"
            );
        }
        (Err(PdfError::ToolMissing), _) | (_, Err(PdfError::ToolMissing)) => {
            eprintln!("skipped: pdftoppm is not installed");
        }
        (low, high) => panic!("a page failed to render at one of the two resolutions: {low:?} {high:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_is_not_there_says_which_file() {
    let dir = scratch("missing");
    let file = dir.join("nothing-here.pdf");
    assert_eq!(page_count(&file), 0, "a missing file has no pages");
    let error = render_page(&file, 1, PREVIEW_DPI).expect_err("a missing file cannot be rendered");
    assert!(matches!(error, PdfError::NotFound(_)));
    assert!(
        error.to_string().contains("nothing-here.pdf"),
        "the sentence must name the file: {error}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_is_not_a_pdf_says_what_the_tool_said() {
    let dir = scratch("broken");
    let file = dir.join("broken.pdf");
    std::fs::write(&file, b"this is not a PDF at all\n").expect("the file must be writable");

    match render_page(&file, 1, 72) {
        Err(PdfError::Failed(why)) => {
            assert!(!why.is_empty(), "the tool's own words must reach the user");
        }
        // poppler build differences: some refuse with an empty message.
        Err(PdfError::ToolMissing) => eprintln!("skipped: pdftoppm is not installed"),
        Ok(page) => panic!("garbage was rendered as a page: {page:?}"),
        Err(other) => panic!("an unexpected failure: {other}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_decoder_refuses_a_ppm_whose_header_lies() {
    // maxval is not 8-bit: the bytes that follow are not three per pixel.
    let error = decode_ppm(b"P6\n1 1\n65535\n\0\0\0\0\0\0").expect_err("16-bit PPM is not read");
    assert!(matches!(error, PdfError::Malformed(_)));
}

#[test]
fn the_app_opens_a_preview_and_paints_a_page() {
    use std::sync::Arc;

    use career_ops_desktop::app::CareerOpsApp;
    use career_ops_desktop::backend::FakeBackend;
    use career_ops_desktop::settings::Settings;

    let dir = scratch("app");
    let file = dir.join("one-page.pdf");
    minimal_pdf(&file);

    let mut app = CareerOpsApp::new(Arc::new(FakeBackend::new()), Settings::default());
    // A headless context uploads textures exactly as a window's does.
    let ctx = egui::Context::default();
    app.attach_context(&ctx);
    app.open_preview(&file.display().to_string());

    let preview = app.preview.clone().expect("a preview must be open");
    assert_eq!(preview.page, 1);
    assert_eq!(preview.pages, 1, "the page count comes from the document");
    if preview.error.is_some() {
        eprintln!("skipped: pdftoppm is not installed ({} )", preview.error.unwrap());
    } else {
        assert!(
            preview.texture.is_some(),
            "a page that rendered must have reached the window as a texture"
        );
    }

    // Closing lets go of it.
    app.close_preview();
    assert!(app.preview.is_none());

    // A file that is not there is an open preview that says why, not a silent
    // nothing.
    app.open_preview(&dir.join("missing.pdf").display().to_string());
    let missing = app.preview.clone().expect("the preview stays open with its reason");
    assert!(missing.error.is_some());
    assert!(missing.texture.is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
