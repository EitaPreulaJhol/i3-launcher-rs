//! Icon resolution and rasterization into `tiny-skia` pixmaps.
//!
//! Resolution follows the XDG Icon Theme spec (see [`theme`]); rendering
//! supports SVG (via `resvg`, at the exact target size) and PNG (via `image`,
//! down/upscaled with Lanczos). Results are cached per icon name.

pub mod theme;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tiny_skia::{IntSize, Pixmap, Transform};

use crate::config::Appearance;

pub use theme::{detect_theme, IconResolver};

/// Resolved, already scaled icon pixmaps.
pub struct IconCache {
    resolver: IconResolver,
    size: u32,
    fallback: String,
    cache: HashMap<String, Option<Arc<Pixmap>>>,
}

impl IconCache {
    pub fn new(appearance: &Appearance, fallback_icon: &str) -> Self {
        Self {
            resolver: IconResolver::new(&appearance.icon_theme),
            size: appearance.icon_size.max(1),
            fallback: if fallback_icon.trim().is_empty() {
                "application-x-executable".to_string()
            } else {
                fallback_icon.to_string()
            },
            cache: HashMap::new(),
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn theme_name(&self) -> &str {
        self.resolver.theme_name()
    }

    /// Return the pixmap for `name`, falling back to the configured generic icon.
    pub fn get(&mut self, name: Option<&str>) -> Option<Arc<Pixmap>> {
        if let Some(name) = name {
            if let Some(pixmap) = self.load(name) {
                return Some(pixmap);
            }
        }
        if let Some(pixmap) = self.load(&self.fallback.clone()) {
            return Some(pixmap);
        }
        None
    }

    fn load(&mut self, name: &str) -> Option<Arc<Pixmap>> {
        if let Some(cached) = self.cache.get(name) {
            return cached.clone();
        }
        let pixmap = self
            .resolver
            .find(name, self.size)
            .and_then(|path| match render_icon(&path, self.size) {
                Ok(pixmap) => Some(pixmap),
                Err(err) => {
                    if std::env::var_os("I3L_DEBUG").is_some() {
                        eprintln!("i3-launcher: ícone `{}`: {err:#}", path.display());
                    }
                    None
                }
            })
            .map(Arc::new);
        self.cache.insert(name.to_string(), pixmap.clone());
        pixmap
    }
}

/// Render an icon file at (at most) `size` pixels on its longest side.
pub fn render_icon(path: &Path, size: u32) -> Result<Pixmap> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("não foi possível ler o ícone `{}`", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "svg" | "svgz" => render_svg(&bytes, size),
        "png" => render_png(&bytes, size),
        other => bail!("formato de ícone não suportado: `{other}`"),
    }
}

fn target_dimensions(width: f32, height: f32, size: u32) -> Result<(u32, u32, f32)> {
    if !(width.is_finite() && height.is_finite()) || width <= 0.0 || height <= 0.0 {
        bail!("ícone com dimensões inválidas ({width}x{height})");
    }
    let box_size = size.max(1) as f32;
    let scale = box_size / width.max(height);
    let w = (width * scale).ceil().clamp(1.0, box_size) as u32;
    let h = (height * scale).ceil().clamp(1.0, box_size) as u32;
    Ok((w, h, scale))
}

fn render_svg(bytes: &[u8], size: u32) -> Result<Pixmap> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default())
        .context("SVG inválido")?;
    let dimensions = tree.size();
    let (width, height) = (dimensions.width(), dimensions.height());
    let (pw, ph, scale) = target_dimensions(width, height, size)?;
    let mut pixmap = Pixmap::new(pw, ph).context("não foi possível alocar o pixmap do ícone")?;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

fn render_png(bytes: &[u8], size: u32) -> Result<Pixmap> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .context("PNG inválido")?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let (pw, ph, _) = target_dimensions(width as f32, height as f32, size)?;
    let scaled = if (pw, ph) != (width, height) {
        image::imageops::resize(&image, pw, ph, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };

    let (w, h) = scaled.dimensions();
    let mut data = scaled.into_raw();
    // tiny-skia expects premultiplied RGBA.
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;
        for channel in &mut pixel[..3] {
            *channel = ((*channel as u32 * alpha) / 255) as u8;
        }
    }
    let size = IntSize::from_wh(w, h).context("tamanho de ícone inválido")?;
    Pixmap::from_vec(data, size).context("pixmap de ícone inválido")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_svg_at_target_size() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48">
            <rect width="48" height="48" fill="#ff0000"/>
        </svg>"##;
        let pixmap = render_svg(svg.as_bytes(), 24).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (24, 24));
        let px = pixmap.pixel(12, 12).unwrap();
        assert_eq!(
            (px.red(), px.green(), px.blue(), px.alpha()),
            (255, 0, 0, 255)
        );
    }

    #[test]
    fn renders_png_and_scales() {
        let mut img = image::RgbaImage::new(10, 10);
        for px in img.pixels_mut() {
            *px = image::Rgba([0, 0, 255, 255]);
        }
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let pixmap = render_png(&bytes, 20).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (20, 20));
        let px = pixmap.pixel(10, 10).unwrap();
        assert_eq!((px.red(), px.green(), px.blue()), (0, 0, 255));
    }

    #[test]
    fn rejects_bad_dimensions() {
        assert!(target_dimensions(0.0, 10.0, 24).is_err());
        assert!(target_dimensions(f32::NAN, 10.0, 24).is_err());
    }
}
