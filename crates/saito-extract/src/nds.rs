//! Optional Mega Man Battle Network 5: Double Team DS (EXE5DS) NitroFS asset
//! extractor and GBA fallback synthesizer.
//!
//! When an optional EXE5DS ROM (`A5TJ`, `A5TE`, or `A5TP`) is configured via
//! [`set_exe5ds_rom_path`] or the `SAITO_EXE5DS_ROM` environment variable, this
//! module extracts the DS-exclusive Sol Cross battle sprites, Sol Cross corner
//! sun, Sol Cross emotion-window faces, and Danger (Critical) faces for MegaMan
//! and all 12 Team Navis directly from the ROM's NitroFS filesystem.
//!
//! When no DS ROM is provided (or if reading it fails), clean GBA fallback
//! assets are synthesized in memory from the extracted GBA EXE5 bundle so
//! Sol Cross and 5DS Danger HUD rendering always work seamlessly.

use crate::{sprite, Error};
use saito_assets::{
    Bundle, FRAME_LAST, FRAME_LOOP, NaviDangerFace, Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles,
};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Supported EXE5DS NDS header game codes (JP / US / EU).
const VALID_GAME_CODES: [&[u8; 4]; 3] = [b"A5TJ", b"A5TE", b"A5TP"];

/// Minimum NDS header length required to validate game code and FNT/FAT fields.
const NDS_HEADER_MIN_LEN: usize = 0x200;

/// Pinned sprite IDs for EXE5DS assets (must match `content/exe5/compat/assets.toml`).
pub const SOLCROSS_MEGAMAN_SPRITE: (u8, u8) = (0x00, 0x0d);
pub const BUSTER_ARM_SPRITE: (u8, u8) = (0x0c, 0x03);
pub const SOL_SUN_SPRITE: (u8, u8) = (0x10, 0x00);

/// Animation index for Sol Cross's buster arm inside `buster-arm` (`0c-03`).
pub const SOLCROSS_BUSTER_ARM_ANIM: usize = 0x0E;

/// Number of GBA EXE5 mugshots before the appended EXE5DS slots (`0x00..=0x26`).
pub const GBA_EXE5_MUGSHOT_COUNT: usize = 0x27;

/// Number of appended EXE5DS mugshot slots (`0x27..=0x2F`).
pub const DS_MUGSHOT_SLOTS: usize = 9;

/// Number of Team Navis in EXE5 (`0x80..=0x8B`: 6 Team ProtoMan + 6 Team Colonel).
pub const TEAM_NAVI_COUNT: usize = 12;

/// NitroFS paths inside EXE5DS ROMs.
const PATH_PL0D: &str = "data/rom/a/pl0d.bin";
const PATH_PSHL03: &str = "data/rom/a/pshl03.bin";
const PATH_SHL00: &str = "data/rom/a/shl00.bin";
const PATH_KOKORO_PIX: &str = "data/rom/a/kokoro_pix.bin";
const PATH_KOKORO_NAVI: &str = "data/rom/a/kokoro_navi.bin";
const PATH_KOKORO_CLT: &str = "data/rom/a/kokoro_clt.bin";

/// Minimum expected sizes for the `kokoro_*` files in EXE5DS.
const KOKORO_PIX_MIN_LEN: usize = 0x2C80;
const KOKORO_NAVI_MIN_LEN: usize = 0x2880;
const KOKORO_CLT_MIN_LEN: usize = 0x600;

/// Global optional EXE5DS ROM path configured by the host/settings.
static EXE5DS_ROM_PATH: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Configure the optional EXE5DS `.nds` ROM path used during `"exe5"` asset
/// extraction. Pass `None` to clear (reverting to `SAITO_EXE5DS_ROM` if set,
/// or GBA fallback).
pub fn set_exe5ds_rom_path(path: Option<PathBuf>) {
    let mut guard = EXE5DS_ROM_PATH.write().unwrap_or_else(|e| e.into_inner());
    *guard = path;
}

/// Currently configured EXE5DS `.nds` ROM path, falling back to the
/// `SAITO_EXE5DS_ROM` environment variable if unset.
pub fn exe5ds_rom_path() -> Option<PathBuf> {
    if let Some(p) = EXE5DS_ROM_PATH.read().unwrap_or_else(|e| e.into_inner()).clone() {
        return Some(p);
    }
    std::env::var_os("SAITO_EXE5DS_ROM")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Validate an NDS header slice and total ROM size, returning
/// `(fnt_off, fnt_len, fat_off, fat_len)` when the ROM is a valid EXE5DS image
/// (`A5TJ`, `A5TE`, or `A5TP`) with in-bounds FNT and FAT tables.
fn validate_header_and_bounds(header: &[u8], file_len: u64) -> Option<(usize, usize, usize, usize)> {
    if file_len < NDS_HEADER_MIN_LEN as u64 || header.len() < 0x50 {
        return None;
    }
    let code: &[u8; 4] = header[0x0C..0x10].try_into().ok()?;
    if !VALID_GAME_CODES.contains(&code) {
        return None;
    }
    let rd32 = |off: usize| u32::from_le_bytes(header[off..off + 4].try_into().unwrap()) as u64;
    let fnt_off = rd32(0x40);
    let fnt_len = rd32(0x44);
    let fat_off = rd32(0x48);
    let fat_len = rd32(0x4C);
    if fnt_len < 8 || fat_len < 8 {
        return None;
    }
    if fnt_off.checked_add(fnt_len)? > file_len || fat_off.checked_add(fat_len)? > file_len {
        return None;
    }
    Some((
        usize::try_from(fnt_off).ok()?,
        usize::try_from(fnt_len).ok()?,
        usize::try_from(fat_off).ok()?,
        usize::try_from(fat_len).ok()?,
    ))
}

/// Return `true` if `path` points to a readable EXE5DS NDS ROM (`A5TJ`, `A5TE`,
/// or `A5TP`) whose FNT and FAT tables are within file bounds.
pub fn is_exe5ds_rom(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(meta) = file.metadata() else {
        return false;
    };
    if !meta.is_file() || meta.len() < NDS_HEADER_MIN_LEN as u64 {
        return false;
    }
    let mut header = [0u8; NDS_HEADER_MIN_LEN];
    if file.read_exact(&mut header).is_err() {
        return false;
    }
    validate_header_and_bounds(&header, meta.len()).is_some()
}

/// Read-only view of an NDS ROM's NitroFS filesystem.
pub struct NitroFs<'a> {
    rom: &'a [u8],
    fnt_off: usize,
    fnt_len: usize,
    fat_off: usize,
    fat_len: usize,
}

impl<'a> NitroFs<'a> {
    /// Validate an EXE5DS ROM image in memory and construct a [`NitroFs`] reader.
    pub fn parse(rom: &'a [u8]) -> Option<Self> {
        let (fnt_off, fnt_len, fat_off, fat_len) = validate_header_and_bounds(rom, rom.len() as u64)?;
        Some(Self { rom, fnt_off, fnt_len, fat_off, fat_len })
    }

    /// Look up a file inside NitroFS by slash-separated path (case-insensitive).
    pub fn read_file(&self, path: &str) -> Option<&'a [u8]> {
        let mut parts = path.split('/').filter(|s| !s.is_empty()).peekable();
        let mut dir_id: u16 = 0xF000;
        let mut depth = 0usize;
        while let Some(segment) = parts.next() {
            depth += 1;
            if depth > 32 {
                return None;
            }
            let want_dir = parts.peek().is_some();
            let dir_idx = (dir_id & 0x0FFF) as usize;
            let main_off = dir_idx.checked_mul(8)?;
            if main_off.checked_add(8)? > self.fnt_len {
                return None;
            }
            let entry = &self.rom[self.fnt_off + main_off..self.fnt_off + main_off + 8];
            let sub_off = u32::from_le_bytes(entry[0..4].try_into().unwrap()) as usize;
            let mut fid = u16::from_le_bytes(entry[4..6].try_into().unwrap()) as usize;
            if sub_off >= self.fnt_len {
                return None;
            }
            let mut pos = sub_off;
            let mut matched_dir = None;
            let mut matched_file = None;
            loop {
                if pos >= self.fnt_len {
                    return None;
                }
                let hdr = self.rom[self.fnt_off + pos];
                pos += 1;
                if hdr == 0 {
                    break;
                }
                let is_dir = (hdr & 0x80) != 0;
                let name_len = (hdr & 0x7F) as usize;
                let name_end = pos.checked_add(name_len)?;
                if name_end > self.fnt_len {
                    return None;
                }
                let name_bytes = &self.rom[self.fnt_off + pos..self.fnt_off + name_end];
                pos = name_end;
                let name_matches = std::str::from_utf8(name_bytes)
                    .is_ok_and(|n| n.eq_ignore_ascii_case(segment));
                if is_dir {
                    let id_end = pos.checked_add(2)?;
                    if id_end > self.fnt_len {
                        return None;
                    }
                    let sub_dir = u16::from_le_bytes(
                        self.rom[self.fnt_off + pos..self.fnt_off + id_end].try_into().unwrap(),
                    );
                    pos = id_end;
                    if want_dir && name_matches {
                        matched_dir = Some(sub_dir);
                        break;
                    }
                } else {
                    if !want_dir && name_matches {
                        matched_file = Some(fid);
                        break;
                    }
                    fid = fid.checked_add(1)?;
                }
            }
            if want_dir {
                dir_id = matched_dir?;
            } else {
                let file_id = matched_file?;
                let fat_entry_off = file_id.checked_mul(8)?;
                if fat_entry_off.checked_add(8)? > self.fat_len {
                    return None;
                }
                let fat_entry = &self.rom[self.fat_off + fat_entry_off..self.fat_off + fat_entry_off + 8];
                let start = u32::from_le_bytes(fat_entry[0..4].try_into().unwrap()) as usize;
                let end = u32::from_le_bytes(fat_entry[4..8].try_into().unwrap()) as usize;
                if start > end || end > self.rom.len() {
                    return None;
                }
                return Some(&self.rom[start..end]);
            }
        }
        None
    }
}

/// Extracted EXE5DS sprites and emotion-window assets ready to merge into an
/// EXE5 [`Bundle`].
#[derive(Clone, Debug)]
pub struct Exe5DsAssets {
    /// Sol Cross MegaMan battle sprite (`00-0d`, from `data/rom/a/pl0d.bin`).
    pub solcross_megaman: SpriteSheet,
    /// Extended buster arm sprite (`0c-03`, 15 animations including `0x0E` for
    /// Sol Cross, from `data/rom/a/pshl03.bin`).
    pub buster_arm: SpriteSheet,
    /// Sol Cross corner sun sprite (`10-00`, from `data/rom/a/shl00.bin`).
    pub sol_sun: SpriteSheet,
    /// Nine appended emotion-window mugshots (`0x27..=0x2F`):
    /// `(face_tiles, box_tiles, palette)`.
    pub mugshots: [(Tiles, Tiles, Palette); DS_MUGSHOT_SLOTS],
    /// Twelve Team Navi Danger faces (`0x80..=0x8B`).
    pub navi_danger_mugshots: Vec<Option<NaviDangerFace>>,
}

/// Mask off bit 15 of every BGR555 palette color (consistent with GBA `hud.palette`).
fn mask_palette(p: Palette) -> Palette {
    p.map(|c| c & 0x7FFF)
}

fn mask_sheet_palettes(sheet: &mut SpriteSheet) {
    for set in &mut sheet.palette_sets {
        for pal in set {
            *pal = mask_palette(*pal);
        }
    }
}

fn read_palette(data: &[u8], off: usize) -> Option<Palette> {
    let slice = data.get(off..off + 32)?;
    let pal = std::array::from_fn(|i| u16::from_le_bytes([slice[2 * i], slice[2 * i + 1]]) & 0x7FFF);
    Some(pal)
}

fn read_face_and_box(data: &[u8], off: usize) -> Option<(Tiles, Tiles)> {
    let face = Tiles::from_4bpp(data.get(off..off + 0x100)?);
    let box_tiles = Tiles::from_4bpp(data.get(off + 0x100..off + 0x180)?);
    Some((face, box_tiles))
}

/// Extract all EXE5DS assets from an in-memory `.nds` ROM image.
pub fn extract_from_bytes(rom: &[u8]) -> Result<Exe5DsAssets, Error> {
    let fs = NitroFs::parse(rom).ok_or_else(|| Error("not a valid EXE5DS ROM (expected A5TJ/A5TE/A5TP)".into()))?;

    let pl0d = fs.read_file(PATH_PL0D).ok_or_else(|| Error(format!("missing {PATH_PL0D} in EXE5DS ROM")))?;
    let mut solcross_megaman = sprite::sheet(pl0d, SOLCROSS_MEGAMAN_SPRITE.0, SOLCROSS_MEGAMAN_SPRITE.1)
        .ok_or_else(|| Error(format!("failed to decode {PATH_PL0D}")))?;
    mask_sheet_palettes(&mut solcross_megaman);

    let pshl03 = fs.read_file(PATH_PSHL03).ok_or_else(|| Error(format!("missing {PATH_PSHL03} in EXE5DS ROM")))?;
    let mut buster_arm = sprite::sheet(pshl03, BUSTER_ARM_SPRITE.0, BUSTER_ARM_SPRITE.1)
        .ok_or_else(|| Error(format!("failed to decode {PATH_PSHL03}")))?;
    mask_sheet_palettes(&mut buster_arm);
    if buster_arm.animations.len() <= SOLCROSS_BUSTER_ARM_ANIM {
        return Err(Error(format!(
            "{PATH_PSHL03} has {} animations (expected at least {})",
            buster_arm.animations.len(),
            SOLCROSS_BUSTER_ARM_ANIM + 1
        )));
    }

    let shl00 = fs.read_file(PATH_SHL00).ok_or_else(|| Error(format!("missing {PATH_SHL00} in EXE5DS ROM")))?;
    let mut sol_sun = sprite::sheet(shl00, SOL_SUN_SPRITE.0, SOL_SUN_SPRITE.1)
        .ok_or_else(|| Error(format!("failed to decode {PATH_SHL00}")))?;
    mask_sheet_palettes(&mut sol_sun);

    let pix = fs.read_file(PATH_KOKORO_PIX).ok_or_else(|| Error(format!("missing {PATH_KOKORO_PIX} in EXE5DS ROM")))?;
    let clt = fs.read_file(PATH_KOKORO_CLT).ok_or_else(|| Error(format!("missing {PATH_KOKORO_CLT} in EXE5DS ROM")))?;
    let navi = fs.read_file(PATH_KOKORO_NAVI).ok_or_else(|| Error(format!("missing {PATH_KOKORO_NAVI} in EXE5DS ROM")))?;

    if pix.len() < KOKORO_PIX_MIN_LEN || clt.len() < KOKORO_CLT_MIN_LEN || navi.len() < KOKORO_NAVI_MIN_LEN {
        return Err(Error("truncated kokoro_* emotion window files in EXE5DS ROM".into()));
    }

    // Verified against A5TJ `exe5ds.nds`:
    // - `kokoro_pix.bin` layout:
    //   - `0x0000 + pic * 0x180` (pic 0..=5): Base MegaMan (normal, angry, worried, full_synchro, evil, danger)
    //   - `0x0900` (12 * 0x100): 12 Double Souls
    //   - `0x1500` (11 * 0x80 = 0x580): 11 count boxes (10 down to 0)
    //   - `0x1A80 + (pic - 18) * 0x180` (pic 18..=23): BassCross (normal..danger)
    //   - `0x2380 + (pic - 24) * 0x180` (pic 24..=29): SolCross (normal..danger)
    // - `kokoro_clt.bin` layout (48 palettes x 32 bytes):
    //   - 0..=5: Base MegaMan (5 = danger)
    //   - 6..=17: 12 Double Souls normal
    //   - 18..=23: BassCross Gold (23 = danger)
    //   - 24..=29: SolCross (29 = danger)
    //   - 30..=41: 12 Double Souls Chaos Unison
    //   - 42..=46: BassCross Silver (normal..evil); slot 47 is all-zero in ROM
    //     because BassCross Gold & Silver share palette 23 for Danger.
    let mugshot_specs: [(usize, usize); DS_MUGSHOT_SLOTS] = [
        (0x2380, 24),             // 0x27: megaman-sol
        (0x2380 + 0x180, 25),     // 0x28: megaman-sol-angry
        (0x2380 + 2 * 0x180, 26), // 0x29: megaman-sol-worried
        (0x2380 + 3 * 0x180, 27), // 0x2a: megaman-sol-full-synchro
        (0x2380 + 4 * 0x180, 28), // 0x2b: megaman-sol-dark
        (5 * 0x180, 5),           // 0x2c: megaman-danger
        (0x1A80 + 5 * 0x180, 23), // 0x2d: megaman-hub-danger (BassCross Gold)
        (0x1A80 + 5 * 0x180, 23), // 0x2e: megaman-hub-colonel-danger (BassCross Silver shares pal 23)
        (0x2380 + 5 * 0x180, 29), // 0x2f: megaman-sol-danger
    ];
    let mut mugshots_vec = Vec::with_capacity(DS_MUGSHOT_SLOTS);
    for (pix_off, clt_idx) in mugshot_specs {
        let (face, box_tiles) = read_face_and_box(pix, pix_off)
            .ok_or_else(|| Error(format!("out-of-bounds kokoro_pix offset {pix_off:#x}")))?;
        let pal = read_palette(clt, clt_idx * 32)
            .ok_or_else(|| Error(format!("out-of-bounds kokoro_clt index {clt_idx}")))?;
        mugshots_vec.push((face, box_tiles, pal));
    }
    let mugshots: [(Tiles, Tiles, Palette); DS_MUGSHOT_SLOTS] = mugshots_vec.try_into().unwrap();

    // `kokoro_navi.bin` layout:
    // - `0x0000 + i * 0x180`: 12 Team Navis normal (face + box)
    // - `0x1200 + i * 0x180`: 12 Team Navis Danger (face + box)
    // - `0x2400 + i * 0x40`: normal + Full Synchro palettes
    // - `0x2700 + i * 0x20`: 12 Team Navis Danger palettes
    let mut navi_danger_mugshots = Vec::with_capacity(TEAM_NAVI_COUNT);
    for i in 0..TEAM_NAVI_COUNT {
        let (tiles, box_tiles) = read_face_and_box(navi, 0x1200 + i * 0x180)
            .ok_or_else(|| Error(format!("out-of-bounds kokoro_navi danger face {i}")))?;
        let palette = read_palette(navi, 0x2700 + i * 0x20)
            .ok_or_else(|| Error(format!("out-of-bounds kokoro_navi danger palette {i}")))?;
        navi_danger_mugshots.push(Some(NaviDangerFace { tiles, box_tiles, palette }));
    }

    Ok(Exe5DsAssets {
        solcross_megaman,
        buster_arm,
        sol_sun,
        mugshots,
        navi_danger_mugshots,
    })
}

/// Read an EXE5DS `.nds` ROM from disk and extract its assets.
pub fn extract_from_path(path: &Path) -> Result<Exe5DsAssets, Error> {
    if !is_exe5ds_rom(path) {
        return Err(Error(format!("{} is not a valid EXE5DS ROM", path.display())));
    }
    let bytes = std::fs::read(path)?;
    extract_from_bytes(&bytes)
}

/// Apply EXE5DS assets to an extracted EXE5 [`Bundle`]: uses the configured
/// EXE5DS ROM if set and valid, otherwise seamlessly synthesizes GBA fallback
/// assets. Returns any non-fatal warnings.
pub fn apply_exe5_ds_assets(graphics: &mut Bundle) -> Vec<String> {
    let configured = exe5ds_rom_path();
    apply_exe5_ds_assets_from_path(graphics, configured.as_deref())
}

/// Like [`apply_exe5_ds_assets`], taking an explicit optional ROM path (useful
/// for deterministic testing without touching global state).
pub fn apply_exe5_ds_assets_from_path(graphics: &mut Bundle, rom_path: Option<&Path>) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(path) = rom_path {
        match extract_from_path(path) {
            Ok(ds) => {
                merge_ds_assets(graphics, ds);
                return warnings;
            }
            Err(e) => {
                warnings.push(format!("exe5ds ROM fallback ({e}); using synthesized GBA assets"));
            }
        }
    }
    apply_gba_fallback(graphics);
    warnings
}

fn upsert_sprite(sprites: &mut Vec<SpriteSheet>, sheet: SpriteSheet) {
    let key = (sheet.category, sheet.index);
    if let Some(existing) = sprites.iter_mut().find(|s| (s.category, s.index) == key) {
        *existing = sheet;
    } else {
        sprites.push(sheet);
    }
    sprites.sort_by_key(|s| (s.category, s.index));
}

fn ensure_mugshot_slots(graphics: &mut Bundle, entries: [(Tiles, Tiles, Palette); DS_MUGSHOT_SLOTS]) {
    let target_len = GBA_EXE5_MUGSHOT_COUNT + DS_MUGSHOT_SLOTS;
    if graphics.hud.mugshots.len() < GBA_EXE5_MUGSHOT_COUNT {
        graphics.hud.mugshots.resize(GBA_EXE5_MUGSHOT_COUNT, (Tiles::default(), [0u16; 16]));
    }
    if graphics.hud.mugshot_boxes.len() < graphics.hud.mugshots.len() {
        graphics.hud.mugshot_boxes.resize(graphics.hud.mugshots.len(), Tiles::default());
    }
    if graphics.hud.mugshots.len() < target_len {
        graphics.hud.mugshots.resize(target_len, (Tiles::default(), [0u16; 16]));
        graphics.hud.mugshot_boxes.resize(target_len, Tiles::default());
    }
    for (offset, (face, box_tiles, pal)) in entries.into_iter().enumerate() {
        let slot = GBA_EXE5_MUGSHOT_COUNT + offset;
        graphics.hud.mugshots[slot] = (face, pal);
        graphics.hud.mugshot_boxes[slot] = box_tiles;
    }
}

fn merge_ds_assets(graphics: &mut Bundle, ds: Exe5DsAssets) {
    upsert_sprite(&mut graphics.sprites, ds.solcross_megaman);
    upsert_sprite(&mut graphics.sprites, ds.buster_arm);
    upsert_sprite(&mut graphics.sprites, ds.sol_sun);
    ensure_mugshot_slots(graphics, ds.mugshots);
    graphics.hud.navi_danger_mugshots = ds.navi_danger_mugshots;
}

// ---- GBA Fallback Synthesis -------------------------------------------------

/// Remap a GBA BGR555 palette into a warm Sol Cross palette (gold/amber/crimson
/// armor highlights while keeping skin tones and dark outlines crisp).
fn remap_solar_palette(pal: Palette) -> Palette {
    let mut out = [0u16; 16];
    for (i, &c) in pal.iter().enumerate() {
        if i == 0 || c == 0 {
            out[i] = c & 0x7FFF;
            continue;
        }
        let r = c & 31;
        let g = (c >> 5) & 31;
        let b = (c >> 10) & 31;
        let (nr, ng, nb) = if b > r && b >= g {
            // Blue/cyan armor -> solar gold/amber/crimson.
            let nr = (b + 8).min(31);
            let ng = ((b * 3) / 5 + g / 2).min(29);
            let nb = (r / 2 + 2).min(14);
            (nr, ng, nb)
        } else if r >= 22 && g >= 22 && b >= 22 {
            // Bright white highlights -> warm sunlit cream/white.
            (31, g.min(30), b.saturating_sub(4))
        } else {
            // Skin/outlines/accents -> subtle warm shift.
            ((r + 3).min(31), (g + 1).min(31), b.saturating_sub(2))
        };
        out[i] = nr | (ng << 5) | (nb << 10);
    }
    out
}

/// Remap a GBA BGR555 palette into a high-contrast crimson/rose Danger palette
/// inspired by EXE5DS `kokoro_clt.bin` slots 5 / 23 / 29.
fn remap_danger_palette(pal: Palette) -> Palette {
    let mut out = [0u16; 16];
    for (i, &c) in pal.iter().enumerate() {
        if i == 0 || c == 0 {
            out[i] = c & 0x7FFF;
            continue;
        }
        let r = c & 31;
        let g = (c >> 5) & 31;
        let b = (c >> 10) & 31;
        let lum = (r * 3 + g * 5 + b * 2) / 10;
        let nr = (lum + 11).min(31);
        let ng = ((lum * 2) / 5).min(22);
        let nb = ((lum * 3) / 5 + 4).min(27);
        out[i] = nr | (ng << 5) | (nb << 10);
    }
    out
}

/// Synthesize a clean 32x32 golden corner sun sprite sheet (`10-00`) anchored
/// at `(x = -32, y = 0)` matching EXE5DS `shl00.bin`'s top-right corner anchor.
fn synthesize_sol_sun_sprite() -> SpriteSheet {
    // 4x4 tiles (32x32 pixels) drawing a stylized golden sun & rays in the top-right corner.
    let mut tiles = Tiles { pixels: vec![0u8; 16 * Tiles::TILE] };
    let mut set_px = |x: usize, y: usize, color: u8| {
        if x < 32 && y < 32 {
            let tile_idx = (y / 8) * 4 + (x / 8);
            tiles.pixels[tile_idx * Tiles::TILE + (y % 8) * 8 + (x % 8)] = color;
        }
    };
    let cx: i32 = 24;
    let cy: i32 = 7;
    for y in 0..32usize {
        for x in 0..32usize {
            let dx = x as i32 - cx;
            let dy = y as i32 - cy;
            let d2 = dx * dx + dy * dy;
            let color = if d2 <= 16 {
                5 // white-hot core
            } else if d2 <= 42 {
                4 // bright sun yellow
            } else if d2 <= 76 {
                3 // rich solar gold
            } else if d2 <= 110 {
                2 // warm orange corona
            } else if d2 <= 220 && ((dx.abs() <= 1 && dy > 0) || (dy.abs() <= 1 && dx < 0) || (dx + dy).abs() <= 2) {
                1 // soft golden rays extending inward
            } else {
                0
            };
            if color != 0 {
                set_px(x, y, color);
            }
        }
    }
    let pal_a: Palette = [
        0x0000,
        saito_assets::bgr555([0xE8, 0x80, 0x18]),
        saito_assets::bgr555([0xF8, 0xA8, 0x20]),
        saito_assets::bgr555([0xFF, 0xD0, 0x38]),
        saito_assets::bgr555([0xFF, 0xF0, 0x78]),
        saito_assets::bgr555([0xFF, 0xFF, 0xD8]),
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let pal_b: Palette = [
        0x0000,
        saito_assets::bgr555([0xF8, 0x98, 0x20]),
        saito_assets::bgr555([0xFF, 0xC0, 0x30]),
        saito_assets::bgr555([0xFF, 0xE0, 0x58]),
        saito_assets::bgr555([0xFF, 0xF8, 0xA0]),
        saito_assets::bgr555([0xFF, 0xFF, 0xF0]),
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let parts = vec![SpritePart {
        tile: 0,
        x: -32,
        y: 0,
        width: 32,
        height: 32,
        hflip: false,
        vflip: false,
        palette: 0,
    }];
    SpriteSheet {
        category: SOL_SUN_SPRITE.0,
        index: SOL_SUN_SPRITE.1,
        tilesets: vec![tiles],
        palette_sets: vec![vec![pal_a], vec![pal_b]],
        part_lists: vec![parts],
        animations: vec![vec![
            SpriteFrame { tileset: 0, palette_set: 0, parts: 0, duration: 15, flags: 0 },
            SpriteFrame { tileset: 0, palette_set: 1, parts: 0, duration: 15, flags: FRAME_LAST | FRAME_LOOP },
        ]],
    }
}

fn minimal_fallback_sprite(category: u8, index: u8) -> SpriteSheet {
    let tiles = Tiles { pixels: vec![1u8; Tiles::TILE] };
    let pal: Palette = [
        0x0000,
        saito_assets::bgr555([0xFF, 0xC8, 0x30]),
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    SpriteSheet {
        category,
        index,
        tilesets: vec![tiles],
        palette_sets: vec![vec![pal]],
        part_lists: vec![vec![SpritePart { tile: 0, x: -4, y: -4, width: 8, height: 8, ..Default::default() }]],
        animations: vec![vec![SpriteFrame { duration: 1, flags: FRAME_LAST, ..Default::default() }]],
    }
}

/// Synthesize GBA fallback EXE5DS sprites & emotion-window mugshots directly
/// from the already-extracted GBA EXE5 bundle.
pub fn apply_gba_fallback(graphics: &mut Bundle) {
    // 1. `solcross-megaman` (`00-0d`): clone GBA `megaman` (`00-00`) with solar palette remap.
    let solcross_sheet = match graphics.sprite(0x00, 0x00).cloned() {
        Some(mut sheet) => {
            sheet.category = SOLCROSS_MEGAMAN_SPRITE.0;
            sheet.index = SOLCROSS_MEGAMAN_SPRITE.1;
            for set in &mut sheet.palette_sets {
                for pal in set {
                    *pal = remap_solar_palette(*pal);
                }
            }
            sheet
        }
        None => minimal_fallback_sprite(SOLCROSS_MEGAMAN_SPRITE.0, SOLCROSS_MEGAMAN_SPRITE.1),
    };
    upsert_sprite(&mut graphics.sprites, solcross_sheet);

    // 2. `sol-sun` (`10-00`): dedicated synthesized corner solar icon.
    upsert_sprite(&mut graphics.sprites, synthesize_sol_sun_sprite());

    // 3. `buster-arm` (`0c-03`): ensure animation `0x0E` exists and its palette set
    //    has EXE5DS Sol Cross palette slots `0x21` (normal) and `0x22` (no_charge).
    if let Some(arm) = graphics.sprites.iter_mut().find(|s| (s.category, s.index) == BUSTER_ARM_SPRITE) {
        if arm.animations.len() <= SOLCROSS_BUSTER_ARM_ANIM {
            let src_anim_idx = arm.animations.len().saturating_sub(1);
            let template_anim = arm.animations.get(src_anim_idx).cloned().unwrap_or_default();
            let mut remapped_sets = std::collections::HashMap::<u16, u16>::new();
            let mut sol_anim = template_anim.clone();
            for frame in &mut sol_anim {
                let new_ps = *remapped_sets.entry(frame.palette_set).or_insert_with(|| {
                    let mut cloned: Vec<Palette> = arm
                        .palette_sets
                        .get(frame.palette_set as usize)
                        .cloned()
                        .unwrap_or_default();
                    let base = cloned.first().copied().unwrap_or([0u16; 16]);
                    let sol_normal = remap_solar_palette(base);
                    let sol_dark = remap_danger_palette(base);
                    if cloned.len() <= 0x22 {
                        cloned.resize(0x23, sol_normal);
                    }
                    cloned[0x21] = sol_normal;
                    cloned[0x22] = sol_dark;
                    arm.palette_sets.push(cloned);
                    (arm.palette_sets.len() - 1) as u16
                });
                frame.palette_set = new_ps;
            }
            while arm.animations.len() < SOLCROSS_BUSTER_ARM_ANIM {
                arm.animations.push(template_anim.clone());
            }
            arm.animations.push(sol_anim);
        }
    }

    // 4. Mugshots `0x27..=0x2F`:
    //    - `0x27..=0x2B`: SolCross (normal, angry, worried, full_synchro, dark)
    //      cloned from MegaMan `0x00..=0x04` with warm solar palette remap.
    //    - `0x2C..=0x2F`: Danger faces cloned from each variant's worried face
    //      (`0x02` MegaMan, `0x0D` Hub Gold, `0x24` Hub Colonel, `0x29` SolCross)
    //      with crimson/rose Danger palette remap.
    let get_mug = |idx: usize| -> (Tiles, Tiles, Palette) {
        let (face, pal) = graphics.hud.mugshots.get(idx).cloned().unwrap_or_default();
        let box_tiles = graphics.hud.mugshot_boxes.get(idx).cloned().unwrap_or_default();
        (face, box_tiles, pal)
    };
    let mut sol_entries: Vec<(Tiles, Tiles, Palette)> = Vec::with_capacity(DS_MUGSHOT_SLOTS);
    for base_idx in 0..5usize {
        let (face, box_tiles, pal) = get_mug(base_idx);
        sol_entries.push((face, box_tiles, remap_solar_palette(pal)));
    }
    // 0x2C: MegaMan Danger (from worried `0x02`)
    {
        let (face, box_tiles, pal) = get_mug(0x02);
        sol_entries.push((face, box_tiles, remap_danger_palette(pal)));
    }
    // 0x2D: BassCross Gold Danger (from worried `0x0D`)
    {
        let (face, box_tiles, pal) = get_mug(0x0d);
        sol_entries.push((face, box_tiles, remap_danger_palette(pal)));
    }
    // 0x2E: BassCross Silver / Colonel Danger (from worried `0x24`)
    {
        let (face, box_tiles, pal) = get_mug(0x24);
        sol_entries.push((face, box_tiles, remap_danger_palette(pal)));
    }
    // 0x2F: SolCross Danger (from synthesized SolCross worried `0x29` = `sol_entries[2]`)
    {
        let (face, box_tiles, pal) = sol_entries[2].clone();
        sol_entries.push((face, box_tiles, remap_danger_palette(pal)));
    }
    let arr: [(Tiles, Tiles, Palette); DS_MUGSHOT_SLOTS] = sol_entries.try_into().unwrap();
    ensure_mugshot_slots(graphics, arr);

    // 5. Team Navi Danger faces stay empty in GBA fallback (saito-render draws a
    //    red-tinted pulsing Danger palette over the normal Team Navi face).
    graphics.hud.navi_danger_mugshots.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_EXE5DS_ROM: &str = "/home/harp/projects/nds/roms/exe5ds.nds";

    fn sample_gba_exe5_bundle() -> Bundle {
        let mut b = Bundle::default();
        // MegaMan `00-00` with blue armor palette
        let blue_pal: Palette = [
            0x0000,
            saito_assets::bgr555([0x20, 0x50, 0xF0]),
            saito_assets::bgr555([0x40, 0x90, 0xFF]),
            saito_assets::bgr555([0xF8, 0xD0, 0x98]),
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        b.sprites.push(SpriteSheet {
            category: 0x00,
            index: 0x00,
            tilesets: vec![Tiles { pixels: vec![1; 4 * Tiles::TILE] }],
            palette_sets: vec![vec![blue_pal]],
            part_lists: vec![vec![SpritePart { tile: 0, x: -8, y: -16, width: 16, height: 16, ..Default::default() }]],
            animations: vec![vec![SpriteFrame { duration: 4, flags: FRAME_LAST, ..Default::default() }]],
        });
        // Buster arm `0c-03` with 14 animations (`0x00..=0x0D`)
        let arm_anims = (0..14)
            .map(|_| vec![SpriteFrame { tileset: 0, palette_set: 0, parts: 0, duration: 2, flags: FRAME_LAST }])
            .collect();
        b.sprites.push(SpriteSheet {
            category: BUSTER_ARM_SPRITE.0,
            index: BUSTER_ARM_SPRITE.1,
            tilesets: vec![Tiles { pixels: vec![2; Tiles::TILE] }],
            palette_sets: vec![vec![blue_pal]],
            part_lists: vec![vec![SpritePart { tile: 0, x: 0, y: 0, width: 8, height: 8, ..Default::default() }]],
            animations: arm_anims,
        });
        // 39 GBA mugshots (`0x00..=0x26`) + boxes
        for i in 0..GBA_EXE5_MUGSHOT_COUNT {
            let mut pal = blue_pal;
            pal[4] = saito_assets::bgr555([(i as u8 * 6).min(248), 0x60, 0xD0]);
            b.hud.mugshots.push((Tiles { pixels: vec![(i % 15 + 1) as u8; 8 * Tiles::TILE] }, pal));
            b.hud.mugshot_boxes.push(Tiles { pixels: vec![(i % 15 + 1) as u8; 4 * Tiles::TILE] });
        }
        b
    }

    #[test]
    fn header_validation_checks_code_and_table_bounds() {
        let mut rom = vec![0u8; 0x1000];
        // Missing game code -> rejected
        assert!(NitroFs::parse(&rom).is_none());
        // Wrong game code -> rejected
        rom[0x0C..0x10].copy_from_slice(b"BRBE");
        rom[0x40..0x44].copy_from_slice(&0x200u32.to_le_bytes());
        rom[0x44..0x48].copy_from_slice(&0x100u32.to_le_bytes());
        rom[0x48..0x4C].copy_from_slice(&0x300u32.to_le_bytes());
        rom[0x4C..0x50].copy_from_slice(&0x100u32.to_le_bytes());
        assert!(NitroFs::parse(&rom).is_none());
        // Valid game codes A5TJ / A5TE / A5TP -> accepted
        for code in VALID_GAME_CODES {
            rom[0x0C..0x10].copy_from_slice(code);
            assert!(NitroFs::parse(&rom).is_some(), "expected {code:?} to parse");
        }
        // Out-of-bounds FAT -> rejected
        rom[0x4C..0x50].copy_from_slice(&0x2000u32.to_le_bytes());
        assert!(NitroFs::parse(&rom).is_none());
    }

    #[test]
    fn gba_fallback_synthesizes_all_solcross_and_danger_assets() {
        let mut bundle = sample_gba_exe5_bundle();
        let warnings = apply_exe5_ds_assets_from_path(&mut bundle, None);
        assert!(warnings.is_empty(), "silent fallback should not warn: {warnings:?}");

        // Sprites: `solcross-megaman` (`00-0d`), `sol-sun` (`10-00`), `buster-arm` (`0c-03`) anim `0x0E`
        let sol = bundle.sprite(0x00, 0x0d).expect("solcross-megaman synthesized");
        let base = bundle.sprite(0x00, 0x00).unwrap();
        assert_eq!(sol.animations.len(), base.animations.len());
        assert_ne!(sol.palette_sets[0][0], base.palette_sets[0][0], "SolCross palette should differ from base MegaMan");

        let sun = bundle.sprite(0x10, 0x00).expect("sol-sun synthesized");
        assert_eq!(sun.animations.len(), 1);
        assert!(!sun.animations[0].is_empty());
        assert!(sun.tilesets[0].pixels.iter().any(|&p| p != 0));

        let arm = bundle.sprite(0x0c, 0x03).expect("buster-arm present");
        assert_eq!(arm.animations.len(), 15, "buster-arm must have anim 0x0E");
        assert!(!arm.animations[SOLCROSS_BUSTER_ARM_ANIM].is_empty());
        let sol_arm_ps = arm.animations[SOLCROSS_BUSTER_ARM_ANIM][0].palette_set as usize;
        assert!(arm.palette_sets[sol_arm_ps].len() > 0x22, "buster-arm must have SolCross palettes 0x21/0x22");
        assert_ne!(arm.palette_sets[sol_arm_ps][0x21], arm.palette_sets[0][0]);
        assert_ne!(arm.palette_sets[sol_arm_ps][0x22], arm.palette_sets[sol_arm_ps][0x21]);

        // Mugshots `0x27..=0x2F` and boxes
        assert_eq!(bundle.hud.mugshots.len(), 0x30);
        assert_eq!(bundle.hud.mugshot_boxes.len(), 0x30);
        for slot in 0x27..=0x2Fusize {
            let (ref face, pal) = bundle.hud.mugshots[slot];
            assert_eq!(face.len(), 8, "slot {slot:#x} face tiles");
            assert_eq!(bundle.hud.mugshot_boxes[slot].len(), 4, "slot {slot:#x} box tiles");
            assert!(pal.iter().any(|&c| c != 0), "slot {slot:#x} palette non-empty");
        }
        assert_ne!(bundle.hud.mugshots[0x27].1, bundle.hud.mugshots[0x00].1, "SolCross face remapped");
        assert_ne!(bundle.hud.mugshots[0x2c].1, bundle.hud.mugshots[0x02].1, "Danger face remapped");
        assert!(bundle.hud.navi_danger_mugshots.is_empty(), "GBA fallback leaves navi_danger_mugshots empty");
    }

    #[test]
    fn extracts_real_exe5ds_rom_assets_when_available() {
        let rom_path = Path::new(LOCAL_EXE5DS_ROM);
        if !rom_path.exists() {
            return;
        }
        assert!(is_exe5ds_rom(rom_path), "expected {LOCAL_EXE5DS_ROM} to validate as EXE5DS");

        let mut bundle = sample_gba_exe5_bundle();
        let warnings = apply_exe5_ds_assets_from_path(&mut bundle, Some(rom_path));
        assert!(warnings.is_empty(), "real ROM extraction should produce no warnings: {warnings:?}");

        // pl0d.bin -> 00-0d (17 anims)
        let sol = bundle.sprite(0x00, 0x0d).expect("DS solcross-megaman");
        assert_eq!(sol.animations.len(), 17);
        assert!(!sol.tilesets.is_empty());

        // pshl03.bin -> 0c-03 (15 anims, anim 0x0E is SolCross)
        let arm = bundle.sprite(0x0c, 0x03).expect("DS buster-arm");
        assert_eq!(arm.animations.len(), 15);
        assert!(!arm.animations[SOLCROSS_BUSTER_ARM_ANIM].is_empty());

        // shl00.bin -> 10-00 (1 anim, 30 frames)
        let sun = bundle.sprite(0x10, 0x00).expect("DS sol-sun");
        assert_eq!(sun.animations.len(), 1);
        assert_eq!(sun.animations[0].len(), 30);

        // Mugshots 0x27..=0x2F (including Colonel BassCross Danger 0x2e using non-zero pal 23!)
        assert_eq!(bundle.hud.mugshots.len(), 0x30);
        assert_eq!(bundle.hud.mugshot_boxes.len(), 0x30);
        for slot in 0x27..=0x2Fusize {
            let (ref face, pal) = bundle.hud.mugshots[slot];
            assert_eq!(face.len(), 8, "DS mugshot {slot:#x} should be 4x2 tiles");
            assert!(face.pixels.iter().any(|&p| p != 0), "DS mugshot {slot:#x} should have pixels");
            assert_eq!(bundle.hud.mugshot_boxes[slot].len(), 4, "DS mugshot box {slot:#x} should be 2x2 tiles");
            assert!(pal.iter().any(|&c| c != 0), "DS mugshot {slot:#x} palette must not be all black");
        }

        // All 12 Team Navi Danger faces from kokoro_navi.bin
        assert_eq!(bundle.hud.navi_danger_mugshots.len(), TEAM_NAVI_COUNT);
        for (i, entry) in bundle.hud.navi_danger_mugshots.iter().enumerate() {
            let d = entry.as_ref().unwrap_or_else(|| panic!("missing DS danger face for team navi {i}"));
            assert_eq!(d.tiles.len(), 8);
            assert_eq!(d.box_tiles.len(), 4);
            assert!(d.tiles.pixels.iter().any(|&p| p != 0));
            assert!(d.palette.iter().any(|&c| c != 0));
        }
    }

    #[test]
    fn rom_path_setter_and_getter_roundtrip() {
        let prev = exe5ds_rom_path();
        let test_path = PathBuf::from("/tmp/nonexistent-exe5ds-test.nds");
        set_exe5ds_rom_path(Some(test_path.clone()));
        assert_eq!(exe5ds_rom_path(), Some(test_path));
        set_exe5ds_rom_path(None);
        // Restore previous state if any.
        if let Some(p) = prev {
            if std::env::var_os("SAITO_EXE5DS_ROM").as_deref() != Some(p.as_os_str()) {
                set_exe5ds_rom_path(Some(p));
            }
        }
    }
}
