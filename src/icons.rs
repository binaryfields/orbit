use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use file_icon_provider::get_file_icon;

const RASTER_SIZE: u16 = 64;

#[derive(Default)]
pub struct IconCache {
    cache: HashMap<PathBuf, Option<egui::TextureHandle>>,
}

impl IconCache {
    pub fn get(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.cache.get(path) {
            return cached.clone();
        }
        let texture = load_icon(ctx, path);
        self.cache.insert(path.to_path_buf(), texture.clone());
        texture
    }
}

fn load_icon(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    let icon = get_file_icon(path, RASTER_SIZE).ok()?;
    let size = [icon.width as usize, icon.height as usize];
    let image = egui::ColorImage::from_rgba_premultiplied(size, &icon.pixels);
    Some(ctx.load_texture(
        path.display().to_string(),
        image,
        egui::TextureOptions::LINEAR,
    ))
}
