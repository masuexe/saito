//! A match in binary: what a replay keeps of the match it plays, and what
//! a netplay peer reveals of its side (the handshake names the rounds the
//! players agree by name: saito-frontend's `lobby`). It is read against the content it was written
//! on, which whoever carries the bytes names beside them (a replay's head,
//! the netplay handshake: the game and the content's hash), and nothing in
//! it is a name but a round's background: a definition is its handle in the
//! content (its index there, the same on every load of the content the
//! hash names), and a background, which may be any game's (the background
//! pool's, docs/app.md §1, "The battle's background"), is its game and its
//! name in its game, which name the same background whatever the
//! pool beside the content (no player's words: a match holds no other
//! text).
//!
//! ```text
//! place  := stage (u16: its handle), background (string: `<game>:<name>`, its game and its
//!           name in its game)
//! side   := the side's setup of the game's rules in its compact form (`Block::write_compact`:
//!           each fact in the setup's order, a list as its count and its entries)
//! match  := seed (u32), the round wins its set is first to (u8: `n`, 1 to `MAX_FIRST_TO`),
//!           every one of its `2n − 1` rounds' places (as the seed picks those the match
//!           leaves: `Match::places`), the custom screen's time limit (u16: seconds, 0 for none:
//!           `Match::custom_screen_time`), side 0, side 1
//! string := length (LEB128), UTF-8
//! ```
//!
//! Numbers are little-endian. A place's layout is the engine's (a round is
//! fought on a stage and a background, whatever the game); a side's
//! is its game's rules' setup, so a game's facts change no other game's
//! bytes. A reader refuses bytes that end early or run on, a handle past
//! the content's definitions, a background neither the game nor the
//! pool has, and a value no fact may hold, each with where; whether what it
//! read is a match the game's rules allow is the checks' (`check`), as for
//! a match file.

use crate::facts::Facts;
use crate::{Match, RoundSettings, Side, ids};
use saito_battle::content::{BackgroundId, Content};
use saito_content_api::{Block, FieldType, FieldValue, StageHandle};


/// A side, appended to `out`: its facts' block, compact.
pub fn put_side(content: &Content, side: &Side, out: &mut Vec<u8>) {
    let rules = content.defs.rules().expect("a match's content has rules");
    let block = side.facts.block().expect("a side of a game with rules has its facts");
    block.write_compact(content.defs.schema(rules.setup), out);
}

/// A side from the start of `bytes`, and how many bytes it took.
pub fn take_side(content: &Content, bytes: &[u8]) -> Result<(Side, usize), String> {
    let rules = content.defs.rules().ok_or_else(|| format!("{} has no rules", content.game()))?;
    let schema = content.defs.schema(rules.setup);
    let (block, used) = Block::read_compact(rules.setup, schema, bytes)?;
    for i in 0..schema.fields().len() {
        handles_held(content, &block, schema.place(i), &schema.field(i).name)?;
    }
    Ok((Side { facts: Facts::of_block(block) }, used))
}

/// That every definition the block holds at `place` (at any depth) is one
/// of the content's.
fn handles_held(content: &Content, block: &Block, place: saito_content_api::Place, path: &str) -> Result<(), String> {
    match place.ty() {
        FieldType::Record(fields) => {
            for i in 0..fields.fields().len() {
                handles_held(content, block, place.field_at(i), &format!("{path}.{}", fields.field(i).name))?;
            }
        }
        FieldType::List(..) | FieldType::Array(..) => {
            for k in 0..block.len_at(place).expect("a list or an array") {
                handles_held(content, block, place.elem(k).expect("an element it holds"), &format!("{path}[{}]", k + 1))?;
            }
        }
        FieldType::Ref(registry, _) => {
            if let FieldValue::Ref(Some((r, h))) = block.get_at(place)
                && ids::key_of(content, r, h).is_none()
            {
                return Err(format!("{path}: no {registry} {h} in the content"));
            }
        }
        FieldType::Asset(kind) => {
            if let FieldValue::Asset(_, Some(h)) = block.get_at(place)
                && h as usize >= content.assets.names(*kind).len()
            {
                return Err(format!("{path}: no {kind} {h} in the content"));
            }
        }
        _ => {}
    }
    Ok(())
}

/// What the set of `rounds` rounds is first to, appended to `out`: refused
/// for a count no set has.
fn put_first_to(rounds: usize, out: &mut Vec<u8>) -> Result<(), String> {
    out.push(crate::first_to(rounds).ok_or_else(|| crate::check::no_set(rounds))?);
    Ok(())
}

/// The rounds of the set, from what it is first to at the start of `bytes`.
fn take_rounds(bytes: &[u8]) -> Result<usize, String> {
    match bytes.first() {
        None => Err("first to: the bytes end inside it".to_string()),
        Some(&n) if n == 0 || n > crate::MAX_FIRST_TO => Err(format!("first to {n}: a set is first to 1 to {}", crate::MAX_FIRST_TO)),
        Some(&n) => Ok(crate::rounds_for(n)),
    }
}

/// A stage's handle from the start of `bytes`, one of the content's.
fn take_stage(content: &Content, bytes: &[u8], at: &str) -> Result<StageHandle, String> {
    let b = bytes.get(..2).ok_or_else(|| format!("{at}: the bytes end inside it"))?;
    let stage = u16::from_le_bytes([b[0], b[1]]);
    if stage as usize >= content.defs.stages.len() {
        return Err(format!("{at}: no stage {stage} in the content"));
    }
    Ok(StageHandle(stage))
}

/// A string, appended to `out`: its length (LEB128), then its UTF-8.
fn put_string(s: &str, out: &mut Vec<u8>) {
    let mut n = s.len();
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out.extend_from_slice(s.as_bytes());
}

/// A string from the start of `bytes`, and how many bytes it took.
fn take_string<'b>(bytes: &'b [u8], at: &str) -> Result<(&'b str, usize), String> {
    let ended = || format!("{at}: the bytes end inside it");
    let (mut n, mut used) = (0usize, 0usize);
    loop {
        let b = *bytes.get(used).ok_or_else(ended)?;
        if used >= 4 {
            return Err(format!("{at}: a string's length past any"));
        }
        n |= ((b & 0x7F) as usize) << (7 * used);
        used += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    let text = bytes.get(used..used + n).ok_or_else(ended)?;
    let text = std::str::from_utf8(text).map_err(|_| format!("{at}: a string that isn't UTF-8"))?;
    Ok((text, used + n))
}

/// Background `id` as a place holds it: its game and its name in its game
/// (`<game>:<name>`).
fn background_key(content: &Content, id: BackgroundId) -> Result<String, String> {
    content.background_of(id).map(|(game, name)| saito_battle::content::pool_key(game, name)).ok_or_else(|| format!("no background {} in the content", id.0))
}

/// A place's background from the start of `bytes`: the name a match
/// writes for it (`ids::background_name`), and how many bytes it took.
fn take_background(content: &Content, bytes: &[u8], at: &str) -> Result<(String, usize), String> {
    let (key, used) = take_string(bytes, at)?;
    let id = key.split_once(':').and_then(|(game, name)| content.background_handle(game, name));
    let name = id.and_then(|id| ids::background_name(content, id)).ok_or_else(|| format!("{at}: no background {key:?} in the background pool"))?;
    Ok((name.to_string(), used))
}

/// A match in binary: its seed (refused: a match that states none), what
/// its set is first to, every round's place (each part the match leaves
/// picked from the seed), its custom screen's time limit, its sides.
pub fn match_bytes(content: &Content, m: &Match) -> Result<Vec<u8>, String> {
    let seed = m.seed.ok_or("a match kept in binary states its seed")?;
    let mut out = seed.to_le_bytes().to_vec();
    let places = m.places(content, seed)?;
    put_first_to(places.len(), &mut out)?;
    for (i, p) in places.iter().enumerate() {
        out.extend_from_slice(&p.stage.0.to_le_bytes());
        put_string(&background_key(content, p.background).map_err(|e| format!("round {}: {e}", i + 1))?, &mut out);
    }
    out.extend_from_slice(&m.custom_screen_time.unwrap_or(0).to_le_bytes());
    for side in &m.sides {
        put_side(content, side, &mut out);
    }
    if let Some(rank) = m.random_battle {
        out.push(b'R');
        out.push(rank.as_u8());
    }
    if m.abd_mode != crate::AbdMode::Original {
        out.push(b'A');
        out.push(m.abd_mode.as_u8());
    }
    if let Some(cap) = m.abd_chip_cap {
        out.push(b'K');
        out.push(cap);
    }
    if !m.cs_prob_fix.is_empty() {
        out.push(b'C');
        out.extend_from_slice(&m.cs_prob_fix.bits().to_le_bytes());
    }
    if m.all_guard_disabled {
        out.push(b'G');
        out.push(1);
    }
    if m.dark_cut_in_fix {
        out.push(b'D');
        out.push(1);
    }
    if !m.sun_enabled {
        out.push(b'S');
        out.push(0);
    }
    if m.ds_shout_enabled || m.ds_shout_max_only {
        out.push(b'M');
        out.push((m.ds_shout_enabled as u8) | ((m.ds_shout_max_only as u8) << 1));
    }
    Ok(out)
}

/// A match from all of `bytes`: every round's place stated.
pub fn read_match(content: &Content, bytes: &[u8]) -> Result<Match, String> {
    let seed = bytes.get(..4).ok_or("seed: the bytes end inside it")?;
    let seed = u32::from_le_bytes(seed.try_into().expect("four bytes"));
    let mut at = 4;
    let count = take_rounds(&bytes[at..])?;
    at += 1;
    let mut rounds = Vec::with_capacity(count);
    for i in 0..count {
        let round = format!("round {}", i + 1);
        let stage = take_stage(content, &bytes[at..], &round)?;
        let (background, used) = take_background(content, &bytes[at + 2..], &round)?;
        at += 2 + used;
        rounds.push(RoundSettings { stage: Some(stage), background: Some(background) });
    }
    let time = bytes.get(at..at + 2).ok_or("custom screen time: the bytes end inside it")?;
    let custom_screen_time = Some(u16::from_le_bytes([time[0], time[1]])).filter(|&s| s != 0);
    if let Some(problem) = crate::check_custom_screen_time(custom_screen_time) {
        return Err(problem);
    }
    at += 2;
    let mut side = |name: &str| -> Result<Side, String> {
        let (s, used) = take_side(content, &bytes[at..]).map_err(|e| format!("{name}: {e}"))?;
        at += used;
        Ok(s)
    };
    let sides = [side("left")?, side("right")?];
    let mut random_battle = None;
    let mut abd_mode = crate::AbdMode::Original;
    let mut abd_chip_cap = None;
    let mut cs_prob_fix = crate::CsProbFix::NONE;
    let mut all_guard_disabled = false;
    let mut dark_cut_in_fix = false;
    let mut sun_enabled = true;
    let mut ds_shout_enabled = false;
    let mut ds_shout_max_only = false;
    let mut seen_random = false;
    let mut seen_abd = false;
    let mut seen_abd_cap = false;
    let mut seen_cs_prob_fix = false;
    let mut seen_all_guard = false;
    let mut seen_dark_cut_in = false;
    let mut seen_sun = false;
    let mut seen_ds_shout = false;
    while bytes.len() - at >= 2 {
        match bytes[at] {
            b'R' if !seen_random => {
                let rank = crate::RandomRank::from_u8(bytes[at + 1])
                    .ok_or_else(|| format!("random battle: unknown rank {}", bytes[at + 1]))?;
                random_battle = Some(rank);
                seen_random = true;
                at += 2;
            }
            b'A' if !seen_abd => {
                let mode = crate::AbdMode::from_u8(bytes[at + 1])
                    .ok_or_else(|| format!("dark soul ai: unknown mode {}", bytes[at + 1]))?;
                if !mode.allowed_for(content.game()) {
                    return Err(format!("dark soul ai: {mode:?} is not supported for {}", content.game()));
                }
                abd_mode = mode;
                seen_abd = true;
                at += 2;
            }
            b'K' if !seen_abd_cap => {
                let cap = bytes[at + 1];
                if !crate::abd_mode::chip_cap_allowed(content.game(), Some(cap)) {
                    return Err(format!("abd chip cap: {cap} is not supported for {}", content.game()));
                }
                abd_chip_cap = Some(cap);
                seen_abd_cap = true;
                at += 2;
            }
            b'C' if !seen_cs_prob_fix && bytes.len() - at >= 3 => {
                let fix = crate::CsProbFix::from_bits(u16::from_le_bytes([bytes[at + 1], bytes[at + 2]]));
                if !fix.allowed_for(content.game()) {
                    return Err(format!("cs prob fix: {:#06x} is not supported for {}", fix.bits(), content.game()));
                }
                cs_prob_fix = fix;
                seen_cs_prob_fix = true;
                at += 3;
            }
            b'G' if !seen_all_guard => {
                if !crate::capabilities::for_game(content.game()).all_guard_toggle {
                    return Err(format!("all guard option is not supported for {}", content.game()));
                }
                all_guard_disabled = bytes[at + 1] != 0;
                seen_all_guard = true;
                at += 2;
            }
            b'D' if !seen_dark_cut_in => {
                if !crate::capabilities::for_game(content.game()).dark_cut_in_fix {
                    return Err(format!("dark cut-in option is not supported for {}", content.game()));
                }
                dark_cut_in_fix = bytes[at + 1] != 0;
                seen_dark_cut_in = true;
                at += 2;
            }
            b'S' if !seen_sun => {
                if !crate::capabilities::for_game(content.game()).ds_sun_toggle {
                    return Err(format!("sun option is not supported for {}", content.game()));
                }
                sun_enabled = bytes[at + 1] != 0;
                seen_sun = true;
                at += 2;
            }
            b'M' if !seen_ds_shout => {
                if !crate::capabilities::for_game(content.game()).ds_shout_toggle {
                    return Err(format!("ds shout option is not supported for {}", content.game()));
                }
                let bits = bytes[at + 1];
                ds_shout_enabled = (bits & 1) != 0;
                ds_shout_max_only = (bits & 2) != 0;
                seen_ds_shout = true;
                at += 2;
            }
            _ => break,
        }
    }
    finished(bytes, at)?;
    Ok(Match { game: content.game().to_string(), seed: Some(seed), rounds, sides, backgrounds: crate::BackgroundPick::Game, custom_screen_time, random_battle, abd_mode, abd_chip_cap, cs_prob_fix, all_guard_disabled, dark_cut_in_fix, sun_enabled, ds_shout_enabled, ds_shout_max_only })
}


/// That `at` is the end of `bytes`.
fn finished(bytes: &[u8], at: usize) -> Result<(), String> {
    match bytes.len() - at {
        0 => Ok(()),
        n => Err(format!("{n} bytes after its end")),
    }
}

/// A side alone in binary (what a netplay peer reveals).
pub fn side_bytes(content: &Content, side: &Side) -> Vec<u8> {
    let mut out = Vec::new();
    put_side(content, side, &mut out);
    out
}

/// A side from all of `bytes`.
pub fn read_side(content: &Content, bytes: &[u8]) -> Result<Side, String> {
    let (side, used) = take_side(content, bytes)?;
    finished(bytes, used)?;
    Ok(side)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};

    /// A side's facts are numbers, choices and definitions, never text: a
    /// fact's type is one of these, and none of them holds a string. So a
    /// match in binary (in a replay, in a netplay offer) has nowhere a
    /// build's name could be. (A new type that holds text doesn't compile
    /// here until this is looked at.)
    #[test]
    fn no_fact_holds_text() {
        fn holds_text(ty: &FieldType) -> bool {
            match ty {
                FieldType::Bool | FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::I8 | FieldType::I16 | FieldType::I32 => false,
                FieldType::Object | FieldType::Vec3 | FieldType::OptionalU8 | FieldType::Code => false,
                // (Stored as its variant's index; a definition and an asset
                // as their handles.)
                FieldType::Enum(_) | FieldType::Ref(..) | FieldType::Asset(_) => false,
                FieldType::Array(element, _) | FieldType::List(element, _) => holds_text(element),
                FieldType::Record(fields) => fields.fields().iter().any(|f| holds_text(&f.ty)),
            }
        }
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5"), (crate::testing::exe4_content(), "exe4")] {
            let facts = crate::facts::fields(&content);
            assert!(!facts.is_empty(), "{game}");
            for fact in facts {
                assert!(!holds_text(fact.ty), "{game}: the fact {} holds text", fact.name);
            }
        }
    }

    /// Live play's random matches of both games written and read back: the
    /// same match, the same bytes again, a match the checks pass; and on
    /// another load of the content, the same bytes (a handle is the content's
    /// index, the same on every load of content of one hash).
    #[test]
    fn a_match_reads_back_from_its_bytes() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            for seed in [1, 77, 4242] {
                let m = crate::pick::live(&content, game, seed, None).unwrap();
                let bytes = match_bytes(&content, &m).unwrap();
                let back = read_match(&content, &bytes).unwrap_or_else(|e| panic!("{game} {seed}: {e}"));
                assert_eq!(back, m, "{game} {seed}");
                assert_eq!(match_bytes(&content, &back).unwrap(), bytes, "{game} {seed}");
                assert_eq!(crate::check_match(&content, &back), Vec::<String>::new(), "{game} {seed}");
            }
        }
        let again = std::sync::Arc::new(crate::testing::defined(&["exe6"]).unwrap());
        assert_eq!(again.hash(), exe6_content().hash());
        let of = |c: &std::sync::Arc<Content>| match_bytes(c, &crate::pick::live(c, "exe6", 77, None).unwrap()).unwrap();
        assert_eq!(of(&again), of(&exe6_content()));
    }

    /// A random side of each game reads back alone, and fits one datagram
    /// with room to spare, EXE5's auto battle data and all.
    #[test]
    fn a_side_fits_a_datagram() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let m = crate::pick::live(&content, game, 9, None).unwrap();
            let bytes = side_bytes(&content, &m.sides[0]);
            assert!(bytes.len() < 1200, "{game}: {} bytes", bytes.len());
            assert_eq!(read_side(&content, &bytes).unwrap(), m.sides[0], "{game}");
            let mut more = bytes.clone();
            more.push(0);
            assert_eq!(read_side(&content, &more).unwrap_err(), "1 bytes after its end");
        }
    }

    /// A match of five rounds, some parts stated and the rest left to the
    /// seed, is kept with every round's place: it reads back as the match
    /// with them all stated, which starts the same round and keeps the same
    /// bytes.
    #[test]
    fn a_match_keeps_every_rounds_place() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 21, None).unwrap();
        m.rounds = vec![RoundSettings::default(), m.rounds[1].clone(), RoundSettings { stage: m.rounds[0].stage, background: None }, RoundSettings::default(), RoundSettings::default()];
        let bytes = match_bytes(&content, &m).unwrap();
        let back = read_match(&content, &bytes).unwrap();
        assert_eq!(back, m.stated(&content, 21).unwrap());
        assert!(back.rounds.iter().all(|r| r.stage.is_some() && r.background.is_some()));
        assert_eq!(back.places(&content, 0).unwrap(), m.places(&content, 21).unwrap(), "stated: no seed picks them");
        assert_eq!(format!("{:?}", back.round(&content, 21)), format!("{:?}", m.round(&content, 21)));
        assert_eq!(match_bytes(&content, &back).unwrap(), bytes);
        assert_eq!(bytes[4], 3, "first to 3");
    }

    /// A match keeps its custom screen's time limit, or that it has none,
    /// and its rounds count the same; one a match can't have is refused.
    #[test]
    fn a_match_keeps_its_custom_screen_time() {
        let content = exe6_content();
        let live = crate::pick::live(&content, "exe6", 21, None).unwrap();
        for time in [Some(60), Some(120), None] {
            let m = Match { custom_screen_time: time, ..live.clone() };
            let bytes = match_bytes(&content, &m).unwrap();
            let back = read_match(&content, &bytes).unwrap();
            assert_eq!(back.custom_screen_time, time);
            assert_eq!(back.round(&content, 21).custom_screen_limit, time.map(|t| t * 60));
            assert_eq!(match_bytes(&content, &back).unwrap(), bytes);
        }
        let none = match_bytes(&content, &live).unwrap();
        let mut odd = match_bytes(&content, &Match { custom_screen_time: Some(60), ..live.clone() }).unwrap();
        let at = (0..none.len()).find(|&i| none[i] != odd[i]).expect("the limit's bytes");
        odd[at] = 7;
        assert_eq!(read_match(&content, &odd).unwrap_err(), "custom screen time 7: a match's is 30, 45, 60, 90, 120 seconds, or none");
    }

    /// A match whose rounds are drawn on other games' backgrounds (the pool's: one stated, the rest picked from
    /// the pool) keeps each by its game and its name, and reads back as the match with them stated, which starts
    /// the same round: on another load of the content too.
    #[test]
    fn a_match_keeps_another_games_background() {
        let content = crate::testing::exe5_content();
        let mut m = crate::pick::live(&content, "exe5", 9, None).unwrap();
        m.backgrounds = crate::BackgroundPick::Pool;
        m.rounds = vec![RoundSettings { stage: None, background: Some("exe4:acdc-area".into()) }, RoundSettings::default(), RoundSettings::default()];
        let bytes = match_bytes(&content, &m).unwrap();
        let back = read_match(&content, &bytes).unwrap();
        assert_eq!(back.rounds[0].background.as_deref(), Some("exe4:acdc-area"));
        assert_eq!(back, Match { backgrounds: crate::BackgroundPick::Game, ..m.stated(&content, 9).unwrap() });
        assert_eq!(format!("{:?}", back.round(&content, 9)), format!("{:?}", m.round(&content, 9)));
        let key = |at: usize| String::from_utf8(bytes[at + 3..at + 3 + bytes[at + 2] as usize].to_vec()).unwrap();
        assert_eq!(key(5), "exe4:acdc-area", "by its game and its name");
        let again = std::sync::Arc::new(crate::testing::defined(&["exe5"]).unwrap());
        assert_eq!(read_match(&again, &bytes).unwrap(), back, "another load");
        // (A pool that hasn't it refuses it, saying which.)
        let mut alone = (*content).clone();
        alone.background_pool = saito_battle::content::BackgroundPool::default();
        assert_eq!(read_match(&alone, &bytes).unwrap_err(), "round 1: no background \"exe4:acdc-area\" in the background pool");
    }

    /// What isn't a match is refused, saying where.
    #[test]
    fn bytes_that_arent_a_match_are_refused() {
        let content = exe6_content();
        let m = crate::pick::live(&content, "exe6", 5, None).unwrap();
        let bytes = match_bytes(&content, &m).unwrap();
        // (Each place: its stage, then its background's key's length and the key.)
        let places: Vec<usize> = m.places(&content, 5).unwrap().iter().map(|p| 2 + 1 + background_key(&content, p.background).unwrap().len()).collect();
        let place_at = |i: usize| 5 + places[..i].iter().sum::<usize>();
        let read = |b: &[u8]| read_match(&content, b).unwrap_err();
        assert_eq!(read(&bytes[..3]), "seed: the bytes end inside it");
        assert_eq!(read(&bytes[..4]), "first to: the bytes end inside it");
        for n in [0, crate::MAX_FIRST_TO + 1] {
            let mut first_to = bytes.clone();
            first_to[4] = n;
            assert_eq!(read(&first_to), format!("first to {n}: a set is first to 1 to 10"));
        }
        assert_eq!(read(&bytes[..place_at(1) + 1]), "round 2: the bytes end inside it");
        assert_eq!(read(&bytes[..place_at(1) + 3]), "round 2: the bytes end inside it", "inside its background");
        let cut = read(&bytes[..bytes.len() - 1]);
        assert!(cut.starts_with("right: ") && cut.ends_with("the bytes end inside it"), "{cut}");
        let mut more = bytes.clone();
        more.push(0);
        assert_eq!(read(&more), "1 bytes after its end");
        let mut stage = bytes.clone();
        stage[5..7].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&stage), "round 1: no stage 65535 in the content");
        let mut background = bytes.clone();
        background[place_at(2) + 3..place_at(2) + 7].copy_from_slice(b"exe9");
        let named = std::str::from_utf8(&background[place_at(2) + 3..place_at(3)]).unwrap().to_string();
        assert_eq!(read(&background), format!("round 3: no background {named:?} in the background pool"));
        // The left side's navi, a handle past the content's navis (after the
        // custom screen's time limit's two bytes).
        let at = place_at(3) + 2 + before(&content, &m.sides[0], "navi");
        let mut navi = bytes.clone();
        navi[at..at + 2].copy_from_slice(&0x7FFFu16.to_le_bytes());
        assert_eq!(read(&navi), "left: navi: no navi 32766 in the content");
    }

    #[test]
    fn a_match_keeps_its_abd_mode() {
        let exe5 = exe5_content();
        let live5 = crate::pick::live(&exe5, "exe5", 13, None).unwrap();
        let base = match_bytes(&exe5, &live5).unwrap();
        for mode in [crate::AbdMode::Original, crate::AbdMode::LegacyCollection, crate::AbdMode::DoubleTeamDs] {
            let m = Match { abd_mode: mode, ..live5.clone() };
            let bytes = match_bytes(&exe5, &m).unwrap();
            if mode == crate::AbdMode::Original {
                assert_eq!(bytes, base);
            } else {
                assert_eq!(bytes.len(), base.len() + 2);
            }
            let back = read_match(&exe5, &bytes).unwrap();
            assert_eq!(back.abd_mode, mode);
            assert_eq!(match_bytes(&exe5, &back).unwrap(), bytes);
        }
        // EXE4 rejects DoubleTeamDs in binary.
        let exe4 = crate::testing::exe4_content();
        let mut bad4 = crate::pick::live(&exe4, "exe4", 13, None).unwrap();
        bad4.abd_mode = crate::AbdMode::DoubleTeamDs;
        let bytes = match_bytes(&exe4, &bad4).unwrap();
        assert!(read_match(&exe4, &bytes).is_err());
    }

    #[test]
    fn a_match_keeps_its_cs_prob_fix() {
        let exe5 = exe5_content();
        let live5 = crate::pick::live(&exe5, "exe5", 17, None).unwrap();
        let base5 = match_bytes(&exe5, &live5).unwrap();
        for fix in [crate::CsProbFix::NONE, crate::CsProbFix::from_bits(0x0088), crate::CsProbFix::all("exe5")] {
            let m = Match { cs_prob_fix: fix, ..live5.clone() };
            let bytes = match_bytes(&exe5, &m).unwrap();
            if fix.is_empty() {
                assert_eq!(bytes, base5);
            } else {
                assert_eq!(bytes.len(), base5.len() + 3);
            }
            let back = read_match(&exe5, &bytes).unwrap();
            assert_eq!(back.cs_prob_fix, fix);
            assert_eq!(match_bytes(&exe5, &back).unwrap(), bytes);
        }
        // EXE5 rejects EXE6-only bits.
        let mut bad5 = live5.clone();
        bad5.cs_prob_fix = crate::CsProbFix::all("exe6");
        let bytes = match_bytes(&exe5, &bad5).unwrap();
        assert!(read_match(&exe5, &bytes).is_err());
        // EXE4 rejects any non-zero cs_prob_fix.
        let exe4 = crate::testing::exe4_content();
        let mut bad4 = crate::pick::live(&exe4, "exe4", 17, None).unwrap();
        bad4.cs_prob_fix = crate::CsProbFix::from_bits(1);
        let bytes = match_bytes(&exe4, &bad4).unwrap();
        assert!(read_match(&exe4, &bytes).is_err());
    }

    #[test]
    fn exe4_all_guard_and_exe5_dark_cut_in_roundtrip_and_simulate() {
        use saito_battle::battle::{fight, mode};
        use saito_battle::custom::{FolderChip, SavedFolder};
        use saito_battle::input::{PlayerTick, keys};
        use saito_battle::Battle;

        // ---- S-094: EXE4 All Guard option ----
        let exe4 = crate::testing::exe4_content();
        let live4 = crate::pick::live(&exe4, "exe4", 17, None).unwrap();
        let mut m4 = live4.clone();
        m4.all_guard_disabled = true;
        let bytes4 = match_bytes(&exe4, &m4).unwrap();
        let back4 = read_match(&exe4, &bytes4).unwrap();
        assert!(back4.all_guard_disabled);
        assert_ne!(back4.round(&exe4, 17).settings.effects & saito_battle::setup::effects::NO_ALL_GUARD, 0);

        // Install both halves of All Guard (076 + 077) on side 0.
        let c1 = crate::ids::entry(&exe4, "exe4", "patch_cards", "076-all-guard-1-2").unwrap();
        let c2 = crate::ids::entry(&exe4, "exe4", "patch_cards", "077-all-guard-2-2").unwrap();
        let mut with_ag = live4.clone();
        with_ag.sides[0]
            .set_fact(
                &exe4,
                "patch_cards",
                &[
                    saito_battle::rules::Fact::Record(vec![
                        ("card", saito_battle::rules::Fact::Value(saito_content_api::Value::Def(saito_content_api::Registry::Entry, c1.0))),
                        ("slot", saito_battle::rules::Fact::Value(saito_content_api::Value::Int(2))),
                    ]),
                    saito_battle::rules::Fact::Record(vec![
                        ("card", saito_battle::rules::Fact::Value(saito_content_api::Value::Def(saito_content_api::Registry::Entry, c2.0))),
                        ("slot", saito_battle::rules::Fact::Value(saito_content_api::Value::Int(3))),
                    ]),
                ],
            )
            .unwrap();
        let has_guard = |disabled: bool| {
            let mut m = with_ag.clone();
            m.all_guard_disabled = disabled;
            let mut b = Battle::new(m.round(&exe4, 7), exe4.clone());
            for _ in 0..30 {
                b.tick(&[PlayerTick::default(), PlayerTick::default()], Default::default());
            }
            let p0 = b.player(0).unwrap();
            let cid = b.objects.get(p0).collision.unwrap();
            b.collision.get(cid).f1 & saito_battle::collision::f1::GUARD != 0
        };
        assert!(has_guard(false), "EXE4 Original GBA: All Guard must raise guard");
        assert!(!has_guard(true), "EXE4 Legacy Collection: All Guard must be disabled");

        // ---- S-093: EXE5 Light MegaMan dimming dark chip cut-in ----
        let exe5 = exe5_content();
        let live5 = crate::pick::live(&exe5, "exe5", 17, None).unwrap();
        let mut m5 = live5.clone();
        m5.dark_cut_in_fix = true;
        let bytes5 = match_bytes(&exe5, &m5).unwrap();
        let back5 = read_match(&exe5, &bytes5).unwrap();
        assert!(back5.dark_cut_in_fix);
        assert_ne!(back5.round(&exe5, 17).settings.effects & saito_battle::setup::effects::CONSUME_REFUSED_CUT_IN, 0);

        // Set up side 0 (Light MegaMan with darkinvs) vs side 1 (with roll, a dimming chip).
        let darkinvs = crate::ids::chip(&exe5, "exe5", "darkinvs").unwrap();
        let roll = crate::ids::chip(&exe5, "exe5", "roll").unwrap();
        let cannon = crate::ids::chip(&exe5, "exe5", "cannon").unwrap();
        let code_of = |c| exe5.defs.chip(c).record.codes[0];
        let mut f0 = [FolderChip::new(cannon, code_of(cannon)); 30];
        f0[0] = FolderChip::new(darkinvs, code_of(darkinvs));
        let mut f1 = [FolderChip::new(cannon, code_of(cannon)); 30];
        f1[0] = FolderChip::new(roll, code_of(roll));
        let mut m5_sim = live5.clone();
        m5_sim.sides[0].set_folder(&exe5, &SavedFolder { chips: f0, regular: Some(0), tags: None }.into()).unwrap();
        m5_sim.sides[1].set_folder(&exe5, &SavedFolder { chips: f1, regular: Some(0), tags: None }.into()).unwrap();

        let run_cut_in = |fix: bool| -> (u16, usize) {
            let mut m = m5_sim.clone();
            m.dark_cut_in_fix = fix;
            let mut b = Battle::new(m.round(&exe5, 0x5EED), exe5.clone());
            let mut last = [None; 2];
            for _ in 0..3000 {
                if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
                    break;
                }
                let input = [0, 1].map(|side| {
                    last[side] = saito_battle::scenario::custom_choice(&b, side, last[side]);
                    PlayerTick::buttons(0).with_choice(last[side])
                });
                b.tick(&input, Default::default());
            }
            // Give side 0 ChipRecovery = 50 and lower HP so heal_on_use is observable.
            b.stats[0].chip_recovery = 50;
            let p0 = b.player(0).unwrap();
            let start_hp = 500;
            b.objects.get_mut(p0).hp = start_hp;
            // Side 1 uses Roll (dimming).
            b.tick(&[PlayerTick::default(), PlayerTick::buttons(keys::A)], Default::default());
            // Wait until side 1's dimming is ShowingName, then side 0 presses A to cut in with DarkInvs.
            let mut cut_in_done = false;
            for _ in 0..40 {
                let p0_btn = if !cut_in_done && b.dimming[1].state == saito_battle::dimming::DimmingState::ShowingName {
                    cut_in_done = true;
                    keys::A
                } else {
                    0
                };
                b.tick(&[PlayerTick::buttons(p0_btn), PlayerTick::default()], Default::default());
            }
            assert!(cut_in_done, "side 1 dimming never entered ShowingName");
            let p0 = b.player(0).unwrap();
            let healed = b.objects.get(p0).hp.saturating_sub(start_hp);
            let remaining = b.hands[0].remaining() as usize;
            (healed, remaining)
        };

        let (healed_orig, rem_orig) = run_cut_in(false);
        assert_eq!(healed_orig, 50, "Original GBA: heal_on_use triggers on cut-in");
        assert!(rem_orig >= 1, "Original GBA: refused DarkInvs stays in hand");

        let (healed_lc, rem_lc) = run_cut_in(true);
        assert_eq!(healed_lc, 50, "Legacy Collection: heal_on_use triggers once");
        assert_eq!(rem_lc, rem_orig - 1, "Legacy Collection: refused DarkInvs is consumed from hand");
    }

    /// Where fact `name` of a side starts in its compact form: the bytes of
    /// the facts before it.
    fn before(content: &Content, side: &Side, name: &str) -> usize {
        let schema = content.defs.schema(content.defs.rules().unwrap().setup);
        let block = side.facts.block().unwrap();
        (0..schema.index_of(name).unwrap()).map(|i| compact_len(block, schema.place(i))).sum()
    }

    fn compact_len(block: &Block, place: saito_content_api::Place) -> usize {
        match place.ty() {
            FieldType::Record(fields) => (0..fields.fields().len()).map(|i| compact_len(block, place.field_at(i))).sum(),
            FieldType::List(_, max) => {
                (if *max > 255 { 2 } else { 1 }) + (0..block.len_at(place).unwrap()).map(|k| compact_len(block, place.elem(k).unwrap())).sum::<usize>()
            }
            ty => ty.size(),
        }
    }
}
