//! A chip's pictures on their own, outside a frame: its icon and its
//! custom screen picture as RGBA images, for a tool that shows them (the
//! build creator). They are looked up as a frame looks them up
//! ([`crate::lookups`]: a chip's game's, under its key there) and
//! colored as a frame colors them.

use crate::audit::Problems;
use crate::compose::to_rgb;
use crate::graphics::Graphics;
use saito_assets::{Palette, Tiles};
use saito_battle::Content;
use saito_content_api::ChipHandle;

/// An RGBA image, row by row; index 0 of a palette is see-through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// `tiles` laid out row by row, `w` by `h` tiles, in `palette`; none if
/// there are fewer tiles.
pub fn image(tiles: &Tiles, w: usize, h: usize, palette: &Palette) -> Option<Image> {
    if tiles.len() < w * h {
        return None;
    }
    let (pw, ph) = (w * 8, h * 8);
    let mut rgba = vec![0u8; pw * ph * 4];
    for t in 0..w * h {
        let tile = tiles.get(t)?;
        let (tx, ty) = (t % w * 8, t / w * 8);
        for (i, &p) in tile.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let (x, y) = (tx + i % 8, ty + i / 8);
            let c = to_rgb(palette[p as usize]);
            rgba[(y * pw + x) * 4..][..4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 0xFF]);
        }
    }
    Some(Image { width: pw as u32, height: ph as u32, rgba })
}

/// A chip's 16x16 icon, in its game's HUD's icon palette (the slots'
/// unpicked look).
pub fn chip_icon(graphics: &Graphics, c: &Content, chip: ChipHandle) -> Option<Image> {
    let (tiles, palette) = crate::lookups::chip_icon(graphics, c, chip, &mut Problems::default())?;
    image(tiles, 2, 2, palette)
}

/// Crop height (pixels) of the artwork region inside Legacy Collection Vol. 2's 512×551 chip PNGs
/// (removing the 119px bottom element/attack bar).
pub const HD_CHIP_ART_CROP_HEIGHT: u32 = 432;

/// Decode a single Legacy Collection Vol. 2 chip artwork PNG from `archive` at `(offset, size)`,
/// cropping off the bottom element/attack bar to keep the 512×432 artwork as RGBA8.
pub fn decode_hd_chip_art(archive: &std::path::Path, offset: u64, size: u32) -> Option<Image> {
    use std::io::{Read, Seek, SeekFrom};
    if size == 0 || size > 16 * 1024 * 1024 {
        return None;
    }
    let mut file = std::fs::File::open(archive).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut raw = vec![0u8; size as usize];
    file.read_exact(&mut raw).ok()?;
    decode_hd_chip_art_bytes(&raw)
}

/// Decode raw PNG bytes of a Legacy Collection chip artwork, cropping to [`HD_CHIP_ART_CROP_HEIGHT`].
pub fn decode_hd_chip_art_bytes(png_bytes: &[u8]) -> Option<Image> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];
    let w = info.width;
    let h = info.height.min(HD_CHIP_ART_CROP_HEIGHT);
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.get(..(w * h * 4) as usize)?.to_vec(),
        png::ColorType::Rgb => {
            let src = bytes.get(..(w * h * 3) as usize)?;
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for px in src.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 0xFF]);
            }
            out
        }
        _ => return None,
    };
    Some(Image { width: w, height: h, rgba })
}

/// A chip's 56x48 picture in the chip window, in its definition's
/// `art_palette` where it has one (a chip whose palette no ROM holds), else
/// its own.
pub fn chip_art(graphics: &Graphics, c: &Content, chip: ChipHandle) -> Option<Image> {
    let art = crate::lookups::chip_art(graphics, c, chip, &mut Problems::default())?;
    image(&art.picture.tiles, 7, 6, &c.chip(chip).art_palette.unwrap_or(art.picture.palette))
}

/// Like [`chip_art`], preferring the Legacy Collection HD artwork when `hd` is true and available,
/// falling back to the original 56x48 ROM picture otherwise.
pub fn chip_art_with_hd(graphics: &Graphics, c: &Content, chip: ChipHandle, hd: bool) -> Option<Image> {
    if hd {
        let key = saito_content_api::keys::local(&c.defs.chip(chip).key);
        if let Some(img) = graphics.hd_chip_art(key) {
            return Some((*img).clone());
        }
    }
    chip_art(graphics, c, chip)
}

/// The palette a chip family's element icon is drawn in: the icons' with
/// the family's six colors from color 10, or the icon's own where the game's graphics
/// draws it as a sprite. None: the game's graphics have no icon for the family.
fn element_palette(a: &saito_assets::CustomScreen, family: usize) -> Option<Palette> {
    match &a.element_sprite {
        Some(sprite) => Some(sprite.palette),
        None => {
            let mut palette = a.icon_palette;
            palette[10..].copy_from_slice(a.element_colors.get(family)?);
            Some(palette)
        }
    }
}

/// A chip family's 16x16 element icon, as the game's chip window
/// shows it for a chip of the family: its frame with its symbol over it
/// (`CustomScreen::element_icon`), in its palette. None: the game's graphics
/// have no icon for the family.
pub fn element_icon(graphics: &Graphics, family: saito_battle::content::ChipFamily) -> Option<Image> {
    let a = &graphics.bundle().custom;
    let family = family.0 as usize;
    let palette = element_palette(a, family)?;
    image(&a.element_icon(family)?, 2, 2, &palette)
}

/// A chip family's element icon as an SVG of 16x16 frame pixels: the frame
/// the game's graphics draw (`shapes.element_frame`) in the family's colors, smooth
/// at any size, with the symbol over it as its pixels, square-edged (the
/// icon's pixels inside the frame's ring: `CustomScreen::element_inside`).
/// The user: "you should draw the element icons in the ui with the smooth
/// frame as well". None: the graphics have no drawing of the frame, or no icon
/// for the family ([`element_icon`] is the picture).
pub fn element_icon_svg(graphics: &Graphics, family: saito_battle::content::ChipFamily) -> Option<String> {
    let a = &graphics.bundle().custom;
    let family = family.0 as usize;
    let d = a.shapes.element_frame.as_ref().filter(|d| d.size == [16, 16])?;
    let palette = element_palette(a, family)?;
    let inside = a.element_inside()?;
    let icon = a.elements.pixels.get(4 * family * Tiles::TILE..4 * (family + 1) * Tiles::TILE)?;
    let hex = |c: u16| format!("#{:06x}", to_rgb(c));
    // (A pixel's index in the icon, by x and y; a symbol's pixel's index,
    // none outside the ring.)
    let at = |x: usize, y: usize| (2 * (y / 8) + x / 8) * Tiles::TILE + 8 * (y % 8) + x % 8;
    let symbol = |x: usize, y: usize| Some(icon[at(x, y)]).filter(|&p| p != 0 && inside[at(x, y)]);
    // (Each row's runs of one color, a rectangle each.)
    let mut rects = String::new();
    for y in 0..16 {
        let mut x = 0;
        while x < 16 {
            let Some(p) = symbol(x, y) else {
                x += 1;
                continue;
            };
            let end = (x + 1..16).find(|&e| symbol(e, y) != Some(p)).unwrap_or(16);
            rects += &format!("<rect x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"1\" fill=\"{}\"/>", end - x, hex(palette[p as usize & 15]));
            x = end;
        }
    }
    let frame = saito_assets::shapes::recolor(&d.source, &|_, k| palette[k as usize & 15]);
    let end = frame.rfind("</svg>")?;
    Some(format!("{}<g shape-rendering=\"crispEdges\">{rects}</g>\n{}", &frame[..end], &frame[end..]))
}

/// A form's face in the emotion window (its definition's mugshot, the
/// plain emotion's), 32x16, in its first palette: a soul's or a Cross's
/// picture for a tool.
pub fn form_face(graphics: &Graphics, c: &Content, form: saito_content_api::FormHandle) -> Option<Image> {
    // (The emotion when nothing else holds: the plain one.)
    let (_, face) = crate::lookups::form_face(graphics, c, form, saito_battle::content::Emotion::default(), false, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}

/// A form's face in the emotion window for `emotion` (by its name in the
/// game's rules), of its second set with `variant`, 32x16, in its first
/// palette: what a round shows of a navi, for a tool.
pub fn face(graphics: &Graphics, c: &Content, form: saito_content_api::FormHandle, emotion: &str, variant: bool) -> Option<Image> {
    let emotion = c.rules().emotion.by_name(emotion)?;
    let (_, face) = crate::lookups::form_face(graphics, c, form, emotion, variant, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_hd_chip_art_crops_bottom_bar() {
        let (w, h) = (2u32, 551u32);
        let mut raw = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            let color = if y < HD_CHIP_ART_CROP_HEIGHT { [200u8, 100, 50] } else { [40, 40, 40] };
            for _ in 0..w {
                raw.extend_from_slice(&color);
            }
        }
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, w, h);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().unwrap();
            writer.write_image_data(&raw).unwrap();
        }
        let img = decode_hd_chip_art_bytes(&png_bytes).expect("decodes HD PNG");
        assert_eq!((img.width, img.height), (w, HD_CHIP_ART_CROP_HEIGHT));
        assert_eq!(img.rgba.len(), (w * HD_CHIP_ART_CROP_HEIGHT * 4) as usize);
        assert_eq!(&img.rgba[..4], &[200, 100, 50, 255]);
        assert_eq!(&img.rgba[img.rgba.len() - 4..], &[200, 100, 50, 255]);
    }
}
