//! On-demand index into Battle Network Legacy Collection Vol. 2's `launcher/data/vol2.dat`
//! for official high-resolution chip artwork (512×551 PNGs, cropped to 512×432 at decode time).
//!
//! `vol2.dat` begins with a little-endian `u32` entry count followed by `count` 12-byte records
//! `(crc32_hash: u32, offset: u32, size: u32)`. Each chip artwork entry's filename ends in
//! `"{rom_chip_id:03}.png"`; because IEEE CRC32 is affine-linear over XOR, any chip ID's hash is
//! derived directly from the game's `000.png` base hash:
//! `base ^ crc32(b"000.png") ^ crc32(format!("{id:03}.png").as_bytes())`.

use saito_assets::{HdChipArtIndex, HdCrossArt, HdCrossSheet};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Dimensions (pixels) of each EXE6 Legacy Collection HD Cross-selection bar inside `cross_*.pak`.
pub const HD_CROSS_BAR_WIDTH: u32 = 288;
pub const HD_CROSS_BAR_HEIGHT: u32 = 64;

const CROSS_PAK_HEADER_LEN: usize = 0x20010;
const CROSS_PAK_SHEET_SIDE: usize = 640;
const CROSS_PAK_MIN_LEN: usize = CROSS_PAK_HEADER_LEN + CROSS_PAK_SHEET_SIDE * CROSS_PAK_SHEET_SIDE * 4;

const CROSS_VERSIONS: [(&str, &str, &str); 2] = [
    ("gregar", "exe6.dat", "exe6"),
    ("falzar", "exe6f.dat", "exe6f"),
];
const CROSS_LANGS: [(&str, &str); 3] = [
    ("en", "en"),
    ("ja", "jp"),
    ("zh-Hans", "cs"),
];

/// Parse an EXE6 Legacy Collection `cross_*.pak` (640×640 BGRA8 at offset `0x20010`) into 15
/// `288×64` RGBA8 bars (`state * 5 + number`: 0 = selected (color), 1 = unselected (grey), 2 = marked/used (cyan)).
pub fn parse_cross_pak(pak: &[u8]) -> Option<HdCrossSheet> {
    if pak.len() < CROSS_PAK_MIN_LEN {
        return None;
    }
    let px = &pak[CROSS_PAK_HEADER_LEN..CROSS_PAK_MIN_LEN];
    let bw = HD_CROSS_BAR_WIDTH as usize;
    let bh = HD_CROSS_BAR_HEIGHT as usize;
    let mut bars: Vec<Vec<u8>> = Vec::with_capacity(15);
    for state in 0..3usize {
        for num in 0..5usize {
            let (ox, oy) = match state {
                0 => (0, 70 * num),
                1 => (330, 70 * num),
                _ if num < 4 => (0, 350 + 70 * num),
                _ => (330, 350),
            };
            let mut rgba = Vec::with_capacity(bw * bh * 4);
            for y in 0..bh {
                let row = (oy + y) * CROSS_PAK_SHEET_SIDE + ox;
                let slice = &px[row * 4..(row + bw) * 4];
                for bgra in slice.chunks_exact(4) {
                    rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
                }
            }
            bars.push(rgba);
        }
    }
    let arr: [Vec<u8>; 15] = bars.try_into().ok()?;
    Some(HdCrossSheet { bars: std::sync::Arc::new(arr) })
}

fn cross_copy_name(version: &str, lang: &str) -> String {
    format!("cross_{version}_{lang}.zip")
}

/// Stow compacted single-entry zips of EXE6's `cross_*.pak` sheets alongside ROM copies on mobile/web
/// (~150 KB each, <1 MB total) using raw zip entry copy with no recompression.
pub(crate) fn stow_cross_sheets(data_dir: &Path, dst_dir: &Path) {
    for (version, archive_name, prefix) in CROSS_VERSIONS {
        let archive_path = data_dir.join(archive_name);
        let Ok(file) = saito_files::File::open(&archive_path) else { continue };
        let Ok(mut zip) = zip::ZipArchive::new(file) else { continue };
        for (lang, pak_lang) in CROSS_LANGS {
            let entry_name = format!("{prefix}/data/ui/cross_{pak_lang}.pak");
            let Some(idx) = zip.index_for_name(&entry_name) else { continue };
            let Ok(entry) = zip.by_index_raw(idx) else { continue };
            let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            if out.raw_copy_file_rename(entry, "cross.pak").is_ok()
                && let Ok(cursor) = out.finish()
            {
                let _ = saito_files::stow(&dst_dir.join(cross_copy_name(version, lang)), cursor.into_inner());
            }
        }
    }
}

/// Extract EXE6 Legacy Collection HD Cross-selection bars from `roms` (either `exe6.dat`/`exe6f.dat`
/// in a Collection install, or stowed `cross_*.zip` copies alongside `RomSource::Copies`).
pub fn extract_cross_art(roms: &crate::RomSource) -> Option<HdCrossArt> {
    let mut sheets = BTreeMap::new();
    match roms {
        crate::RomSource::Collection(install) => {
            for (version, archive_name, prefix) in CROSS_VERSIONS {
                let archive_path = install.data.join(archive_name);
                let Ok(file) = saito_files::File::open(&archive_path) else { continue };
                let Ok(mut zip) = zip::ZipArchive::new(file) else { continue };
                for (lang, pak_lang) in CROSS_LANGS {
                    let entry_name = format!("{prefix}/data/ui/cross_{pak_lang}.pak");
                    let Ok(mut entry) = zip.by_name(&entry_name) else { continue };
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    if entry.read_to_end(&mut buf).is_ok()
                        && let Some(sheet) = parse_cross_pak(&buf)
                    {
                        sheets.insert((version.to_string(), lang.to_string()), sheet);
                    }
                }
            }
        }
        crate::RomSource::Copies(dir) => {
            for (version, _, _) in CROSS_VERSIONS {
                for (lang, _) in CROSS_LANGS {
                    let path = dir.join(cross_copy_name(version, lang));
                    if !saito_files::is_file(&path) {
                        continue;
                    }
                    if saito_files::fetch(std::slice::from_ref(&path)).is_err() {
                        continue;
                    }
                    let Ok(file) = saito_files::File::open(&path) else { continue };
                    let Ok(mut zip) = zip::ZipArchive::new(file) else { continue };
                    let Ok(mut entry) = zip.by_index(0) else { continue };
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    if entry.read_to_end(&mut buf).is_ok()
                        && let Some(sheet) = parse_cross_pak(&buf)
                    {
                        sheets.insert((version.to_string(), lang.to_string()), sheet);
                    }
                }
            }
        }
        crate::RomSource::Files(_) => return None,
    }
    if sheets.is_empty() {
        return None;
    }
    Some(HdCrossArt { sheets, lang: saito_assets::BASE_LANGUAGE.to_string() })
}

/// File name of the Legacy Collection Vol. 2 art archive (both the original `launcher/data/vol2.dat`
/// and the compacted chip-only copy stowed on phones/browsers).
pub const VOL2_FILE: &str = "vol2.dat";

/// Games in Battle Network Legacy Collection Vol. 2 with HD chip artwork.
const VOL2_GAMES: [&str; 3] = ["exe4", "exe5", "exe6"];
/// Upper bound (inclusive) on ROM chip IDs to scan when compacting `vol2.dat`.
const MAX_ROM_CHIP_ID: u16 = 512;

/// Known `000.png` ("NO DATA") CRC32 base hashes for Vol. 2 games inside `launcher/data/vol2.dat`.
fn base_hash(game: &str) -> Option<u32> {
    match game {
        "exe4" => Some(0x95e9_bce6),
        "exe5" => Some(0x8292_a8a5),
        "exe6" => Some(0xbb1f_9460),
        _ => None,
    }
}

/// Set of all chip artwork hashes across EXE4, EXE5, and EXE6 (IDs `1..=512`).
fn all_chip_hashes() -> HashSet<u32> {
    let mut set = HashSet::with_capacity(VOL2_GAMES.len() * MAX_ROM_CHIP_ID as usize);
    for game in VOL2_GAMES {
        for id in 1..=MAX_ROM_CHIP_ID {
            if let Some(h) = chip_hash(game, id) {
                set.insert(h);
            }
        }
    }
    set
}

/// Check whether `path` is a readable `vol2.dat` archive containing at least one Vol. 2 chip entry.
pub fn is_valid_vol2(path: &Path) -> bool {
    let Ok(file) = File::open(path) else { return false };
    let Ok(table) = read_table(file) else { return false };
    let wanted = all_chip_hashes();
    table.keys().any(|h| wanted.contains(h))
}

/// Compact a full Legacy Collection `vol2.dat` (~1.2 GB) into a chip-art-only `vol2.dat` (~163 MB)
/// at `dst`, preserving the exact `vol2.dat` binary format so [`index_from_vol2`] reads both
/// identically. Streams entry payloads without loading the archive into RAM.
/// Returns the number of chip entries written.
pub fn compact_vol2(src: &Path, dst: &Path) -> std::io::Result<usize> {
    let mut reader = File::open(src)?;
    let table = read_table(&mut reader)?;
    let wanted = all_chip_hashes();
    let mut keep: Vec<(u32, u64, u32)> = table
        .into_iter()
        .filter(|(h, (_, sz))| wanted.contains(h) && *sz > 0)
        .map(|(h, (off, sz))| (h, off, sz))
        .collect();
    // Sort by source offset so reads are sequential.
    keep.sort_by_key(|&(_, off, _)| off);
    if keep.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "vol2.dat contains no Vol. 2 chip artwork entries"));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dst.with_extension("dat.tmp");
    let header_len = 4u64 + (keep.len() as u64) * 12;
    let write_res = (|| -> std::io::Result<()> {
        let mut writer = BufWriter::new(File::create(&tmp)?);
        writer.write_all(&(keep.len() as u32).to_le_bytes())?;
        let mut cur_off = header_len;
        for &(hash, _, sz) in &keep {
            let off32 = u32::try_from(cur_off)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "compacted vol2.dat exceeds 4 GiB"))?;
            writer.write_all(&hash.to_le_bytes())?;
            writer.write_all(&off32.to_le_bytes())?;
            writer.write_all(&sz.to_le_bytes())?;
            cur_off += u64::from(sz);
        }
        let mut buf = vec![0u8; 64 * 1024];
        for &(_, off, sz) in &keep {
            reader.seek(SeekFrom::Start(off))?;
            let mut left = sz as usize;
            while left > 0 {
                let n = left.min(buf.len());
                reader.read_exact(&mut buf[..n])?;
                writer.write_all(&buf[..n])?;
                left -= n;
            }
        }
        writer.flush()?;
        Ok(())
    })();
    if let Err(e) = write_res {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dst)?;
    Ok(keep.len())
}

/// Standard IEEE 802.3 CRC32 (matches `zlib.crc32`).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Compute the `vol2.dat` entry hash for `game`'s ROM chip `rom_id`.
pub(crate) fn chip_hash(game: &str, rom_id: u16) -> Option<u32> {
    let base = base_hash(game)?;
    let zero = crc32(b"000.png");
    let suffix = format!("{rom_id:03}.png");
    Some(base ^ zero ^ crc32(suffix.as_bytes()))
}

/// Parse a `vol2.dat` table from `reader` into `hash -> (offset, size)`.
fn read_table(mut reader: impl Read) -> std::io::Result<HashMap<u32, (u64, u32)>> {
    let mut count_buf = [0u8; 4];
    reader.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;
    // Sanity bound: vol2.dat has ~4729 entries; cap at 200k to avoid bogus allocations.
    if count == 0 || count > 200_000 {
        return Ok(HashMap::new());
    }
    let mut raw = vec![0u8; count * 12];
    reader.read_exact(&mut raw)?;
    let mut map = HashMap::with_capacity(count);
    for chunk in raw.chunks_exact(12) {
        let hash = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let off = u32::from_le_bytes(chunk[4..8].try_into().unwrap()) as u64;
        let size = u32::from_le_bytes(chunk[8..12].try_into().unwrap());
        map.insert(hash, (off, size));
    }
    Ok(map)
}

/// Build an on-demand [`HdChipArtIndex`] for `game` from `vol2_dat`, mapping local chip keys to
/// `(offset, size)` in `vol2_dat`. Returns `None` if the game has no Vol. 2 mapping, the archive
/// cannot be read, or no chip entries match.
pub fn index_from_vol2(vol2_dat: &Path, game: &str, chips: &BTreeMap<u16, String>) -> Option<HdChipArtIndex> {
    base_hash(game)?;
    let file = File::open(vol2_dat).ok()?;
    let table = read_table(file).ok()?;
    let mut entries = BTreeMap::new();
    for (&rom_id, key) in chips {
        if rom_id == 0 {
            continue;
        }
        if let Some(hash) = chip_hash(game, rom_id)
            && let Some(&loc) = table.get(&hash)
        {
            let local = saito_content_api::keys::local(key).to_string();
            entries.insert(local, loc);
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(HdChipArtIndex { archive: vol2_dat.to_path_buf(), entries, elements: BTreeMap::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_crc32_matches_known_vol2_hashes() {
        // Verified against Steam MMBN Legacy Collection Vol. 2 launcher/data/vol2.dat:
        // EXE4: 000 = 0x95e9bce6, 001 (Cannon) = 0xa8899556, 002 (HiCannon) = 0xef29ef86, 280 (VidemnDS) = 0xee252fa2.
        assert_eq!(chip_hash("exe4", 0), Some(0x95e9_bce6));
        assert_eq!(chip_hash("exe4", 1), Some(0xa889_9556));
        assert_eq!(chip_hash("exe4", 2), Some(0xef29_ef86));
        assert_eq!(chip_hash("exe4", 280), Some(0xee25_2fa2));
        // EXE5: 000 = 0x8292a8a5, 001 (Cannon) = 0xbff28115.
        assert_eq!(chip_hash("exe5", 0), Some(0x8292_a8a5));
        assert_eq!(chip_hash("exe5", 1), Some(0xbff2_8115));
        // EXE6: 000 = 0xbb1f9460, 313 (Falzar) = 0x060b2788.
        assert_eq!(chip_hash("exe6", 0), Some(0xbb1f_9460));
        assert_eq!(chip_hash("exe6", 313), Some(0x060b_2788));
        assert_eq!(chip_hash("exe1", 1), None);
    }

    #[test]
    fn reads_vol2_table_and_skips_missing_or_zero_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vol2.dat");
        let h1 = chip_hash("exe6", 1).unwrap();
        let h0 = chip_hash("exe6", 0).unwrap();
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&h0.to_le_bytes());
        buf.extend_from_slice(&100u32.to_le_bytes());
        buf.extend_from_slice(&50u32.to_le_bytes());
        buf.extend_from_slice(&h1.to_le_bytes());
        buf.extend_from_slice(&200u32.to_le_bytes());
        buf.extend_from_slice(&1234u32.to_le_bytes());
        std::fs::write(&path, buf).unwrap();

        let mut chips = BTreeMap::new();
        chips.insert(0, "exe6:nodata".to_string());
        chips.insert(1, "exe6:cannon".to_string());
        chips.insert(999, "exe6:missing".to_string());
        let idx = index_from_vol2(&path, "exe6", &chips).expect("should find cannon");
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries.get("cannon"), Some(&(200, 1234)));
    }

    #[test]
    fn compact_vol2_keeps_only_chip_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("full.dat");
        let dst = dir.path().join("compact.dat");
        let h_chip = chip_hash("exe6", 1).unwrap();
        let h_other = 0xDEAD_BEEFu32;
        let payload_other = b"1920x1080 wallpaper bytes";
        let payload_chip = b"fake-chip-png-bytes";
        let header_len = 4 + 2 * 12;
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&h_other.to_le_bytes());
        buf.extend_from_slice(&(header_len as u32).to_le_bytes());
        buf.extend_from_slice(&(payload_other.len() as u32).to_le_bytes());
        buf.extend_from_slice(&h_chip.to_le_bytes());
        buf.extend_from_slice(&((header_len + payload_other.len()) as u32).to_le_bytes());
        buf.extend_from_slice(&(payload_chip.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload_other);
        buf.extend_from_slice(payload_chip);
        std::fs::write(&src, buf).unwrap();

        assert!(is_valid_vol2(&src));
        let count = compact_vol2(&src, &dst).unwrap();
        assert_eq!(count, 1);
        assert!(is_valid_vol2(&dst));

        let mut chips = BTreeMap::new();
        chips.insert(1, "exe6:cannon".to_string());
        let idx = index_from_vol2(&dst, "exe6", &chips).unwrap();
        let &(off, sz) = idx.entries.get("cannon").unwrap();
        let out_bytes = std::fs::read(&dst).unwrap();
        assert_eq!(&out_bytes[off as usize..off as usize + sz as usize], payload_chip);
    }

    #[test]
    fn parse_cross_pak_extracts_15_bars_and_swaps_bgra_to_rgba() {
        let mut pak = vec![0u8; CROSS_PAK_MIN_LEN];
        let set_bgra = |pak: &mut [u8], x: usize, y: usize, bgra: [u8; 4]| {
            let off = CROSS_PAK_HEADER_LEN + (y * CROSS_PAK_SHEET_SIDE + x) * 4;
            pak[off..off + 4].copy_from_slice(&bgra);
        };
        // State 0 (selected), Cross 0 at (0, 0)
        set_bgra(&mut pak, 0, 0, [10, 20, 30, 255]);
        // State 1 (marked), Cross 2 at (330, 140)
        set_bgra(&mut pak, 330, 140, [40, 50, 60, 255]);
        // State 2 (unselected), Cross 4 at (330, 350)
        set_bgra(&mut pak, 330, 350, [70, 80, 90, 255]);

        let sheet = parse_cross_pak(&pak).expect("parses 640x640 cross pak");
        assert_eq!(sheet.bars[0].len(), (HD_CROSS_BAR_WIDTH * HD_CROSS_BAR_HEIGHT * 4) as usize);
        // BGRA [10,20,30,255] -> RGBA [30,20,10,255]
        assert_eq!(&sheet.bars[0][..4], &[30, 20, 10, 255]);
        assert_eq!(&sheet.bars[5 + 2][..4], &[60, 50, 40, 255]);
        assert_eq!(&sheet.bars[10 + 4][..4], &[90, 80, 70, 255]);
    }
}
