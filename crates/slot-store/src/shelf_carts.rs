use std::path::Path;

use crate::atomic::atomic_write;
use crate::slot_state::{list_path, platform_key, recent_value, Recent};

pub const SHELF_CARTS_FILE: &str = "shelf_carts.txt";

pub fn read_shelf_carts(root: &Path) -> Vec<(String, Recent)> {
    let text = std::fs::read_to_string(list_path(root, SHELF_CARTS_FILE)).unwrap_or_default();
    let mut carts: Vec<(String, Recent)> = Vec::new();
    for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
        if key.is_empty() || carts.iter().any(|(k, _)| k == key) {
            continue;
        }
        if let Some(cart) = recent_value(value) {
            carts.push((key.to_string(), cart));
        }
    }
    carts
}

pub fn write_shelf_carts(root: &Path, carts: &[(String, Recent)]) -> std::io::Result<()> {
    let text: String = carts
        .iter()
        .map(|(key, r)| format!("{key}={}/{}\n", platform_key(r.platform), r.stem))
        .collect();
    let path = list_path(root, SHELF_CARTS_FILE);
    if std::fs::read(&path).map_or(text.is_empty(), |old| old == text.as_bytes()) {
        return Ok(());
    }
    atomic_write(&path, text.as_bytes())
}
