//! EXE3 mGBA differential trace recordings: JSONL lines (`setup`, `exchange`,
//! per-frame snapshots) read from EXE3's RAM (`docs/design/exe3-map.md`),
//! decoding into engine terms ([`codec`](crate::codec)), [`RoundSetup`]
//! construction ([`setup_of`]), and frame-by-frame replay comparison
//! ([`run_round`], [`compare_frame`]).

use crate::codec::{self, HAND_BLOCK, HandBlock, NAVI_STATS, NaviStats, Panel};
use crate::{Compat, Style, Version, pool_of_type, pool_slots, type_of_pool};
use saito_battle::console::ConsoleSetup;
use saito_battle::content::ChipCode;
use saito_battle::custom::{BattleFolder, FolderChip, PlayerSetup};
use saito_battle::rules::Fact;
use saito_battle::{Battle, Content, NaviStats as EngineNaviStats, PlayerTick, RoundSetup, TickEvents};
use saito_content_api::{Pool, Registry, StageHandle, Value};
use serde::Deserialize;
use std::fmt;
use std::io::BufRead;
use std::sync::Arc;

/// A battle folder in EXE3's RAM: 30 chips × 4 bytes (`id: u16`, `code: u8`,
/// padding `u8`), `0x78` bytes total.
pub const FOLDER: usize = 0x78;
/// BattleState snapshot size in bytes.
pub const BATTLE_STATE: usize = 0xF0;
/// EXE3's 5×5 NaviCust grid size in bytes (`save 0x1D90`).
pub const NAVICUST_GRID: usize = crate::save::NAVICUST_SIZE * crate::save::NAVICUST_SIZE;

/// Link battle effects bitmask added to a stage's effects (same as EXE4–6).
const LINK_BATTLE_EFFECTS: u32 = 0x600;
/// Set-battle bit within `LINK_BATTLE_EFFECTS`.
const SET: u32 = saito_battle::setup::effects::SET;
/// Emulated link cable delay in frames.
const LINK_DELAY: u8 = 4;
/// Maximum RNG1 draws checked per frame for drift detection.
const RNG1_PICKS_A_FRAME: u32 = 8;

/// Static inputs of an EXE3 round from a trace's `setup` line.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub frame: u32,
    /// Must be `"exe3"`.
    pub game: String,
    /// Both sides' `0x40`-byte NaviStats blocks (`0x02037920 + 0x40 * side`), hex.
    pub navi_stats: [String; 2],
    /// Both sides' `0x78`-byte folders (30 × 4 bytes), hex.
    pub folders: [String; 2],
    /// Both sides' equipped Style bytes `(style_type << 3) | element` (`save 0x1881`).
    pub styles: [u8; 2],
    /// Both sides' Regular chip slot indices (`0..=29`, or `0xFF` none).
    pub regular: [u8; 2],
    /// Recording console's RNG1 (visual/folder) and shared RNG2 (battle).
    pub rng1: u32,
    pub rng2: u32,
    /// Optional per-side RNG1 seeds when captured on both consoles.
    #[serde(default)]
    pub rng1s: Option<[u32; 2]>,
    /// BattleState snapshot (`0xF0` bytes), hex.
    pub battle_state: String,
    /// Both sides' game versions (`"white"` or `"blue"` / `"black"`).
    pub game_versions: [String; 2],
    /// Both sides' regions (`"us"` or `"jp"`).
    pub game_regions: [String; 2],
    /// Stage background id (`0..=29`; NetBattle stages use `[0, 4, 5, 6, 23, 16, 2, 10]`).
    pub background: u8,
    /// Both sides' NaviCust placements as held in RAM/save.
    pub navicusts: [NaviCustSetup; 2],
    /// Recording console's frame counter on the setup frame.
    pub frame_counter: u16,
    /// Both consoles' joypad repeat phases on the setup frame.
    pub joypad_phases: [u8; 2],
    /// Optional explicit base max HP per side (overrides `navi_stats[s].base_hp_units * 5`).
    #[serde(default)]
    pub base_hp: Option<[u16; 2]>,
}

/// A console's NaviCust as EXE3 stores it (`0x1300` list + `0x1D90` 5×5 grid).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustSetup {
    /// 25 parts × 8 bytes (`0xC8` bytes), hex.
    pub parts: String,
    /// 5×5 grid (`0x19` = 25 bytes), hex.
    pub grid: String,
}

/// A battle object captured from EXE3's pools (8 actors / 32 attacks / 32 effects).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Object {
    #[serde(rename = "type")]
    pub kind: u8,
    pub index: u8,
    pub flags: u8,
    pub params: u32,
    pub state: [u8; 4],
    pub panel: [u8; 2],
    pub alliance: u8,
    #[serde(alias = "flip")]
    pub element: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub pos: [i32; 3],
    pub timer: u16,
    pub anim: u8,
    pub status: u32,
    pub f1: u32,
    #[serde(default)]
    pub sprite: [u32; 3],
}

/// One battle frame captured from EXE3's RAM.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub frame: u32,
    pub state: [u8; 4],
    #[serde(default)]
    pub link: u8,
    pub rng1: u32,
    pub rng2: u32,
    #[serde(default)]
    pub bs: String,
    #[serde(default)]
    pub fight: String,
    pub gauge: u16,
    pub gauge_rate: u16,
    pub paused: u8,
    #[serde(default)]
    pub hud_tasks: u32,
    #[serde(default)]
    pub banner: String,
    /// Both sides' `0x40`-byte NaviStats blocks this frame, hex.
    pub navi_stats: [String; 2],
    /// Per-side joypad inputs `[held, pressed, repeated]`.
    pub input: [[u16; 3]; 2],
    /// Live objects across EXE3's Actor/Attack/Effect pools.
    pub objects: Vec<Object>,
    /// The 18 field panels (6×3 row-major) as `[panel_type, alliance]`.
    pub panels: Vec<[u8; 2]>,
    /// Both sides' `0x24`-byte chip hand blocks (`0x02034060 + 0x24 * side`), hex.
    pub chip_blocks: [String; 2],
}

/// An exchange line: both sides' `0x40`-byte NaviStats after a link sync.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
}

/// A parsed line of an EXE3 JSONL trace.
#[derive(Clone, Debug)]
pub enum Line {
    Setup(Box<Setup>),
    Exchange(Exchange),
    Frame(Box<Frame>),
}

#[derive(Deserialize)]
struct SetupLine {
    setup: Setup,
}

#[derive(Deserialize)]
struct ExchangeLine {
    exchange: Exchange,
}

/// Parse one JSONL line of an EXE3 trace.
pub fn parse_line(l: &str) -> Result<Line, String> {
    let l = l.trim();
    if l.starts_with("{\"setup\"") {
        Ok(Line::Setup(Box::new(
            serde_json::from_str::<SetupLine>(l).map_err(|e| format!("setup: {e}"))?.setup,
        )))
    } else if l.starts_with("{\"exchange\"") {
        Ok(Line::Exchange(
            serde_json::from_str::<ExchangeLine>(l).map_err(|e| format!("exchange: {e}"))?.exchange,
        ))
    } else {
        Ok(Line::Frame(Box::new(serde_json::from_str(l).map_err(|e| format!("frame: {e}"))?)))
    }
}

/// Read all lines of an EXE3 JSONL trace file from disk.
pub fn read(path: impl AsRef<std::path::Path>) -> Result<Vec<Line>, String> {
    let path = path.as_ref();
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::io::BufReader::new(f)
        .lines()
        .enumerate()
        .filter_map(|(n, l)| match l {
            Ok(s) if s.trim().is_empty() => None,
            Ok(s) => Some(parse_line(&s).map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))),
            Err(e) => Some(Err(format!("{}: {e}", path.display()))),
        })
        .collect()
}

/// Parse all non-empty lines of an EXE3 JSONL trace from an in-memory string.
pub fn read_str(text: &str) -> Result<Vec<Line>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(n, l)| parse_line(l).map_err(|e| format!("line {}: {e}", n + 1)))
        .collect()
}

/// Decode a hex string into bytes.
pub fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("odd hex string of {} digits", s.len()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("hex {s:.16}: {e}")))
        .collect()
}

fn navi_stats_hex(s: &str) -> Result<NaviStats, String> {
    let b: [u8; NAVI_STATS] = unhex(s)?
        .try_into()
        .map_err(|b: Vec<u8>| format!("a NaviStats block of {:#x} bytes, not {NAVI_STATS:#x}", b.len()))?;
    Ok(codec::navi_stats(&b))
}

/// Decode a `0x78`-byte EXE3 folder (30 × 4 bytes: `id: u16`, `code: u8`, pad).
fn folder_entries(b: &[u8]) -> Result<[Option<(u16, u8)>; 30], String> {
    if b.len() != FOLDER {
        return Err(format!("an EXE3 folder of {:#x} bytes, not {FOLDER:#x}", b.len()));
    }
    Ok(std::array::from_fn(|i| {
        let at = 4 * i;
        let id = u16::from_le_bytes([b[at], b[at + 1]]);
        let code = b[at + 2];
        (id != 0 && code <= 26).then_some((id, code))
    }))
}

/// A decoded EXE3 trace [`Setup`].
#[derive(Clone, Debug)]
pub struct DecodedSetup {
    pub navi_stats: [NaviStats; 2],
    pub styles: [Style; 2],
    pub versions: [Version; 2],
    pub japanese: [bool; 2],
    pub battle_state: Vec<u8>,
    pub folders: [[Option<(u16, u8)>; 30]; 2],
}

impl DecodedSetup {
    /// The recording console's side (`BattleState +0x0D & 1`).
    pub fn local(&self) -> usize {
        self.battle_state.get(0x0D).copied().unwrap_or(0) as usize & 1
    }
}

/// Validate and decode an EXE3 trace [`Setup`].
pub fn decode_setup(s: &Setup) -> Result<DecodedSetup, String> {
    if s.game != crate::ROOT {
        return Err(format!("expected an {} recording, got {:?}", crate::ROOT, s.game));
    }
    let version = |v: &str| Version::named(v).ok_or_else(|| format!("unknown EXE3 version {v:?}"));
    let region = |r: &str| match r {
        "us" => Ok(false),
        "jp" => Ok(true),
        r => Err(format!("unknown region {r:?}")),
    };
    let style = |side: usize, byte: u8| {
        Style::unpack(byte).ok_or_else(|| format!("side {side}: invalid EXE3 style byte {byte:#04x}"))
    };
    let battle_state = unhex(&s.battle_state)?;
    if battle_state.len() != BATTLE_STATE {
        return Err(format!("a BattleState of {:#x} bytes, not {BATTLE_STATE:#x}", battle_state.len()));
    }
    let folders = [
        folder_entries(&unhex(&s.folders[0])?)?,
        folder_entries(&unhex(&s.folders[1])?)?,
    ];
    for (side, c) in s.navicusts.iter().enumerate() {
        let (parts, grid) = (unhex(&c.parts)?, unhex(&c.grid)?);
        if parts.len() != crate::save::NAVICUST_PARTS * 8 || grid.len() != NAVICUST_GRID {
            return Err(format!(
                "side {side}: NaviCust has {:#x} part bytes and {:#x} grid bytes (expected {:#x} and {:#x})",
                parts.len(),
                grid.len(),
                crate::save::NAVICUST_PARTS * 8,
                NAVICUST_GRID,
            ));
        }
    }
    Ok(DecodedSetup {
        navi_stats: [navi_stats_hex(&s.navi_stats[0])?, navi_stats_hex(&s.navi_stats[1])?],
        styles: [style(0, s.styles[0])?, style(1, s.styles[1])?],
        versions: [version(&s.game_versions[0])?, version(&s.game_versions[1])?],
        japanese: [region(&s.game_regions[0])?, region(&s.game_regions[1])?],
        battle_state,
        folders,
    })
}

/// A decoded battle object from a trace frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedObject {
    pub pool: Pool,
    pub kind: u8,
    pub element: u8,
    pub object: Object,
}

/// A decoded battle frame from an EXE3 trace.
#[derive(Clone, Debug)]
pub struct DecodedFrame {
    pub frame: u32,
    pub panels: Vec<Panel>,
    pub objects: Vec<DecodedObject>,
    pub hands: [HandBlock; 2],
    pub navi_stats: [NaviStats; 2],
}

/// Validate and decode a trace [`Frame`].
pub fn decode_frame(compat: &Compat, f: &Frame) -> Result<DecodedFrame, String> {
    if f.panels.len() != 18 {
        return Err(format!("frame {}: {} panels (expected 18)", f.frame, f.panels.len()));
    }
    let panels = f
        .panels
        .iter()
        .map(|&[t, a]| codec::panel(t, a))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("frame {}: {e}", f.frame))?;
    let mut counts = [0usize; 3];
    let mut objects = Vec::with_capacity(f.objects.len());
    for o in &f.objects {
        let pool = pool_of_type(o.kind).ok_or_else(|| format!("frame {}: invalid EXE3 object type {}", f.frame, o.kind))?;
        let slot = &mut counts[match pool {
            Pool::Actor => 0,
            Pool::Attack => 1,
            Pool::Effect => 2,
        }];
        *slot += 1;
        if *slot > pool_slots(pool) {
            return Err(format!("frame {}: more {pool:?} objects than EXE3's {} slots", f.frame, pool_slots(pool)));
        }
        objects.push(DecodedObject { pool, kind: o.index, element: o.element, object: o.clone() });
    }
    let hand = |s: &str| codec::hand_block(compat, &unhex(s)?);
    Ok(DecodedFrame {
        frame: f.frame,
        panels,
        objects,
        hands: [hand(&f.chip_blocks[0])?, hand(&f.chip_blocks[1])?],
        navi_stats: [navi_stats_hex(&f.navi_stats[0])?, navi_stats_hex(&f.navi_stats[1])?],
    })
}

/// Decode an [`Exchange`] line.
pub fn decode_exchange(e: &Exchange) -> Result<[NaviStats; 2], String> {
    Ok([navi_stats_hex(&e.navi_stats[0])?, navi_stats_hex(&e.navi_stats[1])?])
}

// ---- Rounds & Setup Construction --------------------------------------------

/// One round of an EXE3 trace recording: its setup, exchanges, and frames.
#[derive(Clone, Debug)]
pub struct Round {
    pub setup: Setup,
    pub exchanges: Vec<Exchange>,
    pub frames: Vec<Frame>,
}

/// Group parsed trace lines into rounds (each `Setup` begins a new round).
pub fn rounds_from_lines(lines: Vec<Line>) -> Vec<Round> {
    let mut out: Vec<Round> = Vec::new();
    let mut pending = Vec::new();
    for line in lines {
        match line {
            Line::Setup(s) => out.push(Round {
                setup: *s,
                exchanges: std::mem::take(&mut pending),
                frames: Vec::new(),
            }),
            Line::Exchange(e) => match out.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending.push(e),
            },
            Line::Frame(f) => {
                if let Some(r) = out.last_mut() {
                    r.frames.push(*f);
                }
            }
        }
    }
    out
}

/// Read an EXE3 JSONL trace file and group its lines into rounds.
pub fn rounds(path: impl AsRef<std::path::Path>) -> Result<Vec<Round>, String> {
    Ok(rounds_from_lines(read(path)?))
}

impl Round {
    /// Frames the engine simulates (top state `4` running or `8` end, from
    /// `setup.frame` onward).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames
            .iter()
            .filter(|f| f.frame >= self.setup.frame)
            .take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// Emulated cable link delay in frames.
    pub fn link_delay(&self) -> u8 {
        LINK_DELAY
    }

    /// Inputs and link events for frame index `i` in `frames`.
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let input = std::array::from_fn(|p| PlayerTick::buttons(f.input[p][0] & 0x3FF));
        let mut events = TickEvents::default();
        if i > 0 && f.state[0] == 8 && f.state[1] == 4 && frames[i - 1].state[1] == 0 {
            events.link_closed = true;
        }
        (input, events)
    }

    /// Build a [`RoundSetup`] for this round against `content` and `compat`.
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> Result<RoundSetup, String> {
        setup_of(&self.setup, content, compat)
    }

    /// Construct a fresh [`Battle`] ready to tick this round.
    pub fn start(&self, content: Arc<Content>, compat: &Compat) -> Result<Battle, String> {
        let setup = setup_of(&self.setup, &content, compat)?;
        Ok(Battle::new(setup, content))
    }
}

fn triple_battle(battle_type: u8) -> bool {
    matches!(battle_type, 0x48..=0x4A)
}

fn resolve_stage(content: &Content, compat: &Compat, background: u8) -> Result<StageHandle, String> {
    if let Some((_, entry)) = compat.stage_by_background(background) {
        let candidates = match entry.name.as_str() {
            "sci-lab-net" => [entry.name.as_str(), "scilab-net"],
            "lan-hp" => [entry.name.as_str(), "lans-hp"],
            other => [other, other],
        };
        for key in candidates {
            if let Some(h) = content.defs.stage_by_key(key) {
                return Ok(h);
            }
        }
    }
    content
        .defs
        .stage_by_key("acdc-net")
        .ok_or_else(|| format!("EXE3 content has no stage for background {background:#04x}"))
}

fn battle_folder(
    content: &Content,
    compat: &Compat,
    entries: &[Option<(u16, u8)>; 30],
    regular: u8,
) -> Result<BattleFolder, String> {
    let mut chips = [None; saito_battle::custom::folder::FOLDER_SIZE];
    for (slot, entry) in chips.iter_mut().zip(entries) {
        let Some((id, code)) = *entry else { continue };
        let key = compat.chip_key(id).ok_or_else(|| format!("chip {id:#05x} has no key in EXE3 compat"))?;
        let handle = content
            .defs
            .chip_by_key(key)
            .ok_or_else(|| format!("EXE3 content has no chip {key} ({id:#05x})"))?;
        *slot = Some(FolderChip::new(handle, ChipCode(code)));
    }
    // EXE3 stores folders unrotated; BattleFolder expects the Regular chip at slot 0.
    let regular_pending = if (regular as usize) < chips.len() && chips[regular as usize].is_some() {
        let reg = chips[regular as usize];
        chips.copy_within(0..regular as usize, 1);
        chips[0] = reg;
        true
    } else {
        false
    };
    Ok(BattleFolder { chips, regular_pending })
}

fn reset_stats(content: &Content, base_hp: u16) -> Result<EngineNaviStats, String> {
    let navi = content
        .defs
        .navi_by_key("megaman")
        .ok_or_else(|| "EXE3 content has no megaman navi".to_string())?;
    let mut stats = EngineNaviStats::fresh(navi, content).ok_or_else(|| "megaman has no fresh stats".to_string())?;
    stats.max_base_hp = base_hp;
    stats.hp = base_hp;
    stats.max_hp = base_hp;
    Ok(stats)
}

/// Build a [`RoundSetup`] for [`saito_battle::Battle`] from an EXE3 trace [`Setup`].
pub fn setup_of(setup: &Setup, content: &Content, compat: &Compat) -> Result<RoundSetup, String> {
    if content.game() != crate::ROOT {
        return Err(format!("expected {} content, got {}", crate::ROOT, content.game()));
    }
    if content.defs.rules().is_none() {
        return Err("the content has no rules (EXE3's)".into());
    }
    let d = decode_setup(setup)?;
    let bs = &d.battle_state;

    let bg_name = compat
        .assets
        .backgrounds
        .iter()
        .find(|&(_, &n)| n == setup.background)
        .map(|(name, _)| name.as_str())
        .ok_or_else(|| format!("EXE3 compat has no background {:#04x}", setup.background))?;
    let background = content
        .background_handle(crate::ROOT, bg_name)
        .ok_or_else(|| format!("EXE3 content has no background {bg_name:?} ({:#04x})", setup.background))?;
    let stage = resolve_stage(content, compat, setup.background)?;
    let effects = content.stage(stage).effects | LINK_BATTLE_EFFECTS & !if triple_battle(bs[0x0F]) { 0 } else { SET };
    let settings = saito_battle::BattleSettings { stage, background, effects };

    let local = d.local();
    let base_hp_of = |side: usize| -> u16 {
        if let Some(explicit) = setup.base_hp {
            return explicit[side];
        }
        let units = d.navi_stats[side].base_hp_units;
        if units > 0 { units.saturating_mul(5) } else { 1000 }
    };

    let stats0 = reset_stats(content, base_hp_of(0))?;
    let stats1 = reset_stats(content, base_hp_of(1))?;

    let players = [0usize, 1].map(|side| -> Result<PlayerSetup, String> {
        let folder = battle_folder(content, compat, &d.folders[side], setup.regular[side])?;
        let frames = (setup.frame_counter as u32).wrapping_sub(1) & 0xFFFF;
        let rng = if side == local {
            setup.rng1
        } else {
            setup.rng1s.map_or(0, |r| r[side])
        };
        let mut player = PlayerSetup {
            folder,
            joypad_phase: setup.joypad_phases[side],
            console: ConsoleSetup { rng, tag_pair: None, frames },
            rules: None,
        };
        let navi = content.defs.navi_by_key("megaman").expect("checked in reset_stats");
        player.set_fact(content, "navi", &[Fact::Value(Value::Def(Registry::Navi, navi.0))])?;
        player.set_fact(content, "hp", &[Fact::Value(Value::Int(base_hp_of(side) as i64))])?;
        player.set_fact(content, "style_type", &[Fact::Name(d.styles[side].kind.name())])?;
        player.set_fact(content, "style_element", &[Fact::Name(d.styles[side].element.name())])?;

        let list = unhex(&setup.navicusts[side].parts)?;
        let programs = crate::setup::navicust(content, compat, &crate::save::parts(&list))?;
        player.set_fact(content, "navicust_programs", &programs)?;
        Ok(player)
    });
    let [p0, p1] = players;

    Ok(RoundSetup {
        content: content.hash(),
        settings,
        navi_stats: [stats0, stats1],
        rng: setup.rng2,
        local_side: local as u8,
        score: saito_battle::SetScore {
            wins: bs[0x18],
            losses: bs[0x19],
            round: bs[0x1A],
            max_combo: bs[0x1B],
        },
        later_stages: vec![saito_battle::Stage { stage, background }; 2],
        low_hp_music_latched: false,
        host_keys: saito_battle::setup::HostKeys::NONE,
        custom_screen_limit: None,
        cs_prob_fix: 0,
        players: [p0?, p1?],
    })
}

// ---- Frame-by-Frame Differential Comparison ---------------------------------

/// One field-level discrepancy between the engine and a recorded EXE3 frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub frame: u32,
    pub field: String,
    pub ours: String,
    pub theirs: String,
}

impl fmt::Display for Diff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame {} {}: ours {} theirs {}", self.frame, self.field, self.ours, self.theirs)
    }
}

/// Why a round replay stopped before matching all frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Setup(String),
    Panic { frame: u32, message: String },
    Differs { frame: u32, diffs: Vec<Diff> },
}

/// Outcome of replaying a recorded EXE3 round via [`run_round`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Total battle frames in the round.
    pub frames: usize,
    /// Number of consecutive frames that matched with zero diffs.
    pub matched: usize,
    /// Diffs recorded at the stopping frame (empty if all frames matched).
    pub diffs: Vec<Diff>,
    /// Stopping reason, or `None` if every frame matched cleanly.
    pub stopped: Option<Stop>,
}

impl Report {
    /// Whether every frame of the round matched with no diffs or errors.
    pub fn is_ok(&self) -> bool {
        self.stopped.is_none() && self.diffs.is_empty()
    }
}

fn compiled_engine_stats(b: &Battle, s: &EngineNaviStats) -> String {
    let buster_mult = match s.game_stat(&b.content, "buster_mult") {
        Some(saito_content_api::FieldValue::U8(n)) => n,
        _ => 1,
    };
    format!(
        "sa={} fs={} as={} us={} atk={} rpd={} chg={} cust={} mega={} giga={} bmult={} fb={}",
        s.super_armor as u8,
        s.float_shoes as u8,
        s.air_shoes as u8,
        s.undershirt as u8,
        s.attack,
        s.rapid,
        s.charge,
        s.custom_level,
        s.mega_level,
        s.giga_level,
        buster_mult,
        s.first_barrier.is_some() as u8,
    )
}

fn compiled_trace_stats(s: &NaviStats) -> String {
    format!(
        "sa={} fs={} as={} us={} atk={} rpd={} chg={} cust={} mega={} giga={} bmult={} fb={}",
        s.super_armor as u8,
        s.float_shoes as u8,
        s.air_shoes as u8,
        s.undershirt as u8,
        s.attack,
        s.rapid,
        s.charge,
        s.custom_level,
        s.mega_level,
        s.giga_level,
        s.buster_mult,
        s.first_barrier as u8,
    )
}

fn engine_hand_block(b: &Battle, compat: &Compat, h: &saito_battle::hand::ChipHand) -> HandBlock {
    let chip = |c: saito_content_api::ChipHandle| {
        let key = saito_match::ids::local(&b.content.defs.chip(c).key).to_string();
        codec::Chip {
            id: compat.chip_entry(&key).map_or(0xFFFF, |e| e.id),
            key: Some(key),
        }
    };
    HandBlock {
        ids: h.ids.map(|c| c.map(chip)),
        damage: h.damage,
        bonus: h.attack_bonus,
    }
}

fn format_hand_ids(h: &HandBlock) -> String {
    h.ids
        .iter()
        .map(|c| match c {
            Some(c) => match &c.key {
                Some(k) => format!("{k}({:#05x})", c.id),
                None => format!("{:#05x}", c.id),
            },
            None => "-".to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compare [`Battle`] state against a recorded EXE3 [`Frame`], returning all
/// field-level [`Diff`]s.
pub fn compare_frame(b: &Battle, f: &Frame, compat: &Compat) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let mut check = |field: &str, ours: String, theirs: String| {
        if ours != theirs {
            diffs.push(Diff { frame: f.frame, field: field.to_string(), ours, theirs });
        }
    };

    let r = &b.round;
    check("state", format!("{:?}", [r.top, r.mode, r.sub, r.init]), format!("{:?}", f.state));
    check("rng2", format!("{:#010x}", b.rng.state), format!("{:#010x}", f.rng2));
    check("paused", (b.paused as u8).to_string(), f.paused.to_string());
    check("gauge", format!("{:#x}", b.gauge_for(r.local_side)), format!("{:#x}", f.gauge));
    check("gauge_rate", format!("{:#x}", b.gauge.rate), format!("{:#x}", f.gauge_rate));

    // Optional BattleState bytes when present in the frame.
    if !f.bs.is_empty() {
        if let Ok(bs) = unhex(&f.bs) {
            if bs.len() >= BATTLE_STATE {
                check("actors", format!("{:?}", r.actor_count), format!("{:?}", [bs[0x04], bs[0x05]]));
                check("turn", r.turn.to_string(), bs[0x08].to_string());
                check("time_up", r.time_up.to_string(), bs[0x0B].to_string());
                check("winner", r.winner.to_string(), bs[0x10].to_string());
                check("alive", format!("{:?}", r.alive), format!("{:?}", [bs[0x12], bs[0x13]]));
                check("score", format!("{:?}", [r.wins, r.losses]), format!("{:?}", [bs[0x18], bs[0x19]]));
                check("round", r.round.to_string(), bs[0x1A].to_string());
                check("delay", r.delay.to_string(), i16::from_le_bytes([bs[0x28], bs[0x29]]).to_string());
                check(
                    "battle_time",
                    r.battle_time.to_string(),
                    u32::from_le_bytes([bs[0x40], bs[0x41], bs[0x42], bs[0x43]]).to_string(),
                );
                check("intro_bits", format!("{:#x}", r.intro_bits & 0x0F), format!("{:#x}", bs[0x64]));
            }
        }
    }

    // 18 panels (6x3 row-major).
    let our_panels: Vec<String> = (1..=3)
        .flat_map(|y| (1..=6).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = b.field.panel(x, y).expect("a field panel");
            format!("[{}, {}]", p.kind.0, p.alliance)
        })
        .collect();
    let their_panels: Vec<String> = f.panels.iter().map(|[t, a]| format!("[{t}, {a}]")).collect();
    check("panels", our_panels.join(", "), their_panels.join(", "));

    // Both sides' 0x24-byte chip hand blocks.
    for (side, hex) in f.chip_blocks.iter().enumerate() {
        match unhex(hex).and_then(|raw| codec::hand_block(compat, &raw)) {
            Ok(theirs) => {
                let ours = engine_hand_block(b, compat, &b.hands[side]);
                check(&format!("hand[{side}].ids"), format_hand_ids(&ours), format_hand_ids(&theirs));
                check(&format!("hand[{side}].damage"), format!("{:?}", ours.damage), format!("{:?}", theirs.damage));
                check(&format!("hand[{side}].bonus"), format!("{:?}", ours.bonus), format!("{:?}", theirs.bonus));
            }
            Err(e) => check(&format!("hand[{side}]"), format!("{HAND_BLOCK:#x} bytes"), e),
        }
    }

    // Both sides' 0x40-byte NaviStats blocks (if non-empty).
    for (side, hex) in f.navi_stats.iter().enumerate() {
        if hex.is_empty() {
            continue;
        }
        match navi_stats_hex(hex) {
            Ok(theirs) => {
                check(
                    &format!("navi_stats[{side}]"),
                    compiled_engine_stats(b, &b.stats[side]),
                    compiled_trace_stats(&theirs),
                );
            }
            Err(e) => check(&format!("navi_stats[{side}]"), format!("{NAVI_STATS:#x} bytes"), e),
        }
    }

    // Objects across EXE3's Actor / Attack / Effect pools.
    type ObjFields = Vec<(&'static str, String)>;
    let ours: Vec<ObjFields> = b
        .objects
        .in_order()
        .map(|o| {
            let x = b.objects.get(o);
            let key = &b.content.defs.kind(x.kind).key;
            let kind = match compat.kinds.get(key) {
                Some(k) => format!("type {} #{:#04x}", type_of_pool(o.pool), k.index),
                None => format!("type {} {key}", type_of_pool(o.pool)),
            };
            let action = compat.navi_action(b, o);
            let status = x.collision.map(|c| b.collision.get(c).acc.hit_flags).unwrap_or(0);
            let f1 = x.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
            vec![
                ("kind", kind),
                ("flags", format!("{:#04x}", x.flags)),
                ("state", format!("{:#04x}", x.state)),
                ("action", format!("{action:#04x}")),
                ("phase", format!("{:#04x}", x.phase)),
                ("phase_init", format!("{:#04x}", x.phase_init)),
                ("panel", format!("{:?}", [x.panel.x, x.panel.y])),
                ("alliance", x.alliance.to_string()),
                ("element", format!("{:#04x}", x.element)),
                ("hp", format!("{}/{}", x.hp, x.max_hp)),
                ("pos", format!("{},{},{}", x.pos.x, x.pos.y, x.pos.z)),
                ("timer", x.timer.to_string()),
                ("anim", x.anim.to_string()),
                ("status", format!("{status:#x}")),
                ("f1", format!("{f1:#x}")),
            ]
        })
        .collect();
    let theirs: Vec<ObjFields> = f
        .objects
        .iter()
        .map(|o| {
            vec![
                ("kind", format!("type {} #{:#04x}", o.kind, o.index)),
                ("flags", format!("{:#04x}", o.flags)),
                ("state", format!("{:#04x}", o.state[0])),
                ("action", format!("{:#04x}", o.state[1])),
                ("phase", format!("{:#04x}", o.state[2])),
                ("phase_init", format!("{:#04x}", o.state[3])),
                ("panel", format!("{:?}", o.panel)),
                ("alliance", o.alliance.to_string()),
                ("element", format!("{:#04x}", o.element)),
                ("hp", format!("{}/{}", o.hp, o.max_hp)),
                ("pos", format!("{},{},{}", o.pos[0], o.pos[1], o.pos[2])),
                ("timer", o.timer.to_string()),
                ("anim", o.anim.to_string()),
                ("status", format!("{:#x}", o.status)),
                ("f1", format!("{:#x}", o.f1)),
            ]
        })
        .collect();

    let kinds_of = |list: &[ObjFields]| list.iter().map(|o| o[0].1.clone()).collect::<Vec<_>>();
    if kinds_of(&ours) != kinds_of(&theirs) {
        check("objects", format!("{:?}", kinds_of(&ours)), format!("{:?}", kinds_of(&theirs)));
    } else {
        for (i, (a, t)) in ours.iter().zip(&theirs).enumerate() {
            for ((name, x), (_, y)) in a.iter().zip(t).skip(1) {
                if x != y {
                    check(&format!("object[{i}].{name}"), x.clone(), y.clone());
                }
            }
        }
    }

    diffs
}

/// Replay a recorded EXE3 [`Round`] against `content` and `compat`, comparing
/// each frame and returning a structured [`Report`].
pub fn run_round(round: &Round, content: &Arc<Content>, compat: &Compat) -> Report {
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let mut report = Report { frames: frames.len(), matched: 0, diffs: Vec::new(), stopped: None };

    let mut b = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| round.start(content.clone(), compat))) {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            report.stopped = Some(Stop::Setup(e));
            return report;
        }
        Err(e) => {
            report.stopped = Some(Stop::Panic { frame: round.setup.frame, message: panic_message(&*e) });
            return report;
        }
    };

    let local = b.setup.local_side as usize & 1;
    let mut rng1_since: Option<(u32, u32, u32)> = None;
    let mut layers = saito_battle::custom::Lockstep::new(&b);
    let mut strays = saito_battle::collision::StrayWatch::default();

    for i in 0..frames.len() {
        let (input, events) = round.tick_inputs(i, &frames);
        let input = layers.input(&b, &input);
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events))) {
            report.stopped = Some(Stop::Panic { frame: frames[i].frame, message: panic_message(&*e) });
            return report;
        }
        layers.follow(&b);

        let mut diffs = compare_frame(&b, frames[i], compat);
        if let Some(stray) = strays.difference(&b, &[]) {
            diffs.push(Diff {
                frame: frames[i].frame,
                field: "collision_stray".into(),
                ours: stray,
                theirs: "none".into(),
            });
        }
        if !diffs.is_empty() {
            report.diffs = diffs.clone();
            report.stopped = Some(Stop::Differs { frame: frames[i].frame, diffs });
            return report;
        }

        if let Some(next) = frames.get(i + 1) {
            let engine = b.consoles[local].rng.state;
            if saito_battle::rng::Rng::new(frames[i].rng1).passes(engine, next.rng1, RNG1_PICKS_A_FRAME) {
                rng1_since = None;
            } else if rng1_since.is_none() {
                rng1_since = Some((frames[i].frame, engine, next.rng1));
            }
        }
        report.matched += 1;
    }

    let last_compared = frames.len().checked_sub(2).map(|i| frames[i].frame);
    if let Some((frame, engine, trace)) = rng1_since.filter(|&(frame, _, _)| Some(frame) != last_compared) {
        let diff = Diff {
            frame,
            field: "rng1".into(),
            ours: format!("{engine:#010x}"),
            theirs: format!("{trace:#010x}"),
        };
        report.diffs = vec![diff.clone()];
        report.stopped = Some(Stop::Differs { frame, diffs: vec![diff] });
    }
    report
}

fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, StyleType};
    use std::sync::OnceLock;

    fn exe3_content() -> Arc<Content> {
        static EXE3: OnceLock<Arc<Content>> = OnceLock::new();
        EXE3.get_or_init(|| Arc::new(saito_match::testing::defined(&["exe3"]).unwrap_or_else(|e| panic!("content/exe3: {e}")))).clone()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn make_folder_hex(compat: &Compat) -> String {
        let mut raw = vec![0u8; FOLDER];
        let cannon = compat.chip_entry("cannon").unwrap().id;
        let sword = compat.chip_entry("sword").unwrap().id;
        let airshot1 = compat.chip_entry("airshot1").unwrap().id;
        for slot in 0..30 {
            let (id, code) = match slot {
                0 => (cannon, 0u8),
                1 => (sword, 11u8),
                _ => (airshot1, 26u8),
            };
            raw[4 * slot..4 * slot + 2].copy_from_slice(&id.to_le_bytes());
            raw[4 * slot + 2] = code;
        }
        hex(&raw)
    }

    fn make_navicust_hex() -> (String, String) {
        let mut parts = vec![0u8; crate::save::NAVICUST_PARTS * 8];
        // UnderSht (program 30, variant 0 => id 120) at col 2, row 2.
        parts[0..8].copy_from_slice(&[30 * 4, 0, 2, 2, 0, 0, 0, 0]);
        let mut grid = vec![0u8; NAVICUST_GRID];
        grid[5 * 2 + 2] = 1;
        (hex(&parts), hex(&grid))
    }

    fn make_navi_stats_hex(base_hp: u16, custom_level: u8, buster_mult: u8, undershirt: bool) -> String {
        let mut raw = [0u8; NAVI_STATS];
        raw[0x04] = undershirt as u8;
        raw[0x13] = custom_level;
        raw[0x14] = 5;
        raw[0x15] = 1;
        raw[0x1B] = 0xFF;
        raw[0x26] = buster_mult;
        raw[0x2C..0x2E].copy_from_slice(&(base_hp / 5).to_le_bytes());
        hex(&raw)
    }

    fn make_hand_hex(ids: [u16; 6], damage: [u16; 6], bonus: [u16; 6]) -> String {
        let mut raw = [0u8; HAND_BLOCK];
        for i in 0..6 {
            raw[2 * i..2 * i + 2].copy_from_slice(&ids[i].to_le_bytes());
            raw[0x0C + 2 * i..0x0C + 2 * i + 2].copy_from_slice(&damage[i].to_le_bytes());
            raw[0x18 + 2 * i..0x18 + 2 * i + 2].copy_from_slice(&bonus[i].to_le_bytes());
        }
        hex(&raw)
    }

    fn snapshot_frame(b: &Battle, compat: &Compat, frame: u32, rng1: u32) -> String {
        let r = &b.round;
        let panels: Vec<[u8; 2]> = (1..=3)
            .flat_map(|y| (1..=6).map(move |x| (x, y)))
            .map(|(x, y)| {
                let p = b.field.panel(x, y).unwrap();
                [p.kind.0, p.alliance]
            })
            .collect();
        let chip_blocks: [String; 2] = std::array::from_fn(|side| {
            let h = &b.hands[side];
            let ids = h.ids.map(|c| {
                c.and_then(|c| {
                    let key = saito_match::ids::local(&b.content.defs.chip(c).key);
                    compat.chip_entry(key).map(|e| e.id)
                })
                .unwrap_or(0xFFFF)
            });
            make_hand_hex(ids, h.damage, h.attack_bonus)
        });
        let navi_stats: [String; 2] = std::array::from_fn(|side| {
            let s = &b.stats[side];
            let mut raw = [0u8; NAVI_STATS];
            raw[0x01] = s.super_armor as u8;
            raw[0x02] = s.float_shoes as u8;
            raw[0x03] = s.air_shoes as u8;
            raw[0x04] = s.undershirt as u8;
            raw[0x08] = s.attack;
            raw[0x09] = s.rapid;
            raw[0x0A] = s.charge;
            raw[0x13] = s.custom_level;
            raw[0x14] = s.mega_level;
            raw[0x15] = s.giga_level;
            raw[0x1B] = 0xFF;
            raw[0x26] = match s.game_stat(&b.content, "buster_mult") {
                Some(saito_content_api::FieldValue::U8(n)) => n,
                _ => 1,
            };
            raw[0x2A] = s.first_barrier.is_some() as u8;
            raw[0x2C..0x2E].copy_from_slice(&(s.max_base_hp / 5).to_le_bytes());
            hex(&raw)
        });
        let objects: Vec<serde_json::Value> = b
            .objects
            .in_order()
            .map(|o| {
                let x = b.objects.get(o);
                let idx = compat.kinds.get(&b.content.defs.kind(x.kind).key).map_or(0, |k| k.index);
                let action = compat.navi_action(b, o);
                let status = x.collision.map(|c| b.collision.get(c).acc.hit_flags).unwrap_or(0);
                let f1 = x.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
                serde_json::json!({
                    "type": type_of_pool(o.pool),
                    "index": idx,
                    "flags": x.flags,
                    "params": u32::from_le_bytes(x.params),
                    "state": [x.state, action, x.phase, x.phase_init],
                    "panel": [x.panel.x, x.panel.y],
                    "alliance": x.alliance,
                    "element": x.element,
                    "hp": x.hp,
                    "max_hp": x.max_hp,
                    "pos": [x.pos.x, x.pos.y, x.pos.z],
                    "timer": x.timer,
                    "anim": x.anim,
                    "status": status,
                    "f1": f1,
                    "sprite": [0, 0, 0],
                })
            })
            .collect();
        serde_json::json!({
            "frame": frame,
            "state": [r.top, r.mode, r.sub, r.init],
            "link": 0,
            "rng1": rng1,
            "rng2": b.rng.state,
            "gauge": b.gauge_for(r.local_side),
            "gauge_rate": b.gauge.rate,
            "paused": b.paused as u8,
            "navi_stats": navi_stats,
            "input": [[0, 0, 0], [0, 0, 0]],
            "objects": objects,
            "panels": panels,
            "chip_blocks": chip_blocks,
        })
        .to_string()
    }

    fn build_setup_line(compat: &Compat) -> String {
        let folder = make_folder_hex(compat);
        let (parts, grid) = make_navicust_hex();
        let ns0 = make_navi_stats_hex(1000, 5, 2, true);
        let ns1 = make_navi_stats_hex(1000, 6, 1, true);
        let bs = hex(&[0u8; BATTLE_STATE]);
        let s0 = Style { kind: StyleType::Guts, element: Element::Heat }.pack();
        let s1 = Style { kind: StyleType::Custom, element: Element::Aqua }.pack();
        serde_json::json!({
            "setup": {
                "frame": 100,
                "game": "exe3",
                "navi_stats": [ns0, ns1],
                "folders": [folder, folder],
                "styles": [s0, s1],
                "regular": [1, 255],
                "rng1": 0x1234_5678u32,
                "rng2": 0x9ABC_DEF0u32,
                "battle_state": bs,
                "game_versions": ["white", "blue"],
                "game_regions": ["us", "jp"],
                "background": 0,
                "navicusts": [
                    { "parts": parts, "grid": grid },
                    { "parts": parts, "grid": grid },
                ],
                "frame_counter": 100,
                "joypad_phases": [0, 0],
            }
        })
        .to_string()
    }

    #[test]
    fn parses_synthetic_jsonl_runs_deterministically_and_reports_diffs() {
        let content = exe3_content();
        let mut compat = Compat::exe3().clone();
        // Register the intro effect kind for synthetic round-trip testing.
        compat
            .kinds
            .entry("engine/intro".to_string())
            .or_insert(crate::KindEntry { pool: "effect".into(), index: 0 });
        let compat = &compat;
        let setup_line = build_setup_line(compat);
        let Line::Setup(setup) = parse_line(&setup_line).unwrap() else { panic!("expected setup") };

        // Build RoundSetup and verify compiled style + NaviCust stats.
        let round_setup = setup_of(&setup, &content, compat).unwrap();
        assert_eq!(round_setup.rng, 0x9ABC_DEF0);
        assert!(round_setup.players[0].folder.regular_pending);
        assert!(!round_setup.players[1].folder.regular_pending);
        // Side 0 Regular chip (slot 1: Sword) was moved to slot 0 in BattleFolder.
        let reg_chip = round_setup.players[0].folder.chips[0].unwrap();
        assert_eq!(saito_match::ids::local(&content.defs.chip(reg_chip.id).key), "sword");

        // Record 12 frames from a reference Battle run into a JSONL trace.
        let mut ref_battle = Battle::new(round_setup, content.clone());
        assert!(
            !ref_battle.stats[0].super_armor,
            "EXE3 Guts Style does not grant innate SuperArmor (requires SprArmor NCP)"
        );
        assert_eq!(
            ref_battle.stats[0].game_stat(&content, "buster_mult"),
            Some(saito_content_api::FieldValue::U8(2)),
            "Guts Style doubles normal buster damage"
        );
        assert!(ref_battle.stats[0].undershirt, "NaviCust UnderSht compiled");
        assert_eq!(ref_battle.stats[1].custom_level, 6, "Custom Style sets custom_level = 6");

        let mut layers = saito_battle::custom::Lockstep::new(&ref_battle);
        let mut jsonl_lines = vec![setup_line.clone()];
        let exchange_line = serde_json::json!({
            "exchange": {
                "frame": 99,
                "navi_stats": [
                    make_navi_stats_hex(1000, 5, 2, true),
                    make_navi_stats_hex(1000, 6, 1, true),
                ],
            }
        })
        .to_string();
        jsonl_lines.insert(0, exchange_line);

        for tick in 0..12u32 {
            let input = layers.input(&ref_battle, &[PlayerTick::default(), PlayerTick::default()]);
            ref_battle.tick(&input, TickEvents::default());
            layers.follow(&ref_battle);
            let rng1 = ref_battle.consoles[0].rng.state;
            jsonl_lines.push(snapshot_frame(&ref_battle, compat, 100 + tick, rng1));
        }
        let trace_text = jsonl_lines.join("\n");

        // Parse JSONL, group into rounds, and replay deterministically.
        let parsed = read_str(&trace_text).unwrap();
        let rounds = rounds_from_lines(parsed);
        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].exchanges.len(), 1);
        assert!(decode_exchange(&rounds[0].exchanges[0]).is_ok());

        let report = run_round(&rounds[0], &content, compat);
        assert!(report.is_ok(), "unexpected diffs: {report:?}");
        assert_eq!(report.frames, 12);
        assert_eq!(report.matched, 12);

        // Perturb frame 5's rng2 and gauge in the trace and verify diff reporting catches it at frame 105.
        let mut mutated = rounds[0].clone();
        mutated.frames[5].rng2 ^= 0x1;
        mutated.frames[5].gauge = 0x1234;
        let diff_report = run_round(&mutated, &content, compat);
        assert!(!diff_report.is_ok());
        assert_eq!(diff_report.matched, 5);
        assert!(diff_report.diffs.iter().any(|d| d.frame == 105 && d.field == "rng2"), "{:?}", diff_report.diffs);
        assert!(diff_report.diffs.iter().any(|d| d.frame == 105 && d.field == "gauge"), "{:?}", diff_report.diffs);
    }

    #[test]
    fn decode_frame_rejects_wrong_panel_count_and_invalid_pool_type() {
        let compat = Compat::exe3();
        let bad = Frame {
            frame: 1,
            state: [4, 0, 0, 0],
            link: 0,
            rng1: 0,
            rng2: 0,
            bs: String::new(),
            fight: String::new(),
            gauge: 0,
            gauge_rate: 0x20,
            paused: 0,
            hud_tasks: 0,
            banner: String::new(),
            navi_stats: [hex(&[0u8; NAVI_STATS]), hex(&[0u8; NAVI_STATS])],
            input: [[0; 3]; 2],
            objects: Vec::new(),
            panels: vec![[2, 0]; 17],
            chip_blocks: [hex(&[0xFFu8; HAND_BLOCK]), hex(&[0xFFu8; HAND_BLOCK])],
        };
        assert!(decode_frame(compat, &bad).unwrap_err().contains("17 panels"));
    }

    #[test]
    fn exe3_rook_mine_plasma3_and_navis_run_without_softlock() {
        use saito_battle::input::keys;
        use saito_battle::object::flags as obj_flags;
        let content = exe3_content();
        let compat = Compat::exe3();
        let chip_id = |name: &str| {
            content
                .defs
                .chip_by_key(&format!("exe3/{name}"))
                .or_else(|| content.defs.chip_by_key(name))
                .unwrap_or_else(|| panic!("missing chip {name}"))
        };

        let run_chip = |chip_name: &str, ticks: usize| -> (Battle, bool, bool, bool) {
            let setup_line = build_setup_line(compat);
            let Line::Setup(setup) = parse_line(&setup_line).unwrap() else { panic!("expected setup") };
            let round_setup = setup_of(&setup, &content, compat).unwrap();
            let mut b = Battle::new(round_setup, content.clone());
            let mut layers = saito_battle::custom::Lockstep::new(&b);
            // Advance past intro/custom into active fighting.
            for t in 0..400 {
                let mut raw = [PlayerTick::default(); 2];
                if b.round.mode == saito_battle::battle::mode::CUSTOM {
                    if t % 8 == 0 {
                        raw[0].held = keys::START;
                        raw[1].held = keys::START;
                    } else if t % 8 == 4 {
                        raw[0].held = keys::A;
                        raw[1].held = keys::A;
                    }
                }
                let input = layers.input(&b, &raw);
                b.tick(&input, TickEvents::default());
                layers.follow(&b);
                if b.round.mode == saito_battle::battle::mode::FIGHTING
                    && b.fight.state == saito_battle::battle::fight::FIGHTING
                {
                    break;
                }
            }
            assert_eq!(b.fight.state, saito_battle::battle::fight::FIGHTING, "{chip_name}: never became active");
            // Directly load the chip into P0's hand and press A.
            let cid = chip_id(chip_name);
            let dmg = content.defs.chip(cid).record.damage;
            b.hands[0].ids = [Some(cid), None, None, None, None, None];
            b.hands[0].damage = [dmg, 0, 0, 0, 0, 0];
            b.hands[0].cursor = 0;

            let mut saw_dim = false;
            let mut saw_visible_ctrl = false;
            let mut saw_user_hidden = false;
            for t in 0..ticks {
                let mut raw = [PlayerTick::default(); 2];
                if t == 2 {
                    raw[0].held = keys::A;
                }
                let input = layers.input(&b, &raw);
                b.tick(&input, TickEvents::default());
                layers.follow(&b);
                if b.is_dimmed() {
                    saw_dim = true;
                }
                if let Some(p0) = b.player(0) {
                    if b.objects.get(p0).flags & obj_flags::VISIBLE == 0 {
                        saw_user_hidden = true;
                    }
                }
                for oref in b.objects.in_order() {
                    let o = b.objects.get(oref);
                    let k = &content.defs.kind(o.kind).key;
                    if k.contains("-controller") && (o.flags & obj_flags::VISIBLE != 0) {
                        saw_visible_ctrl = true;
                    }
                }
            }
            (b, saw_dim, saw_visible_ctrl, saw_user_hidden)
        };

        // 1. Rook: dims, exits dimming (no softlock!), spawns 500-HP chess rook (anim 4).
        let (b, saw_dim, _, _) = run_chip("rook", 220);
        assert!(saw_dim, "rook must dim");
        assert!(!b.is_dimmed(), "rook must NOT softlock time-freeze");
        let rook = b
            .objects
            .in_order()
            .into_iter()
            .map(|r| b.objects.get(r))
            .find(|o| saito_match::ids::local(&content.defs.kind(o.kind).key) == "trap/rook-obstacle")
            .expect("rook obstacle must be on field");
        assert_eq!(rook.hp, 500, "rook HP matches EXE3 ROM (500)");
        assert_eq!(rook.anim, 4, "rook uses chess sprite anim 4");

        // 2. All dimming trap & utility chips exit dimming cleanly (no u16 timer softlock!).
        for dim_chip in ["mine", "timebomb", "pawn", "hole", "repair", "nrthwind", "fanfare", "spice1", "team1", "sensor1"] {
            let (b, saw_dim, _, _) = run_chip(dim_chip, 220);
            assert!(saw_dim, "{dim_chip} must dim");
            assert!(!b.is_dimmed(), "{dim_chip} must NOT softlock time-freeze");
        }

        // 3. All 17 Navi chips & 4 Navi Giga chips: show Navi sprite, hide & restore user, exit dimming!
        for navi in [
            "flashman", "plantman", "protoman", "bowlman", "mistman", "roll", "gutsman", "beastman", "bubblman",
            "desrtman", "flamman", "drillman", "metalman", "kingman", "darkman", "japanman", "punk",
            "bass", "serenade", "deltaray", "bass-137",
        ] {
            let (b, saw_dim, saw_navi, saw_hidden) = run_chip(navi, 260);
            assert!(saw_dim, "{navi} must dim");
            assert!(saw_navi, "{navi} must render visible Navi sprite during dimming");
            assert!(saw_hidden, "{navi} must hide user while summoned");
            assert!(!b.is_dimmed(), "{navi} must finish dimming cleanly");
            let p0 = b.objects.get(b.player(0).unwrap());
            assert!(p0.flags & obj_flags::VISIBLE != 0, "{navi} must restore user visibility after leaving");
            let p1 = b.objects.get(b.player(1).unwrap());
            if matches!(navi, "flashman" | "protoman" | "beastman") {
                assert!(p1.hp < 1000, "{navi} must damage opponent (hp={})", p1.hp);
            }
        }

        // 4. Plasma1-3: spawns EleBall 1 panel ahead with ROM HP (10/30/50) and fires sparks around it!
        for (name, expected_hp) in [("plasma1", 10u16), ("plasma2", 30), ("plasma3", 50)] {
            let (b, _, _, _) = run_chip(name, 70);
            let ball = b
                .objects
                .in_order()
                .into_iter()
                .map(|r| b.objects.get(r))
                .find(|o| saito_match::ids::local(&content.defs.kind(o.kind).key) == "elec/plasma-ball")
                .unwrap_or_else(|| panic!("{name} EleBall must spawn"));
            assert_eq!(ball.max_hp, expected_hp, "{name} HP matches EXE3 ROM");
            let p0 = b.objects.get(b.player(0).unwrap());
            assert_eq!(ball.panel.x, p0.panel.x + 1, "{name} spawns 1 panel ahead");
            assert_eq!(ball.panel.y, p0.panel.y);
        }
    }
}
