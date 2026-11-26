use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use saito_assets::{
    Background, BannerLayout, Bundle, ChipIcon, DialogueFont, Field, Hud, HudLettering, MapEntry, Palette, SpriteFrame, SpritePart,
    SpriteSheet, TelopLook, TelopTiles, Tiles,
};
use saito_battle::battle::{Battle, mode};
use saito_battle::content::{AssetNumbers, BackgroundId, Content, SpriteNumber};
use saito_battle::custom::screen::Phase;
use saito_battle::custom::{BattleFolder, FolderChip};
use saito_battle::dimming::DimmingState;
use saito_battle::field::PanelType;
use saito_battle::input::{PlayerTick, ScreenChoice, keys};
use saito_battle::perspective::TelopName;
use saito_battle::rules::Fact;
use saito_battle::setup::RoundSetup;
use saito_battle::{TickEvents};
use saito_content::locale::{self, Strings};
use saito_content_api::{ChipHandle, Registry, Value};
use saito_frontend::game::{Game, Graphics, Loaded};
use saito_frontend::viewer::Viewer;
use saito_match::{Match, Set};
use saito_render::compose::{HEIGHT as BASE_HEIGHT, Size, WIDTH as BASE_WIDTH};
use saito_render::present::present;
use saito_render::vfont::{Cut, TextRenderer, VectorFont};
use saito_render::{Consoles, Enhancements, Renderer};
use saito_replay::{Head, Info, MatchKind, Replay, Tick, TickDecoder, Writer};

struct GameFixture {
    content: Arc<Content>,
    game: Game,
    strings: BTreeMap<String, Option<Arc<Strings>>>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn content_dir() -> PathBuf {
    repo_root().join("content")
}

/// Parses top-level `glyphs = [...]` and `[jp]` `glyphs = [...]` from `content/<game>/compat/text.toml`.
fn parse_compat_text_glyphs(raw: &str) -> (Vec<String>, Vec<String>) {
    let mut en = Vec::new();
    let mut jp = Vec::new();
    let mut section = String::new();
    let mut in_glyphs = false;
    let mut target_is_jp = false;

    for raw_line in raw.lines() {
        let mut line = String::new();
        let mut in_str = false;
        let mut esc = false;
        for c in raw_line.chars() {
            if esc {
                line.push(c);
                esc = false;
                continue;
            }
            if c == '\\' && in_str {
                line.push(c);
                esc = true;
                continue;
            }
            if c == '"' {
                in_str = !in_str;
                line.push(c);
                continue;
            }
            if c == '#' && !in_str {
                break;
            }
            line.push(c);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !in_glyphs && trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            continue;
        }
        if !in_glyphs && trimmed.starts_with("glyphs") && trimmed.contains('[') {
            if section.is_empty() || section == "jp" {
                in_glyphs = true;
                target_is_jp = section == "jp";
            }
        }
        if !in_glyphs {
            continue;
        }
        let dest = if target_is_jp { &mut jp } else { &mut en };
        let chars: Vec<char> = trimmed.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '"' {
                i += 1;
                let mut val = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        match chars[i] {
                            'u' | 'U' if i + 4 < chars.len() => {
                                let hex: String = chars[i + 1..i + 5].iter().collect();
                                if let Ok(code) = u32::from_str_radix(&hex, 16)
                                    && let Some(ch) = char::from_u32(code)
                                {
                                    val.push(ch);
                                }
                                i += 5;
                            }
                            '"' => {
                                val.push('"');
                                i += 1;
                            }
                            '\\' => {
                                val.push('\\');
                                i += 1;
                            }
                            other => {
                                val.push(other);
                                i += 1;
                            }
                        }
                    } else {
                        val.push(chars[i]);
                        i += 1;
                    }
                }
                if i < chars.len() && chars[i] == '"' {
                    i += 1;
                }
                dest.push(val);
            } else {
                if chars[i] == ']' {
                    in_glyphs = false;
                    break;
                }
                i += 1;
            }
        }
    }
    assert!(en.iter().any(|g| g == "?"), "en glyphs must contain '?'");
    assert!(jp.iter().any(|g| g == "?"), "jp glyphs must contain '?'");
    (en, jp)
}

fn make_cell_font_tiles(chars: &[String]) -> Tiles {
    let mut pixels = Vec::with_capacity(chars.len() * 2 * Tiles::TILE);
    for (idx, ch) in chars.iter().enumerate() {
        let blank = ch == " ";
        for half in 0..2 {
            for y in 0..8 {
                for x in 0..8 {
                    if blank {
                        pixels.push(0);
                    } else if (1..7).contains(&x) && ((half == 0 && y >= 2) || (half == 1 && y < 6)) {
                        let bit = ((idx + x + y) & 1) as u8;
                        pixels.push(if bit == 0 { 1 } else { 2 });
                    } else {
                        pixels.push(0);
                    }
                }
            }
        }
    }
    Tiles { pixels }
}

fn make_dialogue_font(chars: &[String]) -> DialogueFont {
    let mut pixels = Vec::with_capacity(chars.len() * DialogueFont::WIDTH * DialogueFont::HEIGHT);
    for ch in chars {
        let blank = ch == " ";
        for y in 0..DialogueFont::HEIGHT {
            for x in 0..DialogueFont::WIDTH {
                pixels.push(if !blank && x < 8 && (2..10).contains(&y) { 1 } else { 0 });
            }
        }
    }
    DialogueFont { pixels, advances: vec![8; chars.len()], chars: chars.to_vec() }
}

fn make_hud_tiles() -> Tiles {
    let mut pixels = vec![0u8; 0x40 * Tiles::TILE];
    let mut fill_glyph = |g: usize, pat: u8| {
        for half in 0..2 {
            let start = (2 * g + half) * Tiles::TILE;
            for y in 0..8 {
                for x in 0..8 {
                    if (1..7).contains(&x) && ((half == 0 && y >= 2) || (half == 1 && y < 6)) {
                        pixels[start + y * 8 + x] = pat;
                    }
                }
            }
        }
    };
    for d in 0..10 {
        fill_glyph(12 + d, 1);
    }
    fill_glyph(22, 1); // HIDDEN_GLYPH
    fill_glyph(23, 1); // PLUS_GLYPH
    fill_glyph(25, 1); // TIMES_GLYPH x
    fill_glyph(26, 2); // TIMES_GLYPH 2
    Tiles { pixels }
}

fn make_telop_glyphs() -> Tiles {
    let mut pixels = vec![0u8; 2 * TelopLook::GLYPHS * Tiles::TILE];
    for g in 0..TelopLook::GLYPHS {
        for half in 0..2 {
            let start = (2 * g + half) * Tiles::TILE;
            for y in 0..8 {
                for x in 0..8 {
                    if (1..7).contains(&x) && ((half == 0 && y >= 2) || (half == 1 && y < 6)) {
                        pixels[start + y * 8 + x] = 1;
                    }
                }
            }
        }
    }
    Tiles { pixels }
}

fn empty_sound_bank() -> m4a::SoundBank {
    m4a::SoundBank {
        mixer: m4a::bank::MixerConfig {
            mix_rate: 13379,
            ds_channels: 4,
            master_volume: 15,
            reverb: 0,
            dac_resolution: 1,
        },
        players: Vec::new(),
        songs: Vec::new(),
        voicegroups: Vec::new(),
        key_maps: Vec::new(),
        samples: Vec::new(),
        waves: Vec::new(),
    }
}

fn build_fixture(game: &str, content: Arc<Content>) -> GameFixture {
    let dir = content_dir().join(game);
    let compat_raw = std::fs::read_to_string(dir.join("compat/text.toml")).expect("compat/text.toml");
    let (en_chars, jp_chars) = parse_compat_text_glyphs(&compat_raw);

    let mut pal: Palette = [0x39CE; 16];
    pal[0] = 0;
    pal[1] = 0x7FFF;
    pal[2] = 0x1084;

    let en_font = make_cell_font_tiles(&en_chars);
    let en_dialogue = make_dialogue_font(&en_chars);
    let jp_font = make_cell_font_tiles(&jp_chars);
    let jp_dialogue = make_dialogue_font(&jp_chars);

    let mut banners = vec![
        BannerLayout {
            x: 40,
            y: 32,
            kind: 0,
            glyphs: Tiles { pixels: vec![1; 40 * Tiles::TILE] },
            ..BannerLayout::default()
        };
        64
    ];

    let mut next_slot = 0usize;
    let mut alloc_slot = |skip_telops: bool| -> u8 {
        while next_slot < 64 {
            let n = (next_slot * 4) as u8;
            next_slot += 1;
            if skip_telops && (n == 0x4C || n == 0x50) {
                continue;
            }
            return n;
        }
        panic!("ran out of banner slots");
    };

    let banner_names = &content.assets.banners;
    let mut banner_numbers = Vec::with_capacity(banner_names.len());
    for name in banner_names {
        let local = saito_content_api::keys::local(name);
        if game != "exe4" && local == "telop" {
            banners[0x4C / 4] = BannerLayout { x: 0, y: 32, kind: 3, ..BannerLayout::default() };
            banner_numbers.push(0x4C);
        } else if game != "exe4" && local == "telop-remote" {
            banners[0x50 / 4] = BannerLayout { x: 120, y: 32, kind: 3, ..BannerLayout::default() };
            banner_numbers.push(0x50);
        } else {
            banner_numbers.push(alloc_slot(game != "exe4"));
        }
    }

    let mut hud = Hud {
        tiles: make_hud_tiles(),
        first_tile: match game {
            "exe4" => 0x130,
            "exe5" => 0x180,
            _ => 0x1A0,
        },
        hp_palettes: [pal, pal, pal],
        gauge_palette: pal,
        font: en_font,
        font_chars: en_chars,
        dialogue_font: en_dialogue,
        enemy_palette: pal,
        icon_palette: pal,
        banners,
        banner_palette: pal,
        waiting_palette: pal,
        warning_palette: pal,
        language: "en".to_string(),
        languages: vec![(
            "ja".to_string(),
            HudLettering {
                font: jp_font,
                font_chars: jp_chars,
                dialogue_font: jp_dialogue,
                banner_palette: pal,
                waiting_palette: pal,
                ..HudLettering::default()
            },
        )],
        ..Hud::default()
    };

    if game == "exe4" {
        hud.layout.telop = Some(TelopLook {
            places: [(0, 32), (120, 32)],
            glyphs: make_telop_glyphs(),
            palette: pal,
            tiles: Some(TelopTiles {
                banner: 0x360,
                names: [0x360, 0x384],
                signs: 0x380,
                pause_loads_banner_palette: true,
            }),
        });
    }

    for i in 0..content.defs.chips.len() {
        let key = saito_content_api::keys::local(&content.defs.chips[i].key).to_string();
        hud.chip_icons.push(ChipIcon { key, tiles: Tiles { pixels: vec![1; 4 * Tiles::TILE] } });
    }

    let bg_pal: Palette = [0x1084; 16];
    let field = Field {
        tiles: Tiles { pixels: vec![1; 8 * Tiles::TILE] },
        palettes: vec![bg_pal; 16],
        panel_types: (0..content.rules().panels.types.len())
            .map(|i| content.rules().panels.name(PanelType(i as u8)).to_string())
            .collect(),
        ..Field::default()
    };
    let background = Background {
        tiles: Tiles { pixels: vec![1; Tiles::TILE] },
        palette: Some(bg_pal),
        map_width: 32,
        map_height: 32,
        map: vec![MapEntry { tile: 0, hflip: false, vflip: false, palette: 0 }; 32 * 32],
        ..Background::default()
    };
    let dummy_sprite = SpriteSheet {
        category: 0,
        index: 0,
        animations: vec![vec![SpriteFrame {
            duration: 1,
            flags: 0xC0,
            parts: 0,
            tileset: 0,
            palette_set: 0,
        }]],
        part_lists: vec![vec![SpritePart { tile: 0, x: -4, y: -4, width: 8, height: 8, ..Default::default() }]],
        tilesets: vec![Tiles { pixels: vec![0; Tiles::TILE] }],
        palette_sets: vec![vec![pal]],
        ..SpriteSheet::default()
    };

    let bundle = Arc::new(Bundle {
        sprites: vec![dummy_sprite],
        field,
        backgrounds: vec![Some(background)],
        hud,
        ..Bundle::default()
    });

    let numbers = Arc::new(AssetNumbers {
        sprites: vec![SpriteNumber { category: 0, index: 0 }; content.assets.sprites.len()],
        sounds: vec![0; content.assets.sounds.len()],
        banners: banner_numbers,
        backgrounds: vec![0; content.assets.backgrounds.len()],
        mugshots: vec![0; content.assets.mugshots.len()],
    });

    let game_obj = Game {
        name: game.to_string(),
        content: content.clone(),
        dir: content_dir(),
        graphics: bundle,
        sound: Arc::new(empty_sound_bank()),
        ds_sound: None,
        ds_voices: None,
        numbers,
        index: Default::default(),
        others: Default::default(),
        report: Default::default(),
    };

    let mut strings = BTreeMap::new();
    for lang in ["ja", "zh-Hans"] {
        let path = dir.join(format!("locales/{lang}.toml"));
        let text = std::fs::read_to_string(&path).expect("locale file");
        let parsed = locale::parse(&text, &path.display().to_string()).expect("parse locale");
        strings.insert(lang.to_string(), (lang != saito_content::locale::OWN).then(|| Arc::new(parsed)));
    }

    GameFixture { content, game: game_obj, strings }
}

fn fixture(game: &str) -> &'static GameFixture {
    static EXE4: OnceLock<GameFixture> = OnceLock::new();
    static EXE5: OnceLock<GameFixture> = OnceLock::new();
    static EXE6: OnceLock<GameFixture> = OnceLock::new();
    match game {
        "exe4" => EXE4.get_or_init(|| build_fixture("exe4", saito_match::testing::exe4_content())),
        "exe5" => EXE5.get_or_init(|| build_fixture("exe5", saito_match::testing::exe5_content())),
        "exe6" => EXE6.get_or_init(|| build_fixture("exe6", saito_match::testing::exe6_content())),
        other => panic!("unsupported game {other}"),
    }
}

fn loaded_for(fix: &GameFixture, lang: &str, enhancements: Enhancements) -> Loaded {
    let lang_bundle = Arc::new((*fix.game.graphics).clone().for_language(lang));
    let graphics = Graphics {
        bundle: lang_bundle,
        others: fix.game.others.clone(),
        strings: fix.strings.get(lang).cloned().flatten(),
        numbers: fix.game.numbers.clone(),
        report: Default::default(),
    };
    let font = if enhancements.wants_font() {
        Some(saito_render::vfont::bundled_font(Cut::of(lang)).clone())
    } else {
        None
    };
    Loaded {
        game: fix.game.clone(),
        graphics,
        enhancements,
        font,
        sound: None,
    }
}

fn renderer_for(fix: &GameFixture, consoles: Consoles, lang: &str, enhancements: Enhancements, size: Size) -> Renderer {
    let loaded = loaded_for(fix, lang, enhancements);
    let mut r = loaded.renderer();
    r.consoles = consoles;
    r.size = size;
    r
}

fn find_chip(content: &Content, local_key: &str) -> ChipHandle {
    let matches: Vec<ChipHandle> = (0..content.defs.chips.len())
        .filter(|&i| saito_content_api::keys::local(&content.defs.chips[i].key) == local_key)
        .map(|i| ChipHandle(i as u16))
        .collect();
    assert!(!matches.is_empty(), "chip {local_key:?} not found in {}", content.game());
    matches[0]
}

fn folder_chip(content: &Content, local_key: &str) -> FolderChip {
    let h = find_chip(content, local_key);
    let code = content.chip(h).codes[0];
    FolderChip::new(h, code)
}

fn base_setup(fix: &GameFixture, side0_chips: &[&str], side1_chips: &[&str], exe4_anti_chip: bool) -> (Match, RoundSetup) {
    let mut m = saito_match::pick::live(&fix.content, &fix.game.name, 1, None).expect("live match");
    m.rounds.truncate(1);
    m.seed = Some(1);
    if exe4_anti_chip {
        let card = saito_match::ids::entry(&fix.content, "exe4", "patch_cards", "040-b-left-ability-activation")
            .expect("exe4 patch card 040");
        m.sides[0]
            .set_fact(
                &fix.content,
                "patch_cards",
                &[Fact::Record(vec![
                    ("card", Fact::Value(Value::Def(Registry::Entry, card.0))),
                    ("slot", Fact::Value(Value::Int(3))),
                ])],
            )
            .expect("set patch_cards");
    }
    let mut setup = m.round(&fix.content, 1);
    setup.settings.background = BackgroundId(0);
    for stats in &mut setup.navi_stats {
        stats.hp = 1000;
        stats.max_hp = 1000;
    }
    let apply_chips = |folder: &mut BattleFolder, keys: &[&str]| {
        let fallback = folder_chip(&fix.content, "cannon");
        for i in 0..30 {
            folder.chips[i] = Some(keys.get(i).map_or(fallback, |k| folder_chip(&fix.content, k)));
        }
        folder.regular_pending = false;
    };
    apply_chips(&mut setup.players[0].folder, side0_chips);
    apply_chips(&mut setup.players[1].folder, side1_chips);
    (m, setup)
}

fn is_controllable(b: &Battle, side: u8) -> bool {
    let Some(r) = b.player(side) else { return false };
    let Some(a) = b.objects.get(r).actor else { return false };
    b.actors.get(a).status & saito_battle::actor::status::CONTROLLABLE != 0
}

fn step_into_fight_with_picks(b: &mut Battle, picks0: &[u8], picks1: &[u8]) {
    let mut queue0: Vec<ScreenChoice> = picks0.iter().map(|&s| ScreenChoice::Pick(s)).collect();
    queue0.push(ScreenChoice::Ok);
    let mut queue1: Vec<ScreenChoice> = picks1.iter().map(|&s| ScreenChoice::Pick(s)).collect();
    queue1.push(ScreenChoice::Ok);
    let (mut i0, mut i1) = (0usize, 0usize);

    for _ in 0..500 {
        let mut p = [PlayerTick::default(); 2];
        if b.round.mode == mode::CUSTOM {
            if b.custom.sides[0].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing)
                && let Some(&c) = queue0.get(i0)
            {
                p[0].choice = Some(c);
                i0 += 1;
            }
            if b.custom.sides[1].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing)
                && let Some(&c) = queue1.get(i1)
            {
                p[1].choice = Some(c);
                i1 += 1;
            }
        }
        b.tick(&p, TickEvents::default());
        if b.round.mode == mode::FIGHTING
            && !b.banner.active
            && !b.is_dimmed()
            && is_controllable(b, 0)
            && is_controllable(b, 1)
        {
            return;
        }
    }
    panic!("battle did not enter controllable fighting within 500 ticks");
}

fn wait_until_ready(b: &mut Battle, side: u8, trace: &mut Vec<Battle>) {
    for _ in 0..300 {
        if !b.is_dimmed() && !b.banner.active && !b.remote_telop.active && is_controllable(b, side) {
            return;
        }
        b.tick(&[PlayerTick::default(); 2], TickEvents::default());
        trace.push(b.clone());
    }
    panic!("side {side} did not become controllable within 300 ticks");
}

fn step_panel_to(b: &mut Battle, side: u8, want_x: u8, held: u16, trace: &mut Vec<Battle>) {
    wait_until_ready(b, side, trace);
    for _ in 0..6 {
        let cur_x = b.player(side).map(|r| b.objects.get(r).panel.x).unwrap_or(0);
        if cur_x == want_x {
            wait_until_ready(b, side, trace);
            return;
        }
        let mut p = [PlayerTick::default(); 2];
        p[side as usize].held = held;
        b.tick(&p, TickEvents::default());
        trace.push(b.clone());
        wait_ticks(b, 1, trace);
        wait_until_ready(b, side, trace);
    }
    let cur_x = b.player(side).map(|r| b.objects.get(r).panel.x);
    panic!("side {side} did not reach panel x={want_x} (at {cur_x:?})");
}

fn wait_ticks(b: &mut Battle, n: usize, trace: &mut Vec<Battle>) {
    for _ in 0..n {
        b.tick(&[PlayerTick::default(); 2], TickEvents::default());
        trace.push(b.clone());
    }
}

fn press_cast(b: &mut Battle, side: usize, held: u16, trace: &mut Vec<Battle>) {
    let mut p = [PlayerTick::default(); 2];
    p[side].held = held;
    b.tick(&p, TickEvents::default());
    trace.push(b.clone());
    b.tick(&[PlayerTick::default(); 2], TickEvents::default());
    trace.push(b.clone());
}

fn render_rgba(fix: &GameFixture, renderer: &mut Renderer, b: &Battle, font: Option<&Arc<VectorFont>>) -> Vec<u32> {
    let frame = renderer.render(b, [None, None]);
    let mut tr = font.map(|f| TextRenderer::new(f.clone()));
    let size = renderer.size;
    let mut out = vec![0u32; size.width * size.height];
    present(&frame, tr.as_mut(), &mut out, size.width, size.height);
    let _ = fix;
    out
}

fn count_diff_in_box(a: &[u32], b: &[u32], w: usize, x0: usize, x1: usize, y0: usize, y1: usize) -> usize {
    let mut diffs = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let idx = y * w + x;
            if a[idx] != b[idx] {
                diffs += 1;
            }
        }
    }
    diffs
}

fn telop_problem_lines(r: &Renderer, lang: &str) -> Vec<String> {
    r.problems
        .lines()
        .iter()
        .filter(|line| {
            let l = line.to_ascii_lowercase();
            l.contains("telop")
                || l.contains("unknown")
                || l.contains("has no banner")
                || (lang != "zh-Hans" && l.contains("no glyph for"))
        })
        .cloned()
        .collect()
}

fn check_telop_snapshot(
    fix: &GameFixture,
    trace: &[Battle],
    tick_idx: usize,
    consoles: Consoles,
    view: u8,
    lang: &str,
    enhancements: Enhancements,
    size: Size,
    expected_name: TelopName,
    expected_remote: bool,
    step_label: &str,
    failures: &mut Vec<String>,
) {
    let mut r = renderer_for(fix, consoles, lang, enhancements, size);
    let mut blank_r = renderer_for(fix, consoles, lang, enhancements, size);
    for snap in &trace[..tick_idx] {
        let mut s = snap.clone();
        s.setup.local_side = view;
        r.observe(&s);
        blank_r.observe(&s);
    }
    let mut b = trace[tick_idx].clone();
    b.setup.local_side = view;
    r.observe(&b);

    let mut blank_b = b.clone();
    blank_b.banner = Default::default();
    blank_b.remote_telop = Default::default();
    blank_r.observe(&blank_b);

    let shown_list = match consoles {
        Consoles::One => b.telops_for(view),
        Consoles::Both => b.telops_for_both(view),
    };
    let found = shown_list
        .into_iter()
        .flatten()
        .find(|(t, _)| t.remote == expected_remote)
        .map(|(t, _)| t);

    let label = format!("game={} view={} consoles={consoles:?} lang={lang} enh={enhancements:?} size={}x{} step={step_label} remote={expected_remote}", fix.game.name, view, size.width, size.height);
    match found {
        None => {
            failures.push(format!("{label}: missing expected ShownTelop (list={shown_list:?})"));
            return;
        }
        Some(t) => {
            if t.name != expected_name {
                failures.push(format!("{label}: expected name {expected_name:?}, got {:?}", t.name));
            }
        }
    }

    let font = if enhancements.wants_font() {
        Some(saito_render::vfont::bundled_font(Cut::of(lang)).clone())
    } else {
        None
    };
    let img = render_rgba(fix, &mut r, &b, font.as_ref());
    let blank_img = render_rgba(fix, &mut blank_r, &blank_b, font.as_ref());

    let (x0, x1) = if expected_remote {
        (size.width.saturating_sub(140), size.width)
    } else {
        (0, 140.min(size.width))
    };
    let (y0, y1) = if size.height == BASE_HEIGHT {
        (24, 56)
    } else {
        let dy = (size.height - BASE_HEIGHT) / 2;
        (dy + 24, dy + 56)
    };
    let diffs = count_diff_in_box(&img, &blank_img, size.width, x0, x1, y0, y1);
    if diffs == 0 {
        failures.push(format!("{label}: 0 diff pixels in telop band x={x0}..{x1} y={y0}..{y1}"));
    }

    let probs = telop_problem_lines(&r, lang);
    if !probs.is_empty() {
        failures.push(format!("{label}: problems={probs:?}"));
    }
}

fn find_telop_step_ticks(trace: &[Battle], view: u8, remote: bool) -> (Option<usize>, Option<usize>) {
    let find_step = |want_step: u8| -> Option<usize> {
        trace.iter().enumerate().find_map(|(idx, b)| {
            let mut copy = b.clone();
            copy.setup.local_side = view;
            copy.telops_for(view)
                .into_iter()
                .flatten()
                .any(|(t, blk)| t.remote == remote && blk.step == want_step)
                .then_some(idx)
        })
    };
    let s0 = find_step(0);
    // Pick hold tick with timer >= 3 to stay clear of bounce squish too, or any step == 4
    let s4 = trace.iter().enumerate().find_map(|(idx, b)| {
        let mut copy = b.clone();
        copy.setup.local_side = view;
        copy.telops_for(view)
            .into_iter()
            .flatten()
            .any(|(t, blk)| t.remote == remote && blk.step == 4 && blk.timer >= 3)
            .then_some(idx)
    }).or_else(|| find_step(4));
    (s0, s4)
}

fn verify_defensive_mark_pixels(
    fix: &GameFixture,
    b_orig: &Battle,
    trap_side: u8,
    consoles: Consoles,
    view: u8,
    lang: &str,
    enhancements: Enhancements,
    failures: &mut Vec<String>,
) {
    let size = Size { width: BASE_WIDTH, height: BASE_HEIGHT };
    let mut r = renderer_for(fix, consoles, lang, enhancements, size);
    let mut blank_r = renderer_for(fix, consoles, lang, enhancements, size);

    let mut b = b_orig.clone();
    b.setup.local_side = view;
    let mut blank = b.clone();
    blank.linked = Default::default();

    let font = if enhancements.wants_font() {
        Some(saito_render::vfont::bundled_font(Cut::of(lang)).clone())
    } else {
        None
    };
    let img = render_rgba(fix, &mut r, &b, font.as_ref());
    let blank_img = render_rgba(fix, &mut blank_r, &blank, font.as_ref());
    let (x0, x1) = match consoles {
        Consoles::One if view == trap_side => (40, 88),
        Consoles::One => (200, 240),
        Consoles::Both if view == trap_side => (40, 88),
        Consoles::Both => (152, 200),
    };
    let diffs = count_diff_in_box(&img, &blank_img, BASE_WIDTH, x0, x1, 14, 34);
    if diffs == 0 {
        failures.push(format!(
            "defensive mark ???? missing: game={} trap_side={} view={} consoles={consoles:?} lang={lang} enh={enhancements:?} x={x0}..{x1}",
            fix.game.name, trap_side, view
        ));
    }
}

fn verify_used_chip_pixels(fix: &GameFixture, trace: &[Battle], user_side: u8, expected_chip: ChipHandle, failures: &mut Vec<String>) {
    let viewer = user_side ^ 1;
    let Some((idx, _)) = trace.iter().enumerate().find(|(_, b)| b.used_chips[user_side as usize].is_some_and(|u| u.chip == expected_chip)) else {
        failures.push(format!("used_chip missing for {} side={} chip={expected_chip:?}", fix.game.name, user_side));
        return;
    };
    for (lang, enh) in [
        ("ja", Enhancements::NONE),
        ("ja", Enhancements::ALL),
        ("zh-Hans", Enhancements::NONE),
        ("zh-Hans", Enhancements::ALL),
    ] {
        let size = Size { width: BASE_WIDTH, height: BASE_HEIGHT };
        let mut r = renderer_for(fix, Consoles::One, lang, enh, size);
        let mut blank_r = renderer_for(fix, Consoles::One, lang, enh, size);
        for snap in &trace[..idx] {
            let mut s = snap.clone();
            s.setup.local_side = viewer;
            r.observe(&s);
            blank_r.observe(&s);
        }
        let mut b = trace[idx].clone();
        b.setup.local_side = viewer;
        r.observe(&b);
        let mut blank = b.clone();
        blank.used_chips = [None; 2];
        blank_r.observe(&blank);

        let font = if enh.wants_font() { Some(saito_render::vfont::bundled_font(Cut::of(lang)).clone()) } else { None };
        let img = render_rgba(fix, &mut r, &b, font.as_ref());
        let blank_img = render_rgba(fix, &mut blank_r, &blank, font.as_ref());
        let diffs = count_diff_in_box(&img, &blank_img, BASE_WIDTH, 110, BASE_WIDTH, 24, 56);
        if diffs == 0 {
            failures.push(format!(
                "used_chip pixels missing: game={} viewer={} lang={lang} enh={enh:?}",
                fix.game.name, viewer
            ));
        }
        let probs = telop_problem_lines(&r, lang);
        if !probs.is_empty() {
            failures.push(format!("used_chip problems: game={} lang={lang} enh={enh:?} {probs:?}", fix.game.name));
        }
    }
}

#[test]
fn matrix_a_all_trap_casts_across_generations() {
    let mut failures = Vec::new();
    let specs: &[(&str, &[&str])] = &[
        ("exe4", &["antifire", "antiwatr", "antielec", "antiwood", "antinavi", "antidmg", "antiswrd", "antirecv", "mine"]),
        ("exe5", &["antifire", "antiwatr", "antielec", "antiwood", "antinavi", "antidmg", "antiswrd", "antirecv", "mine"]),
        ("exe6", &["elemtrap", "antinavi", "antidmg", "antiswrd", "antirecv", "mine"]),
    ];

    let full_matrix_langs = ["ja", "zh-Hans"];
    let full_matrix_enhs = [
        Enhancements::NONE,
        Enhancements { text: true, ..Enhancements::NONE },
        Enhancements { hud: true, ..Enhancements::NONE },
        Enhancements::ALL,
    ];

    for &(game, chips) in specs {
        let fix = fixture(game);
        for (chip_idx, &chip_key) in chips.iter().enumerate() {
            for caster in 0..=1u8 {
                if caster == 1 && chip_idx > 0 && chip_key != "antinavi" {
                    continue;
                }
                let (s0, s1): (&[&str], &[&str]) = if caster == 0 { (&[chip_key], &[]) } else { (&[], &[chip_key]) };
                let (_, setup) = base_setup(fix, s0, s1, false);
                let mut b = Battle::new(setup, fix.content.clone());
                if caster == 0 {
                    step_into_fight_with_picks(&mut b, &[0], &[]);
                } else {
                    step_into_fight_with_picks(&mut b, &[], &[0]);
                }
                let mut trace = Vec::new();
                wait_ticks(&mut b, 4, &mut trace);
                press_cast(&mut b, caster as usize, keys::A, &mut trace);
                wait_ticks(&mut b, 100, &mut trace);

                let chip_handle = find_chip(&fix.content, chip_key);
                let is_mine = chip_key == "mine";

                let view_configs: Vec<(Consoles, u8)> = vec![
                    (Consoles::One, caster),
                    (Consoles::One, caster ^ 1),
                    (Consoles::Both, 0),
                    (Consoles::Both, 1),
                ];

                for (consoles, view) in view_configs {
                    let remote = view != caster;
                    let (s0_tick, s4_tick) = find_telop_step_ticks(&trace, view, remote);
                    if s0_tick.is_none() || s4_tick.is_none() {
                        failures.push(format!("{game} {chip_key} caster={caster} view={view}: telop step ticks missing (s0={s0_tick:?}, s4={s4_tick:?})"));
                        continue;
                    }
                    let expected_name = match (consoles, remote, is_mine) {
                        (_, _, true) => TelopName::Chip(chip_handle),
                        (Consoles::One, false, false) => TelopName::Chip(chip_handle),
                        (Consoles::One, true, false) => TelopName::Hidden,
                        (Consoles::Both, _, false) => TelopName::Chip(chip_handle),
                    };

                    let thorough = chip_key == "antidmg" || chip_key == "antinavi" || chip_key == "elemtrap";
                    let langs: &[&str] = if thorough { &full_matrix_langs } else { &["ja", "zh-Hans"] };
                    let enhs: &[Enhancements] = if thorough { &full_matrix_enhs } else { &[Enhancements::NONE, Enhancements::ALL] };

                    for &lang in langs {
                        for &enh in enhs {
                            let sizes: &[Size] = if enh.fill && thorough {
                                &[Size { width: 240, height: 160 }, Size { width: 320, height: 160 }]
                            } else {
                                &[Size { width: 240, height: 160 }]
                            };
                            for &size in sizes {
                                for (step_label, idx) in [("step0", s0_tick.unwrap()), ("step4", s4_tick.unwrap())] {
                                    check_telop_snapshot(
                                        fix,
                                        &trace,
                                        idx,
                                        consoles,
                                        view,
                                        lang,
                                        enh,
                                        size,
                                        expected_name,
                                        remote,
                                        &format!("{chip_key}:{step_label}"),
                                        &mut failures,
                                    );
                                }
                            }
                        }
                    }
                }

                // Check persistent ???? defensive marks once trap finishes setting up
                if !is_mine
                    && let Some(armed_b) = trace.iter().find(|s| !s.is_dimmed() && s.linked[caster as usize].chip.is_some())
                {
                    for &(consoles, view) in &[(Consoles::One, caster), (Consoles::One, caster ^ 1), (Consoles::Both, 0)] {
                        for &enh in &[Enhancements::NONE, Enhancements::ALL] {
                            verify_defensive_mark_pixels(fix, armed_b, caster, consoles, view, "zh-Hans", enh, &mut failures);
                        }
                    }
                }
            }
        }
    }

    // EXE6 BodyGrd via PA sequence: antinavi, antiswrd, antidmg
    {
        let fix = fixture("exe6");
        let mut setup = base_setup(fix, &["cannon"], &[], false).1;
        let an = find_chip(&fix.content, "antinavi");
        let asw = find_chip(&fix.content, "antiswrd");
        let ad = find_chip(&fix.content, "antidmg");
        let star = saito_battle::content::ChipCode::ASTERISK;
        setup.players[0].folder.chips[0] = Some(FolderChip::new(an, star));
        setup.players[0].folder.chips[1] = Some(FolderChip::new(asw, star));
        setup.players[0].folder.chips[2] = Some(FolderChip::new(ad, star));
        let mut b = Battle::new(setup, fix.content.clone());
        step_into_fight_with_picks(&mut b, &[0, 1, 2], &[]);
        let mut trace = Vec::new();
        wait_ticks(&mut b, 4, &mut trace);
        press_cast(&mut b, 0, keys::A, &mut trace);
        wait_ticks(&mut b, 100, &mut trace);
        let bg = find_chip(&fix.content, "bodygrd");
        for (consoles, view, want) in [
            (Consoles::One, 0, TelopName::Chip(bg)),
            (Consoles::One, 1, TelopName::Hidden),
            (Consoles::Both, 0, TelopName::Chip(bg)),
        ] {
            let (s0, s4) = find_telop_step_ticks(&trace, view, view != 0);
            if let (Some(i0), Some(i4)) = (s0, s4) {
                for enh in [Enhancements::NONE, Enhancements::ALL] {
                    check_telop_snapshot(fix, &trace, i0, consoles, view, "ja", enh, Size { width: 240, height: 160 }, want, view != 0, "bodygrd:step0", &mut failures);
                    check_telop_snapshot(fix, &trace, i4, consoles, view, "zh-Hans", enh, Size { width: 240, height: 160 }, want, view != 0, "bodygrd:step4", &mut failures);
                }
            } else {
                failures.push(format!("exe6 bodygrd telop ticks missing: s0={s0:?}, s4={s4:?}"));
            }
        }
    }

    // EXE4 B+Left megaman/anti-chip (TelopHidden::FromBoth)
    {
        let fix = fixture("exe4");
        let (_, setup) = base_setup(fix, &["cannon"], &[], true);
        let mut b = Battle::new(setup, fix.content.clone());
        step_into_fight_with_picks(&mut b, &[], &[]);
        let mut trace = Vec::new();
        wait_ticks(&mut b, 4, &mut trace);
        press_cast(&mut b, 0, keys::LEFT | keys::B, &mut trace);
        wait_ticks(&mut b, 100, &mut trace);
        for (consoles, view) in [(Consoles::One, 0), (Consoles::One, 1), (Consoles::Both, 0)] {
            let (s0, s4) = find_telop_step_ticks(&trace, view, view != 0);
            if let (Some(i0), Some(i4)) = (s0, s4) {
                for &lang in &full_matrix_langs {
                    for &enh in &full_matrix_enhs {
                        check_telop_snapshot(fix, &trace, i0, consoles, view, lang, enh, Size { width: 240, height: 160 }, TelopName::Hidden, view != 0, "exe4-b-left:step0", &mut failures);
                        check_telop_snapshot(fix, &trace, i4, consoles, view, lang, enh, Size { width: 240, height: 160 }, TelopName::Hidden, view != 0, "exe4-b-left:step4", &mut failures);
                    }
                }
            } else {
                failures.push(format!("exe4 b+left telop ticks missing: s0={s0:?}, s4={s4:?}"));
            }
        }
    }

    assert!(failures.is_empty(), "matrix_a failures ({}):\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn matrix_b_trap_cut_ins() {
    let mut failures = Vec::new();
    for game in ["exe4", "exe5", "exe6"] {
        let fix = fixture(game);
        let dim_chip = if game == "exe4" { "invis" } else { "invisibl" };
        let cases: &[(&str, &str, bool, bool)] = &[
            ("antidmg", dim_chip, true, false),
            (dim_chip, "antidmg", false, true),
            ("antidmg", "antiswrd", true, true),
        ];

        for &(c0_key, c1_key, c0_trap, c1_trap) in cases {
            let (_, setup) = base_setup(fix, &[c0_key], &[c1_key], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 4, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);

            let mut cut_in_done = false;
            for _ in 0..80 {
                let mut p = [PlayerTick::default(); 2];
                if !cut_in_done && b.is_dimmed() && b.dimming[0].state == DimmingState::ShowingName {
                    p[1].held = keys::A;
                    cut_in_done = true;
                }
                b.tick(&p, TickEvents::default());
                trace.push(b.clone());
            }
            wait_ticks(&mut b, 80, &mut trace);
            assert!(cut_in_done, "{game} {c0_key}->{c1_key}: never saw ShowingName");

            let h0 = find_chip(&fix.content, c0_key);
            let h1 = find_chip(&fix.content, c1_key);

            for (side, handle, is_trap) in [(0u8, h0, c0_trap), (1u8, h1, c1_trap)] {
                for (consoles, view) in [(Consoles::One, 0u8), (Consoles::One, 1u8), (Consoles::Both, 0u8)] {
                    let remote = view != side;
                    let (s0, s4) = find_telop_step_ticks(&trace, view, remote);
                    let want = if is_trap && consoles == Consoles::One && remote {
                        TelopName::Hidden
                    } else {
                        TelopName::Chip(handle)
                    };
                    if let (Some(i0), Some(i4)) = (s0, s4) {
                        for enh in [Enhancements::NONE, Enhancements::ALL] {
                            check_telop_snapshot(
                                fix,
                                &trace,
                                i0,
                                consoles,
                                view,
                                "ja",
                                enh,
                                Size { width: 240, height: 160 },
                                want,
                                remote,
                                &format!("cutin:{c0_key}->{c1_key}:s{side}:step0"),
                                &mut failures,
                            );
                            check_telop_snapshot(
                                fix,
                                &trace,
                                i4,
                                consoles,
                                view,
                                "zh-Hans",
                                enh,
                                Size { width: 240, height: 160 },
                                want,
                                remote,
                                &format!("cutin:{c0_key}->{c1_key}:s{side}:step4"),
                                &mut failures,
                            );
                        }
                    } else {
                        failures.push(format!("{game} cut-in {c0_key}->{c1_key} side={side} view={view}: missing telop s0={s0:?} s4={s4:?}"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "matrix_b failures ({}):\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn matrix_c_trap_springs() {
    let mut failures = Vec::new();
    for game in ["exe4", "exe5", "exe6"] {
        let fix = fixture(game);

        // 1. AntiRecv sprung by Recov10 and by Roll
        for spring_key in ["recov10", "roll"] {
            let (_, setup) = base_setup(fix, &["antirecv"], &[spring_key], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);
            wait_until_ready(&mut b, 1, &mut trace);
            let armed_end = trace.len();
            press_cast(&mut b, 1, keys::A, &mut trace);
            wait_ticks(&mut b, 120, &mut trace);

            let antirecv = find_chip(&fix.content, "antirecv");
            let spring_trace = &trace[armed_end..];
            for (consoles, view) in [(Consoles::One, 0u8), (Consoles::One, 1u8), (Consoles::Both, 0u8)] {
                // Note: AntiRecv counterattack runs on side 1 (healer side), so remote for view=0 is true, remote for view=1 is false
                let (s0, s4) = find_telop_step_ticks(spring_trace, view, view != 1);
                if let (Some(i0), Some(i4)) = (s0, s4) {
                    for enh in [Enhancements::NONE, Enhancements::ALL] {
                        check_telop_snapshot(
                            fix,
                            spring_trace,
                            i0,
                            consoles,
                            view,
                            "ja",
                            enh,
                            Size { width: 240, height: 160 },
                            TelopName::Chip(antirecv),
                            view != 1,
                            &format!("antirecv-{spring_key}:step0"),
                            &mut failures,
                        );
                        check_telop_snapshot(
                            fix,
                            spring_trace,
                            i4,
                            consoles,
                            view,
                            "zh-Hans",
                            enh,
                            Size { width: 240, height: 160 },
                            TelopName::Chip(antirecv),
                            view != 1,
                            &format!("antirecv-{spring_key}:step4"),
                            &mut failures,
                        );
                    }
                } else {
                    failures.push(format!("{game} antirecv+{spring_key} view={view}: spring telop missing (s0={s0:?}, s4={s4:?})"));
                }
            }
        }

        // 2. AntiNavi sprung by Navi chip (standalone and cut-in)
        let navi_key = match game {
            "exe4" => "fireman",
            "exe5" => "protoman",
            _ => "heatman",
        };
        for as_cut_in in [false, true] {
            let (_, setup) = base_setup(fix, &["antinavi"], &[navi_key], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            let antinavi = find_chip(&fix.content, "antinavi");
            let spring_start;
            if !as_cut_in {
                press_cast(&mut b, 0, keys::A, &mut trace);
                wait_until_ready(&mut b, 1, &mut trace);
                spring_start = trace.len();
                press_cast(&mut b, 1, keys::A, &mut trace);
                wait_ticks(&mut b, 160, &mut trace);
            } else {
                press_cast(&mut b, 1, keys::A, &mut trace);
                let mut cut = false;
                for _ in 0..80 {
                    let mut p = [PlayerTick::default(); 2];
                    if !cut && b.is_dimmed() && b.dimming[1].state == DimmingState::ShowingName {
                        p[0].held = keys::A;
                        cut = true;
                    }
                    b.tick(&p, TickEvents::default());
                    trace.push(b.clone());
                }
                wait_ticks(&mut b, 360, &mut trace);
                spring_start = trace
                    .iter()
                    .position(|s| s.linked[0].chip == Some(antinavi))
                    .unwrap_or_else(|| panic!("{game} cut-in antinavi never armed"));
            }
            let navi_chip = find_chip(&fix.content, navi_key);
            let slice = &trace[spring_start..];
            for (want_chip, tag) in [(antinavi, "antinavi-spring"), (navi_chip, "turned-navi")] {
                let found_idx = slice.iter().enumerate().find_map(|(i, s)| {
                    let mut c = s.clone();
                    c.setup.local_side = 0;
                    c.telops_for(0).into_iter().flatten().find(|(t, blk)| !t.remote && t.name == TelopName::Chip(want_chip) && blk.step == 4).map(|_| i)
                });
                if let Some(i) = found_idx {
                    check_telop_snapshot(
                        fix,
                        slice,
                        i,
                        Consoles::One,
                        0,
                        "ja",
                        Enhancements::ALL,
                        Size { width: 240, height: 160 },
                        TelopName::Chip(want_chip),
                        false,
                        &format!("{tag}(cutin={as_cut_in}):view0"),
                        &mut failures,
                    );
                    check_telop_snapshot(
                        fix,
                        slice,
                        i,
                        Consoles::One,
                        1,
                        "zh-Hans",
                        Enhancements::ALL,
                        Size { width: 240, height: 160 },
                        TelopName::Chip(want_chip),
                        true,
                        &format!("{tag}(cutin={as_cut_in}):view1"),
                        &mut failures,
                    );
                } else {
                    failures.push(format!("{game} antinavi cutin={as_cut_in}: missing telop for {tag}"));
                }
            }
        }

        // 3. Elemental trap (antifire in EXE4/5, elemtrap in EXE6) sprung by fire attack
        let (etrap_key, fire_key) = match game {
            "exe4" => ("antifire", "heatshot"),
            "exe5" => ("antifire", "firehit1"),
            _ => ("elemtrap", "firehit1"),
        };
        {
            let (_, setup) = base_setup(fix, &[etrap_key], &[fire_key], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);
            wait_until_ready(&mut b, 1, &mut trace);
            let spring_start = trace.len();
            press_cast(&mut b, 1, keys::A, &mut trace);
            wait_ticks(&mut b, 120, &mut trace);
            let etrap = find_chip(&fix.content, etrap_key);
            let s = &trace[spring_start..];
            let (s0, s4) = find_telop_step_ticks(s, 0, false);
            if let (Some(i0), Some(i4)) = (s0, s4) {
                check_telop_snapshot(fix, s, i0, Consoles::One, 0, "ja", Enhancements::NONE, Size { width: 240, height: 160 }, TelopName::Chip(etrap), false, &format!("{etrap_key}-spring:step0"), &mut failures);
                check_telop_snapshot(fix, s, i4, Consoles::One, 0, "ja", Enhancements::ALL, Size { width: 240, height: 160 }, TelopName::Chip(etrap), false, &format!("{etrap_key}-spring:step4-ja-all"), &mut failures);
                check_telop_snapshot(fix, s, i4, Consoles::One, 1, "zh-Hans", Enhancements::ALL, Size { width: 240, height: 160 }, TelopName::Chip(etrap), true, &format!("{etrap_key}-spring:step4"), &mut failures);
            } else {
                failures.push(format!("{game} {etrap_key}+{fire_key}: elemental trap did not spring cleanly (s0={s0:?}, s4={s4:?})"));
            }
        }

        // 4. AntiDmg sprung by fire attack -> used_chips
        {
            let (_, setup) = base_setup(fix, &["antidmg"], &[fire_key], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);
            wait_until_ready(&mut b, 1, &mut trace);
            let spring_start = trace.len();
            press_cast(&mut b, 1, keys::A, &mut trace);
            wait_ticks(&mut b, 80, &mut trace);
            verify_used_chip_pixels(fix, &trace[spring_start..], 0, find_chip(&fix.content, "antidmg"), &mut failures);
        }

        // 5. AntiSwrd sprung by sword after moving navis adjacent
        {
            let (_, setup) = base_setup(fix, &["antiswrd"], &["sword"], false);
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);
            wait_until_ready(&mut b, 1, &mut trace);
            step_panel_to(&mut b, 0, 3, keys::RIGHT, &mut trace);
            step_panel_to(&mut b, 1, 4, keys::RIGHT, &mut trace);
            let spring_start = trace.len();
            press_cast(&mut b, 1, keys::A, &mut trace);
            wait_ticks(&mut b, 80, &mut trace);
            verify_used_chip_pixels(fix, &trace[spring_start..], 0, find_chip(&fix.content, "antiswrd"), &mut failures);
        }

        // 6. EXE6 BodyGrd spring -> used_chips
        if game == "exe6" {
            let mut setup = base_setup(fix, &["cannon"], &[fire_key], false).1;
            let an = find_chip(&fix.content, "antinavi");
            let asw = find_chip(&fix.content, "antiswrd");
            let ad = find_chip(&fix.content, "antidmg");
            let star = saito_battle::content::ChipCode::ASTERISK;
            setup.players[0].folder.chips[0] = Some(FolderChip::new(an, star));
            setup.players[0].folder.chips[1] = Some(FolderChip::new(asw, star));
            setup.players[0].folder.chips[2] = Some(FolderChip::new(ad, star));
            let mut b = Battle::new(setup, fix.content.clone());
            step_into_fight_with_picks(&mut b, &[0, 1, 2], &[0]);
            let mut trace = Vec::new();
            wait_ticks(&mut b, 2, &mut trace);
            press_cast(&mut b, 0, keys::A, &mut trace);
            wait_until_ready(&mut b, 1, &mut trace);
            let spring_start = trace.len();
            press_cast(&mut b, 1, keys::A, &mut trace);
            wait_ticks(&mut b, 80, &mut trace);
            verify_used_chip_pixels(fix, &trace[spring_start..], 0, find_chip(&fix.content, "bodygrd"), &mut failures);
        }
    }

    assert!(failures.is_empty(), "matrix_c failures ({}):\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn matrix_d_viewer_replay_and_live_spectate() {
    let mut failures = Vec::new();
    for game in ["exe4", "exe5", "exe6"] {
        let fix = fixture(game);
        let mut m = saito_match::pick::live(&fix.content, game, 7, None).expect("live match");
        m.rounds.truncate(1);
        m.seed = Some(7);
        let ad = folder_chip(&fix.content, "antidmg");
        let mut folder = m.sides[0].folder(&fix.content);
        for slot in &mut folder.chips {
            if let Some(c) = slot
                && c.id == ad.id
            {
                *c = folder_chip(&fix.content, "cannon");
            }
        }
        folder.chips[0] = Some(ad);
        folder.regular = Some(0);
        m.sides[0].set_folder(&fix.content, &folder).expect("set folder");
        let m = m.stated(&fix.content, 7).expect("stated match");

        let match_bytes = saito_match::binary::match_bytes(&fix.content, &m).expect("match bytes");
        let start = Set::of(&fix.content, &m, 7).start();
        let head = Head {
            engine: saito_frontend::replay::ENGINE.to_string(),
            game: game.to_string(),
            content: fix.content.hash().0,
            start: start.digest(),
        };
        let info = Info::of(MatchKind::Online, 0);
        let mut buf = Vec::new();
        let writer = Writer::new(&mut buf, &head, &match_bytes, &info).expect("writer new");
        let header_len = writer.into_inner().len();
        // Rebuild writer writing to `raw`:
        let mut raw = Vec::new();
        let mut writer = Writer::new(&mut raw, &head, &match_bytes, &info).expect("writer");

        let mut b = start;
        let mut telop_tick: Option<u32> = None;
        let mut casted = false;
        let mut ok_sent = false;
        let mut settle = 0u32;

        for tick_no in 0..400u32 {
            let mut p = [PlayerTick::default(); 2];
            if b.round.mode == mode::CUSTOM {
                if !ok_sent {
                    if b.custom.sides[0].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing)
                        && b.custom.sides[1].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing)
                    {
                        if b.custom.sides[0].screen.as_ref().unwrap().selected == 0 {
                            p[0].choice = Some(ScreenChoice::Pick(0));
                        } else {
                            p[0].choice = Some(ScreenChoice::Ok);
                            p[1].choice = Some(ScreenChoice::Ok);
                            ok_sent = true;
                        }
                    } else if b.custom.sides[0].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing) {
                        if b.custom.sides[0].screen.as_ref().unwrap().selected == 0 {
                            p[0].choice = Some(ScreenChoice::Pick(0));
                        } else {
                            p[0].choice = Some(ScreenChoice::Ok);
                        }
                    } else if b.custom.sides[1].screen.as_ref().is_some_and(|s| s.phase == Phase::Choosing) {
                        p[1].choice = Some(ScreenChoice::Ok);
                    }
                }
            } else if b.round.mode == mode::FIGHTING && !b.banner.active && !b.is_dimmed() && is_controllable(&b, 0) {
                if !casted {
                    settle += 1;
                    if settle >= 4 {
                        p[0].held = keys::A;
                        casted = true;
                    }
                }
            }

            b.tick(&p, TickEvents::default());
            if telop_tick.is_none()
                && b.telops_for(0).into_iter().flatten().any(|(t, blk)| !t.remote && blk.step == 4 && blk.timer >= 3)
            {
                telop_tick = Some(tick_no + 1);
            }
            let ended = tick_no == 399;
            let digest = if ended || (tick_no + 1) % 60 == 0 { Some(b.digest()) } else { None };
            writer
                .tick(&Tick {
                    inputs: [p[0].word(), p[1].word()],
                    delays: [0, 0],
                    digest,
                    round_ended: ended,
                    set_ended: ended,
                })
                .expect("write tick");
        }
        drop(writer);
        let telop_pos = telop_tick.unwrap_or_else(|| panic!("{game} replay test did not produce telop"));

        let replay = Replay::read(&raw).expect("read replay");
        let loaded_ja = loaded_for(fix, "ja", Enhancements::NONE);
        let loaded_zh_all = loaded_for(fix, "zh-Hans", Enhancements::ALL);
        let mut viewers: Vec<(&str, Viewer)> = vec![
            ("replay", Viewer::open(&loaded_ja, &replay, Some(0)).expect("open replay")),
        ];

        let header = saito_replay::Header::read(&raw[..header_len]).expect("read header");
        let mut live_v = Viewer::open_live(&loaded_ja, header, Some(0), None).expect("open live");
        let chunks = TickDecoder::new().decode_chunk(&raw[header_len..]).expect("decode ticks");
        live_v.push_ticks(&chunks);
        viewers.push(("live_spectate", live_v));

        for (mode_name, v) in &mut viewers {
            v.seek(telop_pos);
            let mut blank_b = v.battle().clone();
            blank_b.banner = Default::default();
            blank_b.remote_telop = Default::default();
            let mut blank_r_ja = renderer_for(fix, Consoles::One, "ja", Enhancements::NONE, Size { width: BASE_WIDTH, height: BASE_HEIGHT });
            blank_r_ja.observe(&blank_b);
            let blank_ja_none = render_rgba(fix, &mut blank_r_ja, &blank_b, None);

            let mut blank_r_zh = renderer_for(fix, Consoles::One, "zh-Hans", Enhancements::ALL, Size { width: BASE_WIDTH, height: BASE_HEIGHT });
            blank_r_zh.observe(&blank_b);
            let zh_font = saito_render::vfont::bundled_font(Cut::Simplified).clone();
            let blank_zh_all = render_rgba(fix, &mut blank_r_zh, &blank_b, Some(&zh_font));

            // Check view=0, view=1, both=true across NONE (ja) & zh-Hans ALL
            for &(view, both, use_zh_all) in &[
                (0u8, false, false),
                (1u8, false, false),
                (0u8, true, false),
                (0u8, true, true),
                (0u8, false, true),
                (1u8, false, true),
            ] {
                let lang = if use_zh_all { "zh-Hans" } else { "ja" };
                if use_zh_all {
                    v.set_language(&loaded_zh_all.graphics);
                    v.set_enhancements(loaded_zh_all.enhancements, loaded_zh_all.font.clone());
                } else {
                    v.set_language(&loaded_ja.graphics);
                    v.set_enhancements(loaded_ja.enhancements, None);
                }
                assert!(v.set_view(view));
                v.set_both(both);
                let mut out = vec![0u32; BASE_WIDTH * BASE_HEIGHT];
                v.present(&mut out, BASE_WIDTH, BASE_HEIGHT);
                let x_range = if view == 1 && !both { 100..BASE_WIDTH } else { 0..140 };
                let blank_img = if use_zh_all { &blank_zh_all } else { &blank_ja_none };
                let diffs = count_diff_in_box(&out, blank_img, BASE_WIDTH, x_range.start, x_range.end, 24, 56);
                if diffs == 0 {
                    failures.push(format!("{game} {mode_name}: 0 diff pixels for view={view} both={both} zh_all={use_zh_all}"));
                }
                let probs = telop_problem_lines(v.renderer(), lang);
                if !probs.is_empty() {
                    failures.push(format!("{game} {mode_name}: problems={probs:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "matrix_d failures ({}):\n{}", failures.len(), failures.join("\n"));
}
