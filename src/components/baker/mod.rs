pub mod chat_area;
pub mod input_bar;
pub mod layout;
pub mod modals;
pub mod models;
pub mod sidebar;
pub mod storage;

pub use layout::Route;

pub(super) fn mime_from_filename(name: &str) -> &'static str {
    match name
        .rsplit('.')
        .next()
        .map(|ext| ext.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

pub(super) fn data_url_from_bytes(mime: &str, bytes: Vec<u8>) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}
