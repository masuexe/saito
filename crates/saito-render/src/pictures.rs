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

/// Bounding box `(x, y, width, height)` of the 108×108 HD element icon inside Legacy Collection
/// Vol. 2's 512×551 chip PNG bottom bar.
pub const HD_ELEMENT_ICON_RECT: (u32, u32, u32, u32) = (138, 438, 108, 108);

/// Decode a single Legacy Collection Vol. 2 chip artwork PNG from `archive` at `(offset, size)`,
/// cropping off the bottom element/attack bar to keep the 512×432 artwork as RGBA8.
pub fn decode_hd_chip_art(archive: &std::path::Path, offset: u64, size: u32) -> Option<Image> {
    let raw = read_vol2_entry(archive, offset, size)?;
    decode_hd_chip_art_bytes(&raw)
}

/// Decode a single Legacy Collection Vol. 2 HD element icon (108×108 RGBA8 with transparent
/// rounded corners) from a chip PNG in `archive` at `(offset, size)`.
pub fn decode_hd_element_icon(archive: &std::path::Path, offset: u64, size: u32) -> Option<Image> {
    let raw = read_vol2_entry(archive, offset, size)?;
    decode_hd_element_icon_bytes(&raw)
}

fn read_vol2_entry(archive: &std::path::Path, offset: u64, size: u32) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    if size == 0 || size > 16 * 1024 * 1024 {
        return None;
    }
    let mut file = std::fs::File::open(archive).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut raw = vec![0u8; size as usize];
    file.read_exact(&mut raw).ok()?;
    Some(raw)
}

/// Decode raw PNG bytes of a Legacy Collection chip artwork, cropping to [`HD_CHIP_ART_CROP_HEIGHT`].
pub fn decode_hd_chip_art_bytes(png_bytes: &[u8]) -> Option<Image> {
    let (w, h, color_type, buf) = decode_png_raw(png_bytes)?;
    let ch = h.min(HD_CHIP_ART_CROP_HEIGHT);
    let rgba = slice_rgba(&buf, w, color_type, 0, 0, w, ch)?;
    Some(Image { width: w, height: ch, rgba })
}

/// Decode raw PNG bytes of a Legacy Collection 512×551 chip artwork and extract its 108×108 HD
/// element icon from [`HD_ELEMENT_ICON_RECT`], clearing the dark gray bar corners outside the
/// rounded icon border to transparent.
pub fn decode_hd_element_icon_bytes(png_bytes: &[u8]) -> Option<Image> {
    let (w, h, color_type, buf) = decode_png_raw(png_bytes)?;
    let (rx, ry, rw, rh) = HD_ELEMENT_ICON_RECT;
    if w < rx + rw || h < ry + rh {
        return None;
    }
    let mut rgba = slice_rgba(&buf, w, color_type, rx, ry, rw, rh)?;
    clear_outer_corners(&mut rgba, rw as usize, rh as usize);
    Some(Image { width: rw, height: rh, rgba })
}

fn decode_png_raw(png_bytes: &[u8]) -> Option<(u32, u32, png::ColorType, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some((info.width, info.height, info.color_type, buf))
}

fn slice_rgba(bytes: &[u8], src_w: u32, color_type: png::ColorType, rx: u32, ry: u32, rw: u32, rh: u32) -> Option<Vec<u8>> {
    let bpp = match color_type {
        png::ColorType::Rgba => 4usize,
        png::ColorType::Rgb => 3usize,
        _ => return None,
    };
    let mut out = Vec::with_capacity((rw * rh * 4) as usize);
    let stride = src_w as usize * bpp;
    for y in ry..ry + rh {
        let row_start = y as usize * stride + rx as usize * bpp;
        let row = bytes.get(row_start..row_start + rw as usize * bpp)?;
        if bpp == 4 {
            out.extend_from_slice(row);
        } else {
            for px in row.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 0xFF]);
            }
        }
    }
    Some(out)
}

/// Flood-fill from the four corners of the cropped 108×108 element icon to clear the `(40,40,40)`
/// bottom-bar background pixels outside the rounded grey frame to transparent.
fn clear_outer_corners(rgba: &mut [u8], w: usize, h: usize) {
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return;
    }
    let bg = [rgba[0], rgba[1], rgba[2]];
    let is_bg = |rgba: &[u8], idx: usize| -> bool {
        let p = &rgba[idx * 4..idx * 4 + 4];
        p[3] != 0
            && p[0].abs_diff(bg[0]) <= 12
            && p[1].abs_diff(bg[1]) <= 12
            && p[2].abs_diff(bg[2]) <= 12
    };
    let mut visited = vec![false; w * h];
    let mut stack = Vec::with_capacity(256);
    for (sx, sy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        let idx = sy * w + sx;
        if !visited[idx] && is_bg(rgba, idx) {
            visited[idx] = true;
            stack.push((sx, sy));
        }
    }
    while let Some((x, y)) = stack.pop() {
        let idx = y * w + x;
        rgba[idx * 4 + 3] = 0;
        let neighbors = [
            x.checked_sub(1).map(|nx| (nx, y)),
            (x + 1 < w).then_some((x + 1, y)),
            y.checked_sub(1).map(|ny| (x, ny)),
            (y + 1 < h).then_some((x, y + 1)),
        ];
        for (nx, ny) in neighbors.into_iter().flatten() {
            let nidx = ny * w + nx;
            if !visited[nidx] && is_bg(rgba, nidx) {
                visited[nidx] = true;
                stack.push((nx, ny));
            }
        }
    }
}

/// A chip's picture in the chip window (`56x48` for 7x6 EXE4–6 pictures, or
/// `64x56` for 8x7 EXE3 pictures), in its definition's `art_palette` where it
/// has one (a chip whose palette no ROM holds), else its own.
pub fn chip_art(graphics: &Graphics, c: &Content, chip: ChipHandle) -> Option<Image> {
    let art = crate::lookups::chip_art(graphics, c, chip, &mut Problems::default())?;
    let (w, h) = art.picture.tile_dims();
    image(&art.picture.tiles, w, h, &c.chip(chip).art_palette.unwrap_or(art.picture.palette))
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

/// A chip family's Legacy Collection HD element icon (108×108 RGBA8) when `hd` is true and
/// available from `vol2.dat`; `None` otherwise so callers can fall back to SVG/pixel icons.
pub fn hd_element_icon(graphics: &Graphics, family: saito_battle::content::ChipFamily, hd: bool) -> Option<Image> {
    if !hd {
        return None;
    }
    graphics.hd_element_icon(family.0).map(|img| (*img).clone())
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
    let (_, face) = crate::lookups::form_face(graphics, c, form, saito_battle::content::Emotion::default(), 0, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}

/// A form's face in the emotion window for `emotion` (by its name in the
/// game's rules), of its variant set `variant`, 32x16, in its first
/// palette: what a round shows of a navi, for a tool.
pub fn face(graphics: &Graphics, c: &Content, form: saito_content_api::FormHandle, emotion: &str, variant: u8) -> Option<Image> {
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

    #[test]
    fn decode_hd_element_icon_crops_and_clears_outer_corners() {
        let (w, h) = (512u32, 551u32);
        let (rx, ry, rw, rh) = HD_ELEMENT_ICON_RECT;
        let mut raw = vec![40u8; (w * h * 3) as usize];
        // Draw a bright border ring inset by 4px at corners, with a dark interior that must NOT be cleared.
        for y in 0..rh {
            for x in 0..rw {
                let in_corner = (x < 4 || x >= rw - 4) && (y < 4 || y >= rh - 4);
                let on_border = !in_corner && (x < 8 || x >= rw - 8 || y < 8 || y >= rh - 8);
                let color = if on_border { [170u8, 170, 170] } else { [40, 40, 40] };
                let idx = (((ry + y) * w + (rx + x)) * 3) as usize;
                raw[idx..idx + 3].copy_from_slice(&color);
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
        let icon = decode_hd_element_icon_bytes(&png_bytes).expect("decodes HD element icon");
        assert_eq!((icon.width, icon.height), (rw, rh));
        // Outer corner must be transparent.
        assert_eq!(icon.rgba[3], 0);
        // Border pixel must be opaque.
        let border_idx = ((6 * rw + 50) * 4) as usize;
        assert_eq!(icon.rgba[border_idx + 3], 255);
        // Dark interior pixel behind the border must stay opaque (not flood-filled).
        let center_idx = (((rh / 2) * rw + (rw / 2)) * 4) as usize;
        assert_eq!(icon.rgba[center_idx + 3], 255);
    }

    #[test]
    fn wide_8x7_picture_renders_at_64x56() {
        let mut pixels = vec![0u8; saito_assets::CustomScreen::WIDE_PICTURE_TILES * Tiles::TILE];
        // Mark the top-right tile (col 7, row 0 = tile index 7) pixel (0, 0) with color 1.
        pixels[7 * Tiles::TILE] = 1;
        let pic = saito_assets::Picture { tiles: Tiles { pixels }, palette: [0, 0x001F, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] };
        let (w, h) = pic.tile_dims();
        assert_eq!((w, h), (8, 7));
        let img = image(&pic.tiles, w, h, &pic.palette).expect("8x7 image");
        assert_eq!((img.width, img.height), (64, 56));
        // Tile (7, 0) pixel (0, 0) lands at x = 56, y = 0 (NOT wrapped onto row 1 as 7x6 would).
        let at_top_right = (56 * 4) as usize;
        assert_eq!(img.rgba[at_top_right + 3], 0xFF);
        assert_eq!(img.rgba[(8 * 64 * 4) + 3], 0x00);
    }
}
