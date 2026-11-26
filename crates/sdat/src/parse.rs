//! Binary parser for Nintendo DS `SDAT` archives (`SSEQ`, `SBNK`, `SWAR`, and
//! `SWAV` PCM8 / PCM16 / IMA-ADPCM waveforms).

use crate::bank::{Instrument, InstrumentBank, NoteDef, NoteKind, Song, SoundBank, Wave, WaveArchive};
use crate::tables::{ADPCM_INDEX_TABLE, ADPCM_STEP_TABLE};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(msg: impl Into<String>) -> Error {
    Error(msg.into())
}

fn rd16(buf: &[u8], off: usize) -> Result<u16, Error> {
    let b = buf.get(off..off + 2).ok_or_else(|| err(format!("truncated u16 at {off:#x}")))?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn rd16_i(buf: &[u8], off: usize) -> Result<i16, Error> {
    Ok(rd16(buf, off)? as i16)
}

fn rd32(buf: &[u8], off: usize) -> Result<u32, Error> {
    let b = buf.get(off..off + 4).ok_or_else(|| err(format!("truncated u32 at {off:#x}")))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Extract only the requested sequence IDs (`seq_ids`) along with every
/// instrument bank (`SBNK`) and wave archive (`SWAR`) they reference.
///
/// Pass an empty slice to extract every sequence present in the SDAT.
pub fn extract_sdat(sdat: &[u8], seq_ids: &[u16]) -> Result<SoundBank, Error> {
    if sdat.len() < 0x30 || &sdat[0..4] != b"SDAT" {
        return Err(err("not an SDAT archive"));
    }
    let info_off = rd32(sdat, 0x18)? as usize;
    let info_len = rd32(sdat, 0x1C)? as usize;
    let fat_off = rd32(sdat, 0x20)? as usize;
    let fat_len = rd32(sdat, 0x24)? as usize;
    if info_off.checked_add(info_len).is_none_or(|e| e > sdat.len())
        || fat_off.checked_add(fat_len).is_none_or(|e| e > sdat.len())
    {
        return Err(err("SDAT INFO/FAT bounds exceed file size"));
    }
    if sdat.get(info_off..info_off + 4) != Some(b"INFO") || sdat.get(fat_off..fat_off + 4) != Some(b"FAT ") {
        return Err(err("SDAT missing INFO or FAT block"));
    }

    let fat_count = rd32(sdat, fat_off + 8)? as usize;
    let get_file = |file_id: u16| -> Result<&[u8], Error> {
        let idx = file_id as usize;
        if idx >= fat_count {
            return Err(err(format!("SDAT file id {file_id} out of range ({fat_count})")));
        }
        let rec = fat_off + 12 + idx * 16;
        let foff = rd32(sdat, rec)? as usize;
        let fsz = rd32(sdat, rec + 4)? as usize;
        sdat.get(foff..foff.checked_add(fsz).ok_or_else(|| err("file range overflow"))?)
            .ok_or_else(|| err(format!("SDAT file {file_id} out of bounds")))
    };

    let info_list = |table_idx: usize| -> Result<Option<usize>, Error> {
        let rel = rd32(sdat, info_off + 8 + table_idx * 4)? as usize;
        Ok((rel != 0).then_some(info_off + rel))
    };

    let seq_list = info_list(0)?.ok_or_else(|| err("SDAT has no SEQ table"))?;
    let bank_list = info_list(2)?.ok_or_else(|| err("SDAT has no BANK table"))?;
    let war_list = info_list(3)?.ok_or_else(|| err("SDAT has no WAVEARC table"))?;

    let seq_count = rd32(sdat, seq_list)? as usize;
    let bank_count = rd32(sdat, bank_list)? as usize;
    let war_count = rd32(sdat, war_list)? as usize;

    let wanted_seqs: Vec<u16> = if seq_ids.is_empty() {
        (0..seq_count as u16).collect()
    } else {
        seq_ids.to_vec()
    };

    let mut out = SoundBank::default();
    let max_seq = wanted_seqs.iter().copied().max().map_or(0, |m| m as usize + 1);
    out.songs.resize(max_seq, None);

    let mut needed_banks = BTreeSet::new();
    for seq_id in wanted_seqs {
        let i = seq_id as usize;
        if i >= seq_count {
            continue;
        }
        let entry_rel = rd32(sdat, seq_list + 4 + i * 4)? as usize;
        if entry_rel == 0 {
            continue;
        }
        let entry = info_off + entry_rel;
        let file_id = rd16(sdat, entry)?;
        let bank_id = rd16(sdat, entry + 4)?;
        let volume = *sdat.get(entry + 6).ok_or_else(|| err("truncated SEQ info"))?;
        let channel_priority = *sdat.get(entry + 7).ok_or_else(|| err("truncated SEQ info"))?;
        let player_priority = *sdat.get(entry + 8).ok_or_else(|| err("truncated SEQ info"))?;
        let player = *sdat.get(entry + 9).ok_or_else(|| err("truncated SEQ info"))?;

        let sseq_bytes = get_file(file_id)?;
        let data = parse_sseq(sseq_bytes)?;
        out.songs[i] = Some(Song {
            data,
            bank: bank_id,
            volume,
            channel_priority,
            player_priority,
            player,
        });
        needed_banks.insert(bank_id);
    }

    let max_bank = needed_banks.iter().copied().max().map_or(0, |m| m as usize + 1);
    out.banks.resize(max_bank, None);

    let mut needed_wars = BTreeSet::new();
    for bank_id in needed_banks {
        let i = bank_id as usize;
        if i >= bank_count {
            return Err(err(format!("SBNK id {bank_id} out of range ({bank_count})")));
        }
        let entry_rel = rd32(sdat, bank_list + 4 + i * 4)? as usize;
        if entry_rel == 0 {
            continue;
        }
        let entry = info_off + entry_rel;
        let file_id = rd16(sdat, entry)?;
        let mut wave_arcs = [None; 4];
        for slot in 0..4 {
            let wid = rd16(sdat, entry + 4 + slot * 2)?;
            if wid != 0xFFFF {
                wave_arcs[slot] = Some(wid);
                needed_wars.insert(wid);
            }
        }
        let sbnk_bytes = get_file(file_id)?;
        let instruments = parse_sbnk(sbnk_bytes)?;
        out.banks[i] = Some(InstrumentBank { wave_arcs, instruments });
    }

    let max_war = needed_wars.iter().copied().max().map_or(0, |m| m as usize + 1);
    out.wave_arcs.resize(max_war, None);

    for war_id in needed_wars {
        let i = war_id as usize;
        if i >= war_count {
            return Err(err(format!("SWAR id {war_id} out of range ({war_count})")));
        }
        let entry_rel = rd32(sdat, war_list + 4 + i * 4)? as usize;
        if entry_rel == 0 {
            continue;
        }
        let entry = info_off + entry_rel;
        let file_id = rd16(sdat, entry)?;
        let swar_bytes = get_file(file_id)?;
        let waves = parse_swar(swar_bytes)?;
        out.wave_arcs[i] = Some(WaveArchive { waves });
    }

    Ok(out)
}

fn parse_sseq(buf: &[u8]) -> Result<Vec<u8>, Error> {
    if buf.len() < 0x1C || &buf[0..4] != b"SSEQ" || &buf[0x10..0x14] != b"DATA" {
        return Err(err("invalid SSEQ file"));
    }
    let data_off = rd32(buf, 0x18)? as usize;
    let slice = buf.get(data_off..).ok_or_else(|| err("SSEQ data offset out of bounds"))?;
    Ok(slice.to_vec())
}

fn parse_note_def(buf: &[u8], ftype: u8, off: usize) -> Result<NoteDef, Error> {
    if buf.len() < off + 10 {
        return Err(err(format!("truncated SBNK note def at {off:#x}")));
    }
    let swav_id = rd16(buf, off)?;
    let swar_id = rd16(buf, off + 2)?;
    let base_note = buf[off + 4];
    let attack = buf[off + 5];
    let decay = buf[off + 6];
    let sustain = buf[off + 7];
    let release = buf[off + 8];
    let pan = buf[off + 9];
    let kind = match ftype {
        1 | 4 => NoteKind::Pcm { swav_id, swar_slot: (swar_id & 3) as u8 },
        2 => NoteKind::Psg { duty: (swav_id & 7) as u8 },
        3 => NoteKind::Noise,
        _ => NoteKind::Empty,
    };
    Ok(NoteDef { kind, base_note, attack, decay, sustain, release, pan })
}

fn parse_sbnk(buf: &[u8]) -> Result<Vec<Instrument>, Error> {
    if buf.len() < 0x3C || &buf[0..4] != b"SBNK" || &buf[0x10..0x14] != b"DATA" {
        return Err(err("invalid SBNK file"));
    }
    let count = rd32(buf, 0x38)? as usize;
    let mut instruments = Vec::with_capacity(count);
    for i in 0..count {
        let rec = rd32(buf, 0x3C + i * 4)?;
        let ftype = (rec & 0xFF) as u8;
        let foff = ((rec >> 8) & 0xFFFF) as usize;
        let inst = match ftype {
            0 => Instrument::Empty,
            1..=5 => Instrument::Single(parse_note_def(buf, ftype, foff)?),
            16 => {
                let min_note = *buf.get(foff).ok_or_else(|| err("truncated drumset"))?;
                let max_note = *buf.get(foff + 1).ok_or_else(|| err("truncated drumset"))?;
                let n = if max_note >= min_note { (max_note - min_note) as usize + 1 } else { 0 };
                let mut entries = Vec::with_capacity(n);
                for k in 0..n {
                    let eoff = foff + 2 + k * 12;
                    let sub_type = *buf.get(eoff).ok_or_else(|| err("truncated drumset entry"))?;
                    entries.push(parse_note_def(buf, sub_type, eoff + 2)?);
                }
                Instrument::DrumSet { min_note, max_note, entries }
            }
            17 => {
                let mut regions = Vec::new();
                for k in 0..8 {
                    let r = *buf.get(foff + k).ok_or_else(|| err("truncated keysplit"))?;
                    if r == 0 {
                        break;
                    }
                    regions.push(r);
                }
                let mut entries = Vec::with_capacity(regions.len());
                for k in 0..regions.len() {
                    let eoff = foff + 8 + k * 12;
                    let sub_type = *buf.get(eoff).ok_or_else(|| err("truncated keysplit entry"))?;
                    entries.push(parse_note_def(buf, sub_type, eoff + 2)?);
                }
                Instrument::KeySplit { regions, entries }
            }
            _ => Instrument::Empty,
        };
        instruments.push(inst);
    }
    Ok(instruments)
}

fn parse_swar(buf: &[u8]) -> Result<Vec<Wave>, Error> {
    if buf.len() < 0x3C || &buf[0..4] != b"SWAR" || &buf[0x10..0x14] != b"DATA" {
        return Err(err("invalid SWAR file"));
    }
    let count = rd32(buf, 0x38)? as usize;
    let mut offsets = Vec::with_capacity(count + 1);
    for i in 0..count {
        offsets.push(rd32(buf, 0x3C + i * 4)? as usize);
    }
    offsets.push(buf.len());

    let mut waves = Vec::with_capacity(count);
    for i in 0..count {
        let start = offsets[i];
        let end = offsets[i + 1].min(buf.len());
        if start + 12 > end {
            return Err(err(format!("truncated SWAV {i} in SWAR")));
        }
        waves.push(decode_swav(&buf[start..end])?);
    }
    Ok(waves)
}

fn decode_swav(buf: &[u8]) -> Result<Wave, Error> {
    let encoding = buf[0];
    let looped = buf[1] != 0;
    let sample_rate = rd16(buf, 2)?;
    let loop_start_words = rd16(buf, 6)? as usize;
    let non_loop_words = rd32(buf, 8)? as usize;
    let total_bytes = (loop_start_words.saturating_add(non_loop_words)).saturating_mul(4);
    let data = buf.get(12..).unwrap_or(&[]);
    let data = &data[..total_bytes.min(data.len())];

    let (samples, loop_start) = match encoding {
        0 => {
            // Signed 8-bit PCM
            let samples: Vec<i16> = data.iter().map(|&b| ((b as i8) as i16) << 8).collect();
            (samples, loop_start_words * 4)
        }
        1 => {
            // Signed 16-bit LE PCM
            let samples: Vec<i16> = data
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            (samples, loop_start_words * 2)
        }
        2 => {
            // IMA-ADPCM: 4-byte initial state header + nibbles (low nibble first)
            if data.len() < 4 {
                return Ok(Wave { samples: Vec::new(), sample_rate, looped: false, loop_start: 0, loop_end: 0 });
            }
            let mut pcm = rd16_i(data, 0)? as i32;
            let mut idx = (rd16_i(data, 2)? as i32).clamp(0, 88);
            let payload = &data[4..];
            let mut samples = Vec::with_capacity(payload.len() * 2);
            for &byte in payload {
                for shift in [0u8, 4u8] {
                    let nibble = (byte >> shift) & 0x0F;
                    let step = ADPCM_STEP_TABLE[idx as usize];
                    let mut diff = step >> 3;
                    if (nibble & 1) != 0 {
                        diff += step >> 2;
                    }
                    if (nibble & 2) != 0 {
                        diff += step >> 1;
                    }
                    if (nibble & 4) != 0 {
                        diff += step;
                    }
                    if (nibble & 8) != 0 {
                        pcm = (pcm - diff).max(-0x7FFF);
                    } else {
                        pcm = (pcm + diff).min(0x7FFF);
                    }
                    idx = (idx + ADPCM_INDEX_TABLE[(nibble & 7) as usize]).clamp(0, 88);
                    samples.push(pcm as i16);
                }
            }
            let loop_start = loop_start_words.saturating_sub(1) * 8;
            (samples, loop_start)
        }
        other => return Err(err(format!("unsupported SWAV encoding {other}"))),
    };

    let loop_end = samples.len();
    let loop_start = loop_start.min(loop_end);
    let looped = looped && loop_end > loop_start;
    Ok(Wave { samples, sample_rate, looped, loop_start, loop_end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Driver, SongId};
    use std::path::Path;
    use std::sync::Arc;

    const LOCAL_EXE5DS_ROM: &str = "/home/harp/projects/nds/roms/exe5ds.nds";

    fn extract_nitrofs_file<'a>(rom: &'a [u8], path: &str) -> Option<&'a [u8]> {
        let fnt_off = u32::from_le_bytes(rom[0x40..0x44].try_into().ok()?) as usize;
        let fat_off = u32::from_le_bytes(rom[0x48..0x4C].try_into().ok()?) as usize;
        let mut dir_id = 0xF000u16;
        let mut parts = path.split('/').peekable();
        while let Some(seg) = parts.next() {
            let want_dir = parts.peek().is_some();
            let main = fnt_off + (dir_id & 0x0FFF) as usize * 8;
            let sub_off = u32::from_le_bytes(rom[main..main + 4].try_into().ok()?) as usize;
            let mut fid = u16::from_le_bytes(rom[main + 4..main + 6].try_into().ok()?) as usize;
            let mut pos = fnt_off + sub_off;
            let mut found = false;
            loop {
                let hdr = rom[pos];
                pos += 1;
                if hdr == 0 {
                    break;
                }
                let is_dir = (hdr & 0x80) != 0;
                let nlen = (hdr & 0x7F) as usize;
                let name = std::str::from_utf8(&rom[pos..pos + nlen]).ok()?;
                pos += nlen;
                if is_dir {
                    let sub_dir = u16::from_le_bytes(rom[pos..pos + 2].try_into().ok()?);
                    pos += 2;
                    if want_dir && name.eq_ignore_ascii_case(seg) {
                        dir_id = sub_dir;
                        found = true;
                        break;
                    }
                } else {
                    if !want_dir && name.eq_ignore_ascii_case(seg) {
                        let start = u32::from_le_bytes(rom[fat_off + fid * 8..fat_off + fid * 8 + 4].try_into().ok()?) as usize;
                        let end = u32::from_le_bytes(rom[fat_off + fid * 8 + 4..fat_off + fid * 8 + 8].try_into().ok()?) as usize;
                        return rom.get(start..end);
                    }
                    fid += 1;
                }
            }
            if !found {
                return None;
            }
        }
        None
    }

    #[test]
    fn parses_and_renders_real_exe5ds_battle_tracks() {
        let path = Path::new(LOCAL_EXE5DS_ROM);
        if !path.exists() {
            return;
        }
        let rom = std::fs::read(path).expect("read exe5ds.nds");
        let sdat_bytes = extract_nitrofs_file(&rom, "data/rom/ds/sound_data.sdat").expect("sound_data.sdat");
        let seq_ids: [u16; 10] = [21, 22, 23, 14, 34, 37, 38, 39, 40, 41];
        let bank = Arc::new(extract_sdat(sdat_bytes, &seq_ids).expect("parse sdat"));
        assert!(bank.bank(82).is_some(), "BANK_BGM01 present");

        let mut driver = Driver::new(bank);
        let mut out = Vec::new();
        for id in seq_ids {
            driver.start(SongId(id));
            assert!(driver.is_playing(), "seq {id} started");
            out.clear();
            // Render 3 seconds (180 frames)
            for _ in 0..180 {
                driver.step_frame();
                driver.take_output(&mut out);
            }
            let peak = out.iter().flat_map(|s| *s).map(f32::abs).fold(0.0_f32, f32::max);
            let rms = (out.iter().map(|s| s[0] * s[0] + s[1] * s[1]).sum::<f32>() / (2.0 * out.len() as f32)).sqrt();
            eprintln!("SEQ[{id:2}] samples={} rms={rms:.4} peak={peak:.4}", out.len());
            assert!(rms > 0.02, "seq {id} too quiet: rms={rms}");
            assert!(peak <= 1.0, "seq {id} invalid peak: {peak}");
        }
    }
}

