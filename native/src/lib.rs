use std::sync::{Mutex, OnceLock};
use napi::bindgen_prelude::*;
use tesseract_rs::{TesseractAPI, TessPageSegMode};

#[macro_use]
extern crate napi_derive;

struct OcrEngine {
    api: TesseractAPI,
}

static ENGINE: OnceLock<Mutex<Option<OcrEngine>>> = OnceLock::new();

fn engine() -> Result<&'static Mutex<Option<OcrEngine>>> {
    Ok(ENGINE.get_or_init(|| Mutex::new(None)))
}

#[napi]
pub fn ocr_init(datapath: Option<String>, language: Option<String>) -> Result<()> {
    let lang = language.as_deref().unwrap_or("eng");
    let mut guard = engine()?.lock().map_err(|_| lock_err())?;
    if guard.is_some() {
        return Ok(());
    }
    let api = TesseractAPI::new();
    let init_res = match datapath.as_deref().filter(|s| !s.is_empty()) {
        Some(p) => api.init(p, lang),
        // embedded traineddata (feature embed-tessdata) — no external file
        None => api.init_embedded(lang),
    };
    init_res.map_err(|e| Error::new(Status::GenericFailure, format!("tesseract init: {e}")))?;
    api.set_page_seg_mode(TessPageSegMode::PSM_AUTO)
        .map_err(|e| Error::new(Status::GenericFailure, format!("tesseract psm: {e}")))?;
    *guard = Some(OcrEngine { api });
    Ok(())
}

/// OCR a decoded-in-Rust image buffer (any format the `image` crate reads:
/// png/jpeg/webp/gif). Returns recognized text, trailing whitespace trimmed
/// to match tesseract.js output shape.
#[napi]
pub fn ocr_recognize(buf: Buffer) -> Result<String> {
    let b: &[u8] = &buf;
    let img = image::load_from_memory(b)
        .map_err(|e| Error::new(Status::InvalidArg, format!("image decode: {e}")))?;
    // RGB 3bpp — matches upstream tesseract-rs test pattern; RGBA 4bpp
    // produced empty text with libtesseract's Pix conversion.
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let guard = engine()?.lock().map_err(|_| lock_err())?;
    let eng = guard
        .as_ref()
        .ok_or_else(|| Error::new(Status::GenericFailure, "ocr_init not called"))?;
    eng.api
        .set_image(rgb.as_raw(), w as i32, h as i32, 3, (w * 3) as i32)
        .map_err(|e| Error::new(Status::GenericFailure, format!("set_image: {e}")))?;
    let text = eng
        .api
        .get_utf8_text()
        .map_err(|e| Error::new(Status::GenericFailure, format!("get_text: {e}")))?;
    Ok(text.trim_end().to_string())
}

/// Free the engine handle (TessBaseAPIDelete via Drop). Memory returns to OS.
#[napi]
pub fn ocr_shutdown() -> Result<()> {
    let mut guard = engine()?.lock().map_err(|_| lock_err())?;
    *guard = None;
    Ok(())
}

fn lock_err() -> Error {
    Error::new(Status::GenericFailure, "ocr engine mutex poisoned")
}
