//! Local measurement only (untracked): time the stages of opening a PDF.
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: open_timing FILE");
    let t = Instant::now();
    let bytes = Arc::new(std::fs::read(&path).expect("read"));
    eprintln!("read        {:>8.2?}  ({} bytes)", t.elapsed(), bytes.len());
    let t = Instant::now();
    let cos = pdfcraft_cos::Document::open_with_password(bytes.clone(), None);
    eprintln!("cos open    {:>8.2?}  ok={}", t.elapsed(), cos.is_ok());
    drop(cos);
    let t = Instant::now();
    let info = pdfcraft_render::inspect(bytes.clone(), None);
    eprintln!("inspect     {:>8.2?}  pages={:?}", t.elapsed(), info.as_ref().map(|i| i.pages.len()).ok());
    drop(info);
    let t = Instant::now();
    let mut s = pdfcraft_engine::Session::new();
    let id = s.open("x.pdf", Some(path.clone()), bytes, None);
    eprintln!("Session::open {:>8.2?}  ok={}", t.elapsed(), id.is_ok());
}
