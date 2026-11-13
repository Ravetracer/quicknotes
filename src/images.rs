//! Images of PDF pages – SVG, PNG and JPEG – rendered with hayro. Image
//! exports print the document to PDF first (like the PDF export), so they
//! look the same, and turn its pages into images.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::Pixmap;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Svg,
}

impl ImageFormat {
    pub const ALL: [Self; 3] = [Self::Png, Self::Jpeg, Self::Svg];

    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Svg => "svg",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPG",
            Self::Svg => "SVG",
        }
    }
}

/// Number of pages of a PDF (0 if it cannot be read).
pub fn page_count(pdf: &[u8]) -> usize {
    Pdf::new(pdf.to_vec()).map(|p| p.pages().len()).unwrap_or(0)
}

/// Pixels per PDF point (2 = 144 dpi, sharp on high-resolution screens).
const SCALE: f32 = 2.0;
/// Largest side of a raster image in pixels.
const MAX_PIXELS: f32 = 32_000.0;
/// Space kept below the content when trimming, in points (about 12 mm).
const TRIM_MARGIN: f32 = 34.0;

/// The pages of a PDF as images. `trim` cuts each page below its content
/// (for the single tall page of a whole document).
pub fn from_pdf(pdf: Vec<u8>, format: ImageFormat, trim: bool) -> Result<Vec<Vec<u8>>, String> {
    let pdf = Pdf::new(pdf).map_err(|e| format!("{e:?}"))?;
    let settings = InterpreterSettings::default();
    let cache = RenderCache::new();
    let mut out = Vec::new();
    for page in pdf.pages().iter() {
        let (width, height) = page.render_dimensions();
        let bottom = if trim { content_bottom(page, &cache, &settings, height) } else { height };
        let image = match format {
            ImageFormat::Svg => {
                let cache = hayro_svg::RenderCache::new();
                let svg_settings = hayro_svg::SvgRenderSettings { bg_color: [255, 255, 255, 255] };
                let svg = hayro_svg::convert(page, &cache, &settings, &svg_settings);
                crop_svg(&svg, width, bottom).into_bytes()
            }
            ImageFormat::Png | ImageFormat::Jpeg => {
                let scale = SCALE.min(MAX_PIXELS / width.max(bottom)).min(MAX_PIXELS / height);
                let pixmap = render(page, &cache, &settings, &RenderSettings::default(), &pixmap_settings(scale));
                let rows = ((bottom * scale).ceil() as u16).clamp(1, pixmap.height());
                encode(&pixmap, rows, format)?
            }
        };
        out.push(image);
    }
    if out.is_empty() {
        return Err("the PDF has no pages".into());
    }
    Ok(out)
}

fn pixmap_settings(scale: f32) -> PixmapSettings {
    PixmapSettings { x_scale: scale, y_scale: scale, bg_color: WHITE }
}

/// Where the content of a page ends (in points), plus a margin: found from
/// a small rendering of the page.
fn content_bottom<'a>(
    page: &'a hayro::hayro_syntax::page::Page<'a>,
    cache: &RenderCache<'a>,
    settings: &InterpreterSettings,
    height: f32,
) -> f32 {
    const PROBE: f32 = 0.5;
    let pixmap = render(page, cache, settings, &RenderSettings::default(), &pixmap_settings(PROBE));
    let width = usize::from(pixmap.width());
    let data = pixmap.data_as_u8_slice();
    let last = data
        .chunks_exact(width * 4)
        .rposition(|row| row.as_chunks::<4>().0.iter().any(|px| px[0] < 250 || px[1] < 250 || px[2] < 250));
    match last {
        Some(row) => ((row + 1) as f32 / PROBE + TRIM_MARGIN).min(height),
        None => height,
    }
}

/// The first `rows` rows of a pixmap as PNG or JPEG.
fn encode(pixmap: &Pixmap, rows: u16, format: ImageFormat) -> Result<Vec<u8>, String> {
    let width = pixmap.width();
    // Opaque (white background), so premultiplied equals straight RGBA.
    let data = &pixmap.data_as_u8_slice()[..usize::from(width) * usize::from(rows) * 4];
    match format {
        ImageFormat::Jpeg => {
            let mut out = Vec::new();
            jpeg_encoder::Encoder::new(&mut out, 90)
                .encode(data, width, rows, jpeg_encoder::ColorType::Rgba)
                .map_err(|e| e.to_string())?;
            Ok(out)
        }
        _ => {
            let mut cropped = Pixmap::new(width, rows);
            cropped.data_as_u8_slice_mut().copy_from_slice(data);
            cropped.into_png().map_err(|e| e.to_string())
        }
    }
}

/// An SVG of a page cut to `height` points (its root element starts with
/// `viewBox="0 0 W H" width="W" height="H"`).
fn crop_svg(svg: &str, width: f32, height: f32) -> String {
    let Some(end) = svg.find('>') else { return svg.to_owned() };
    let root = &svg[..end];
    if !root.starts_with("<svg") {
        return svg.to_owned();
    }
    let mut new_root = String::from("<svg");
    let mut rest = root[4..].trim_start();
    // Rewrites the size attributes, keeps all others.
    while let Some(eq) = rest.find("=\"") {
        let name = rest[..eq].trim();
        let Some(close) = rest[eq + 2..].find('"') else { break };
        let value = &rest[eq + 2..eq + 2 + close];
        let value = match name {
            "viewBox" => format!("0 0 {} {}", num(width), num(height)),
            "width" => num(width),
            "height" => num(height),
            _ => value.to_owned(),
        };
        new_root.push_str(&format!(" {name}=\"{value}\""));
        rest = rest[eq + 2 + close + 1..].trim_start();
    }
    format!("{new_root}{}", &svg[end..])
}

fn num(v: f32) -> String {
    let v = (v * 100.0).round() / 100.0;
    format!("{v}")
}

/// A small PNG for tests.
#[cfg(test)]
pub fn tests_png() -> Vec<u8> {
    let mut pixmap = Pixmap::new(40, 20);
    for (i, px) in pixmap.data_as_u8_slice_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
        px.copy_from_slice(if i % 40 < 20 { &[220, 40, 40, 255] } else { &[40, 90, 220, 255] });
    }
    pixmap.into_png().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crops_svg_root() {
        let svg = "<svg viewBox=\"0 0 595 842\" width=\"595\" height=\"842\" xmlns=\"http://www.w3.org/2000/svg\"><g/></svg>";
        assert_eq!(
            crop_svg(svg, 595.0, 300.456),
            "<svg viewBox=\"0 0 595 300.46\" width=\"595\" height=\"300.46\" xmlns=\"http://www.w3.org/2000/svg\"><g/></svg>"
        );
        assert_eq!(crop_svg("<g/>", 1.0, 1.0), "<g/>");
    }

    #[test]
    fn encodes_cropped_rasters() {
        let mut pixmap = Pixmap::new(4, 10);
        pixmap.data_as_u8_slice_mut().fill(255);
        let png = encode(&pixmap, 3, ImageFormat::Png).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        // Height in the IHDR chunk.
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3);
        let jpg = encode(&pixmap, 3, ImageFormat::Jpeg).unwrap();
        assert_eq!(&jpg[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn rejects_broken_pdfs() {
        assert!(from_pdf(b"not a pdf".to_vec(), ImageFormat::Png, false).is_err());
    }
}
