//! The player's tactics on disk: each a JSON file of its own in the builds folder (`crate::paths::builds`), a
//! folder per game (`builds/exe6/<name>.saitotactic`). A tactic file names the saito format identifier, schema version,
//! game and title, then states the side under `tactic` as a match's side states one (`saito_match::file`: the same
//! names):
//!
//! ```json
//! {
//!   "format": "top.exessrhub.saito.tactic.v1",
//!   "schema": 1,
//!   "game_id": "exe6",
//!   "title": "Tengu rush",
//!   "ruleset_ref": "preset:masters",
//!   "tactic": {
//!     "navi": "tenguman",
//!     "cybeast": "falzar-beast",
//!     "folder": [...]
//!   },
//!   "reserve": {
//!     "navicust_programs": [...],
//!     "patch_cards": [...]
//!   }
//! }
//! ```
//!
//! Files without `format == "top.exessrhub.saito.tactic.v1"` and `schema == 1` (including legacy TOML build files)
//! files, even if renamed to `.saitotactic`) are strictly refused.
//!
//! **The name is the player's own.** It is the file's alone: `tactic` is what a match, a lobby and a replay are
//! given, and it holds no name.
//!
//! **The ruleset it is made for** (the user: "the build in the menu should indicate the ruleset it's for"): a
//! preset's id or the player's own ruleset's name, by kind (`preset:<id>`, `own:<name>`: `crate::rulesets::name_text`),
//! what it is checked under outside a lobby (`crate::rulesets::made_for`). It is the file's alone too, as the name
//! is. A file that names none (written before builds named theirs) is made for the game's first preset; one that
//! names a ruleset no longer there says it is missing, and is checked under that preset. The player's own renamed
//! is renamed in the builds made for it ([`rename_ruleset`]).
//!
//! **What is kept in reserve.** Some of a build's facts are its game's own navi's alone, the one that changes form
//! (MegaMan's NaviCust, his patch cards, his auto battle data: the editor's tabs say which facts are theirs,
//! `crate::screens::builds::tabs`). While a build's navi is another, those facts are kept under `reserve`,
//! outside the side (the rules refuse a NaviCust on a navi that compiles none, and a side states nothing that
//! isn't its navi's), and they are put back when the game's own navi is chosen again ([`Build::set_navi`]).
//!
//! A build made new is `saito_match::build::Builds::new_build` (the game's own navi, a version's group's forms, the
//! folder a new save starts with); there are no random builds. A build states no version (docs/design/versions.md
//! §2.1): a file written before it did (its `version`) is no build now, and the list says it can't be read (§3.2:
//! old builds are dropped, not converted).

use saito_battle::Content;
use saito_battle::content::PlayerFact;
use saito_match::build::Builds;
use saito_match::facts::{Stated, fact_of};
use saito_match::file::{FactsFile, SideFile};
use saito_match::ruleset::Name;
use saito_match::{AbdMode, Facts, Side};
use saito_content_api::NaviHandle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The most letters a build's name has.
pub const NAME_LENGTH: usize = 32;

/// A tactic file's extension (strict JSON format, incompatible with saito).
pub const EXTENSION: &str = "saitotactic";

/// Required format identifier inside `.saitotactic` files.
const FORMAT: &str = "top.exessrhub.saito.tactic.v1";

/// Required schema version inside `.saitotactic` files.
const SCHEMA: u32 = 1;

/// An order-preserving map of fact name to JSON value.
#[derive(Clone, Debug, Default, PartialEq)]
struct JsonFacts(Vec<(String, serde_json::Value)>);

impl JsonFacts {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for JsonFacts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for JsonFacts {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<JsonFacts, D::Error> {
        struct Entries;
        impl<'de> serde::de::Visitor<'de> for Entries {
            type Value = JsonFacts;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a tactic's facts, each a key and its value")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<JsonFacts, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, serde_json::Value>()? {
                    out.push(entry);
                }
                Ok(JsonFacts(out))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Integer(n) => serde_json::Value::Number((*n).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f).map_or(serde_json::Value::Null, serde_json::Value::Number),
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Array(items) => serde_json::Value::Array(items.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, val) in table {
                map.insert(k.clone(), toml_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
    }
}

fn json_to_toml(v: &serde_json::Value) -> Result<toml::Value, String> {
    match v {
        serde_json::Value::Null => Err("null values are not allowed in a tactic file".into()),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => n.as_i64().map(toml::Value::Integer).ok_or_else(|| format!("{n} is no whole number")),
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Array(items) => items.iter().map(json_to_toml).collect::<Result<Vec<_>, _>>().map(toml::Value::Array),
        serde_json::Value::Object(map) => {
            let mut table = toml::map::Map::new();
            for (k, val) in map {
                table.insert(k.clone(), json_to_toml(val)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

fn to_json_facts(side: SideFile) -> JsonFacts {
    JsonFacts(side.facts.0.into_iter().map(|(k, v)| (k, toml_to_json(&v))).collect())
}

fn to_side_file(facts: JsonFacts) -> Result<SideFile, String> {
    let mut out = Vec::with_capacity(facts.0.len());
    for (k, v) in facts.0 {
        out.push((k, json_to_toml(&v)?));
    }
    Ok(SideFile { facts: FactsFile(out) })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TacticFile {
    format: String,
    schema: u32,
    game_id: String,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ruleset_ref: Option<String>,
    #[serde(default)]
    tactic: JsonFacts,
    #[serde(default, skip_serializing_if = "JsonFacts::is_empty")]
    reserve: JsonFacts,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    abd_profiles: BTreeMap<String, JsonFacts>,
}

/// Per-ABD-mode overrides (folder and/or auto battle data) stored alongside the primary GBA side
/// for EXE4 (Legacy Collection) and EXE5 (Legacy Collection, Double Team DS).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbdProfile {
    pub folder: Option<Facts>,
    pub auto_battle: Option<Facts>,
}

impl AbdProfile {
    pub fn is_empty(&self) -> bool {
        self.folder.is_none() && self.auto_battle.is_none()
    }
}

/// A build: its name, the ruleset it is made for, the side a match is handed, the facts kept aside while its
/// navi takes none of them, and optional per-ABD-mode folder/AI profiles (EXE4/EXE5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub name: String,
    /// The ruleset it is made for, by name (`crate::rulesets::made_for`). None: the game's first preset (a file
    /// that names none).
    pub ruleset: Option<Name>,
    pub side: Side,
    /// The facts kept aside, stated; every other one the rules' default.
    aside: Facts,
    /// Optional per-ABD-mode overrides (keyed by non-Original `AbdMode`: LC, 5DS).
    pub profiles: BTreeMap<AbdMode, AbdProfile>,
}

/// A build's file, as it is listed: its path, its name, and its text (read against its game's content once
/// that is loaded: [`read`]).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// A fact's value as the facts' writer takes it: an element each for a list.
fn values(value: &Stated) -> Vec<saito_battle::rules::Fact<'_>> {
    match fact_of(value) {
        saito_battle::rules::Fact::List(items) => items,
        fact => vec![fact],
    }
}

/// Folder-related setup field names for `content`.
pub fn folder_fields(content: &Content) -> Vec<&str> {
    [PlayerFact::Folder, PlayerFact::RegularChip, PlayerFact::TagChips]
        .into_iter()
        .filter_map(|role| content.defs.fact_name(role))
        .collect()
}

/// Auto-battle setup field names for `content`.
pub fn auto_battle_fields(content: &Content) -> Vec<&'static str> {
    [crate::builds::auto::PLACES_FIELD, crate::builds::auto::RECORDS_FIELD]
        .into_iter()
        .filter(|field| saito_match::facts::takes(content, field))
        .collect()
}

/// Copy `fields` from `from` into `to` (resetting any field that is default in `from`).
fn copy_fields(content: &Content, from: &Facts, to: &mut Facts, fields: &[&str]) {
    for &field in fields {
        if from.is_default(content, field) {
            to.reset(content, field);
        } else if let Some(value) = from.get(content, field) {
            let _ = to.set(content, field, &values(&value));
        }
    }
}

/// Whether `a` and `b` agree on every field in `fields`.
fn fields_equal(content: &Content, a: &Facts, b: &Facts, fields: &[&str]) -> bool {
    fields.iter().all(|&field| a.get(content, field) == b.get(content, field))
}

impl Build {
    /// A build named `name` of `side`, made for the game's first preset, nothing kept aside.
    pub fn new(content: &Content, name: &str, side: Side) -> Build {
        Build { name: name.to_string(), ruleset: None, side, aside: Facts::defaults(content), profiles: BTreeMap::new() }
    }

    /// Resolve this build's [`Side`] for `mode` (applying LC/5DS folder and auto-battle overrides
    /// when configured, falling back to the primary GBA side otherwise).
    pub fn side_for(&self, content: &Content, mode: AbdMode) -> Side {
        if mode == AbdMode::Original {
            return self.side.clone();
        }
        let Some(profile) = self.profiles.get(&mode) else {
            return self.side.clone();
        };
        let mut side = self.side.clone();
        if let Some(folder) = &profile.folder {
            copy_fields(content, folder, &mut side.facts, &folder_fields(content));
        }
        if let Some(auto) = &profile.auto_battle
            && self.own_navi(content)
        {
            copy_fields(content, auto, &mut side.facts, &auto_battle_fields(content));
        }
        side
    }

    /// Commit edits made to the resolved side for `mode` back into this build.
    /// Shared facts update `self.side`; folder and auto-battle changes update the active profile.
    pub fn apply_edited_side(&mut self, content: &Content, mode: AbdMode, edited: &Side) {
        if mode == AbdMode::Original {
            self.side = edited.clone();
            return;
        }
        let before = self.side_for(content, mode);
        let f_fields = folder_fields(content);
        let a_fields = auto_battle_fields(content);
        let folder_changed = !fields_equal(content, &before.facts, &edited.facts, &f_fields);
        let auto_changed = !fields_equal(content, &before.facts, &edited.facts, &a_fields);

        // Sync any shared (non-folder, non-auto-battle) facts back to `self.side`.
        let shared: Vec<&str> = saito_match::facts::fields(content)
            .into_iter()
            .map(|f| f.name)
            .filter(|name| !f_fields.contains(name) && !a_fields.contains(name))
            .collect();
        copy_fields(content, &edited.facts, &mut self.side.facts, &shared);

        if folder_changed || auto_changed {
            let entry = self.profiles.entry(mode).or_default();
            if folder_changed {
                let mut facts = Facts::defaults(content);
                copy_fields(content, &edited.facts, &mut facts, &f_fields);
                entry.folder = Some(facts);
            }
            if auto_changed {
                let mut facts = Facts::defaults(content);
                copy_fields(content, &edited.facts, &mut facts, &a_fields);
                entry.auto_battle = Some(facts);
            }
        }
    }

    /// Whether `mode` has a customized folder override (always true for `Original`).
    pub fn has_custom_folder(&self, mode: AbdMode) -> bool {
        mode == AbdMode::Original || self.profiles.get(&mode).is_some_and(|p| p.folder.is_some())
    }

    /// Whether `mode` has a customized auto-battle override (always true for `Original`).
    pub fn has_custom_auto_battle(&self, mode: AbdMode) -> bool {
        mode == AbdMode::Original || self.profiles.get(&mode).is_some_and(|p| p.auto_battle.is_some())
    }

    /// The build made for ruleset `ruleset` (none: the game's first preset).
    pub fn made_for(self, ruleset: Option<Name>) -> Build {
        Build { ruleset, ..self }
    }

    /// The build under another name.
    pub fn named(&self, name: &str) -> Build {
        Build { name: name.to_string(), ..self.clone() }
    }

    /// Whether the build's navi is its game's own, the one that changes form (the one a NaviCust, patch cards
    /// and auto battle data are of).
    pub fn own_navi(&self, content: &Content) -> bool {
        self.side.stated_navi(content).is_some_and(|n| content.navi(n).changes_form())
    }

    /// Whether fact `field` is kept aside.
    #[allow(dead_code)] // (The tabs to come show what they keep.)
    pub fn keeps_aside(&self, content: &Content, field: &str) -> bool {
        !self.aside.is_default(content, field)
    }

    /// Where each of `fields` (the facts that are the game's own navi's alone) belongs for the build's navi:
    /// in the side if the navi is the game's own (what was kept aside is put back), else aside (the side's
    /// own back at the rules' default). Whether anything moved.
    pub fn settle_aside(&mut self, content: &Content, fields: &[String]) -> bool {
        let own = self.own_navi(content);
        let (from, to) = if own { (&mut self.aside, &mut self.side.facts) } else { (&mut self.side.facts, &mut self.aside) };
        let mut moved = false;
        for field in fields {
            if from.is_default(content, field) {
                continue;
            }
            let Some(value) = from.get(content, field) else { continue };
            // (What was kept aside goes back only where the side states none of its own.)
            if (!own || to.is_default(content, field)) && to.set(content, field, &values(&value)).is_err() {
                continue;
            }
            from.reset(content, field);
            moved = true;
        }
        moved
    }

    /// Choose the build's navi, as a build is ([`Builds::set_navi`]: its level at its last, its form list kept or
    /// none, its choices settled), each of `fields` kept aside or put back as the navi takes it
    /// ([`Build::settle_aside`]).
    pub fn set_navi(&mut self, content: &Content, builds: &Builds, navi: NaviHandle, fields: &[String]) -> Result<(), String> {
        let mut made = self.clone();
        // (Aside first where the navi takes none, back last where it does: the side is never one the rules
        // refuse on the way.)
        made.side.set_navi(content, navi)?;
        made.settle_aside(content, fields);
        builds.set_navi(content, &mut made.side, navi)?;
        *self = made;
        Ok(())
    }

    /// Set the build to what every build is ([`Builds::as_built`]: the facts no build states back at the
    /// rules' defaults, its choices settled), with each of `fields` where its navi takes it. Whether it
    /// changed: a build read from a file or made from a save is settled before it is edited or kept.
    pub fn settle(&mut self, content: &Content, builds: &Builds, fields: &[String]) -> bool {
        let before = self.clone();
        self.settle_aside(content, fields);
        builds.as_built(content, &mut self.side);
        *self != before
    }
}

/// A build's name as it is typed: one line, no room at its ends, [`NAME_LENGTH`] letters at most.
pub fn clean_name(text: &str) -> String {
    let line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    line.trim_start().chars().take(NAME_LENGTH).collect()
}

/// The folder of game `game`'s builds.
pub fn folder(game: &str) -> PathBuf {
    crate::paths::builds().join(game)
}

/// Game `game`'s builds, by name: each file of its folder that is a build of it (a file that isn't one is
/// left out).
pub fn list(game: &str) -> Vec<Listed> {
    list_in(&folder(game), game)
}

/// [`list`] of folder `dir`.
fn list_in(dir: &Path, game: &str) -> Vec<Listed> {
    let Ok(dir) = saito_files::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Listed> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| {
            let text = saito_files::read_to_string(&path).ok()?;
            let head = header(&text).ok().filter(|(g, _)| g == game)?;
            Some(Listed { path, name: head.1, text })
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.path.cmp(&b.path)));
    out
}

/// A tactic file's game and title, read before its game's content is.
fn header(text: &str) -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct Header {
        format: String,
        schema: u32,
        game_id: String,
        title: String,
    }
    let h: Header = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if h.format != FORMAT || h.schema != SCHEMA {
        return Err(format!("not a saito tactic (format {:?}, schema {})", h.format, h.schema));
    }
    Ok((h.game_id, h.title))
}

/// A tactic file's text read against its game's content: the build, or what keeps it from being one (a name
/// the game hasn't), in the log's words.
pub fn read(content: &Content, game: &str, text: &str) -> Result<Build, String> {
    let f: TacticFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if f.format != FORMAT || f.schema != SCHEMA {
        return Err(format!("not a saito tactic (format {:?}, schema {})", f.format, f.schema));
    }
    if f.game_id != game {
        return Err(format!("a build of {}, not of {game}", f.game_id));
    }
    let side_file = to_side_file(f.tactic)?;
    let reserve_file = to_side_file(f.reserve)?;
    let mut problems = Vec::new();
    let side = saito_match::file::resolve_side(content, game, &side_file, "the build", &mut problems);
    let aside = saito_match::file::resolve_facts(content, game, &reserve_file, "what the build keeps aside", &mut problems);
    let ruleset = match f.ruleset_ref {
        Some(text) => Some(crate::rulesets::name_of_text(&text).ok_or_else(|| format!("ruleset {text:?}: \"preset:<id>\" or \"own:<name>\""))?),
        None => None,
    };
    let f_fields = folder_fields(content);
    let a_fields = auto_battle_fields(content);
    let mut profiles = BTreeMap::new();
    for (key, json_facts) in f.abd_profiles {
        let Some(mode) = AbdMode::from_key(&key).filter(|&m| m != AbdMode::Original && m.allowed_for(game)) else {
            return Err(format!("unknown or disallowed ABD profile {key:?} for {game}"));
        };
        let has_folder = json_facts.0.iter().any(|(k, _)| f_fields.contains(&k.as_str()));
        let has_auto = json_facts.0.iter().any(|(k, _)| a_fields.contains(&k.as_str()));
        let profile_file = to_side_file(json_facts)?;
        if let Some(resolved) = saito_match::file::resolve_facts(content, game, &profile_file, &format!("ABD profile {key}"), &mut problems) {
            let mut profile = AbdProfile::default();
            if has_folder {
                let mut facts = Facts::defaults(content);
                copy_fields(content, &resolved, &mut facts, &f_fields);
                profile.folder = Some(facts);
            }
            if has_auto {
                let mut facts = Facts::defaults(content);
                copy_fields(content, &resolved, &mut facts, &a_fields);
                profile.auto_battle = Some(facts);
            }
            if !profile.is_empty() {
                profiles.insert(mode, profile);
            }
        }
    }
    match (side, aside) {
        (Some(side), Some(aside)) if problems.is_empty() => Ok(Build { name: f.title, ruleset, side, aside, profiles }),
        _ => Err(problems.join("; ")),
    }
}

/// A build as its `.saitotactic` file's JSON text.
pub fn text(content: &Content, game: &str, build: &Build) -> String {
    let reserve = if build.aside != Facts::defaults(content) {
        to_json_facts(saito_match::file::side_file(content, &Side { facts: build.aside.clone() }))
    } else {
        JsonFacts::default()
    };
    let f_fields = folder_fields(content);
    let a_fields = auto_battle_fields(content);
    let mut abd_profiles = BTreeMap::new();
    for (&mode, profile) in &build.profiles {
        if mode == AbdMode::Original || !mode.allowed_for(game) || profile.is_empty() {
            continue;
        }
        let mut combined = Facts::defaults(content);
        if let Some(folder) = &profile.folder {
            copy_fields(content, folder, &mut combined, &f_fields);
        }
        if let Some(auto) = &profile.auto_battle {
            copy_fields(content, auto, &mut combined, &a_fields);
        }
        let mut json = to_json_facts(saito_match::file::side_file(content, &Side { facts: combined }));
        // Preserve explicitly customized empty folder / empty auto_battle_places so they don't fall back to GBA.
        if profile.folder.is_some()
            && let Some(folder_key) = content.defs.fact_name(PlayerFact::Folder)
            && !json.0.iter().any(|(k, _)| f_fields.contains(&k.as_str()))
        {
            json.0.push((folder_key.to_string(), serde_json::Value::Array(Vec::new())));
        }
        if profile.auto_battle.is_some()
            && saito_match::facts::takes(content, crate::builds::auto::PLACES_FIELD)
            && !json.0.iter().any(|(k, _)| a_fields.contains(&k.as_str()))
        {
            json.0.push((crate::builds::auto::PLACES_FIELD.to_string(), serde_json::Value::Array(Vec::new())));
        }
        if !json.is_empty() {
            abd_profiles.insert(mode.key().to_string(), json);
        }
    }
    let file = TacticFile {
        format: FORMAT.to_string(),
        schema: SCHEMA,
        game_id: game.to_string(),
        title: build.name.clone(),
        ruleset_ref: build.ruleset.as_ref().map(crate::rulesets::name_text),
        tactic: to_json_facts(saito_match::file::side_file(content, &build.side)),
        reserve,
        abd_profiles,
    };
    serde_json::to_string_pretty(&file).expect("a tactic file serializes") + "\n"
}

/// Write a build to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, build: &Build) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        saito_files::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    saito_files::write(path, text(content, game, build)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Ruleset `from` renamed `to` in each of game `game`'s builds made for it: its file read on `content` and written
/// again. Each file that names it and couldn't be, with why (it doesn't read as a build on this content, or
/// can't be written).
pub fn rename_ruleset(content: &Content, game: &str, from: &Name, to: &Name) -> Vec<(PathBuf, String)> {
    rename_ruleset_in(&folder(game), content, game, from, to)
}

/// [`rename_ruleset`] in folder `dir`.
fn rename_ruleset_in(dir: &Path, content: &Content, game: &str, from: &Name, to: &Name) -> Vec<(PathBuf, String)> {
    let mut failed = Vec::new();
    for listed in list_in(dir, game) {
        let named = serde_json::from_str::<TacticFile>(&listed.text)
            .ok()
            .filter(|f| f.format == FORMAT && f.schema == SCHEMA)
            .and_then(|f| f.ruleset_ref)
            .and_then(|r| crate::rulesets::name_of_text(&r));
        if named.as_ref() != Some(from) {
            continue;
        }
        let renamed = read(content, game, &listed.text).and_then(|b| write(&listed.path, content, game, &b.made_for(Some(to.clone()))));
        if let Err(why) = renamed {
            failed.push((listed.path, why));
        }
    }
    failed
}

/// A path for a new build of `game` named `name` that no file has: the name's letters and digits
/// (`falzar-heat.saitotactic`), numbered if it is taken.
pub fn new_path(game: &str, name: &str) -> PathBuf {
    path_for(game, name, None)
}

/// [`new_path`] for build `name` of `game` kept at `own` until now (renamed): its own file counts as free, so a name
/// whose letters and digits are the same keeps it.
pub fn renamed_path(game: &str, name: &str, own: &Path) -> PathBuf {
    path_for(game, name, Some(own))
}

fn path_for(game: &str, name: &str, own: Option<&Path>) -> PathBuf {
    let mut stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if stem.is_empty() {
        stem = "tactic".into();
    }
    let dir = folder(game);
    let mut path = dir.join(format!("{stem}.{EXTENSION}"));
    for n in 2.. {
        if !saito_files::exists(&path) || own == Some(path.as_path()) {
            break;
        }
        path = dir.join(format!("{stem}-{n}.{EXTENSION}"));
    }
    path
}

/// A name for a new build that none of `taken` has: `base`, else `base 2`, and so on.
pub fn new_name(base: &str, taken: &[String]) -> String {
    let free = |n: &str| !taken.iter().any(|t| t.to_lowercase() == n.to_lowercase());
    if free(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base} {n}")).find(|n| free(n)).expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_match::Wrong;
    use saito_match::build::Checked;
    use saito_match::testing::{PlacedProgram, exe5_content, exe6_content, patch_cards, set_navicust};

    /// The facts the tests keep aside: what the editor's tabs name for the game's own navi.
    fn fields() -> Vec<String> {
        ["navicust_programs", "patch_cards", "auto_battle_places"].map(String::from).to_vec()
    }

    /// A tactic file reads back as the build it was written from; one of another game, or naming what the
    /// game hasn't, isn't one. Its title is the file's alone: the tactic body says none.
    #[test]
    fn a_build_reads_back() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let build = Build::new(&content, "Falzar \"heat\"", builds.new_build(&content, "falzar").unwrap());
        let text = text(&content, "exe6", &build);
        assert!(text.contains(FORMAT) && text.contains("\"tactic\"") && !text.contains("\"reserve\""), "{text}");
        assert_eq!(header(&text), Ok(("exe6".into(), "Falzar \"heat\"".into())));
        assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
        assert!(read(&content, "exe5", &text).unwrap_err().contains("not of exe5"));
        let wrong = text.replacen("\"navi\": \"megaman\"", "\"navi\": \"nobody\"", 1);
        assert_ne!(wrong, text);
        assert!(read(&content, "exe6", &wrong).is_err());
        let side = serde_json::to_string(&to_json_facts(saito_match::file::side_file(&content, &build.side))).unwrap();
        assert!(!side.contains("Falzar") && !side.contains("title"), "the side a match is given says no title: {side}");
    }

    /// Legacy TOML build files and JSON payloads missing the exact saito format/schema are strictly rejected,
    /// even if given the `.saitotactic` extension.
    #[test]
    fn a_legacy_build_file_is_rejected() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let build = Build::new(&content, "Valid", builds.new_build(&content, "falzar").unwrap());

        let legacy_toml = format!(
            "game = \"exe6\"\nname = \"Legacy\"\n\n{}",
            saito_match::file::side_toml(&content, "side", &build.side)
        );
        assert!(header(&legacy_toml).is_err());
        assert!(read(&content, "exe6", &legacy_toml).is_err());

        let valid_json = text(&content, "exe6", &build);
        let wrong_format = valid_json.replacen(FORMAT, "com.example.legacy.build.v1", 1);
        assert!(header(&wrong_format).is_err());
        assert!(read(&content, "exe6", &wrong_format).is_err());

        let wrong_schema = valid_json.replacen("\"schema\": 1", "\"schema\": 2", 1);
        assert!(header(&wrong_schema).is_err());
        assert!(read(&content, "exe6", &wrong_schema).is_err());

        let legacy_json = serde_json::json!({
            "game": "exe6",
            "name": "Legacy",
            "side": to_json_facts(saito_match::file::side_file(&content, &build.side)),
        })
        .to_string();
        assert!(header(&legacy_json).is_err());
        assert!(read(&content, "exe6", &legacy_json).is_err());
    }

    /// EXE5 auto-battle records contain negative integers (`dx`) and empty `{}` entries; both round-trip cleanly
    /// through `.saitotactic` JSON.
    #[test]
    fn an_exe5_build_with_negative_integers_and_empty_records_roundtrips() {
        use saito_battle::rules::Fact;
        use saito_content_api::{Registry, Value};
        let content = exe5_content();
        let builds = Builds::of(&content, "exe5").unwrap();
        let mut side = builds.new_build(&content, builds.groups[0].id.as_str()).unwrap();
        let sword = saito_match::ids::chip(&content, "exe5", "sword").unwrap().0;
        let def = |h| Fact::Value(Value::Def(Registry::Chip, h));
        let int = |n| Fact::Value(Value::Int(n));
        let mut places: Vec<Fact> = (0..4).map(|_| Fact::Record(Vec::new())).collect();
        places[3] = Fact::Record(vec![("chip", def(sword))]);
        let chips = Fact::List(vec![Fact::Record(vec![("chip", def(sword))]), Fact::Record(Vec::new()), Fact::Record(Vec::new()), Fact::Record(Vec::new()), Fact::Record(Vec::new())]);
        let records = vec![Fact::Record(vec![("dx", int(-2)), ("dy", int(-1)), ("chips", chips), ("score", int(5))])];
        side.set_fact(&content, "auto_battle_places", &places).unwrap();
        side.set_fact(&content, "auto_battle_records", &records).unwrap();
        let build = Build::new(&content, "Auto", side);
        let written = text(&content, "exe5", &build);
        assert!(written.contains("-2"), "{written}");
        assert_eq!(read(&content, "exe5", &written), Ok(build));
    }

    /// A new build is its game's own navi's, from the group asked for, with the folder a new save starts with,
    /// and nothing wrong with it.
    #[test]
    fn a_new_build_is_ready() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let builds = Builds::of(&content, game).unwrap();
            for group in &builds.groups {
                let build = Build::new(&content, "New", builds.new_build(&content, &group.id).unwrap());
                assert!(build.own_navi(&content));
                assert_eq!(build.side.folder(&content), builds.new_folder);
                assert!(Checked::of(&content, game, &build.side).ready());
            }
        }
    }

    /// A build file written before builds stated no version (its side's `version`, a card a name and no record)
    /// is no build: it can't be read, and says why; nothing stops (docs/design/versions.md §3.2).
    #[test]
    fn a_stale_build_is_unreadable() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let text = text(&content, "exe6", &Build::new(&content, "Old", builds.new_build(&content, "falzar").unwrap()));
        let old = text.replacen("\"cybeast\": \"falzar-beast\"", "\"version\": \"falzar\"", 1);
        assert_ne!(old, text);
        assert!(read(&content, "exe6", &old).unwrap_err().contains("version"));
        let cards = text.replacen("\"tactic\": {", "\"tactic\": {\n    \"patch_cards\": [\"canodumb\"],", 1);
        assert_ne!(cards, text);
        assert!(read(&content, "exe6", &cards).is_err());
    }

    /// While another navi is chosen, the game's own navi's NaviCust and patch cards are kept aside, outside
    /// the side (which the rules then find nothing wrong with), written to the file and read back; they are
    /// in the side again when the game's own navi is chosen.
    #[test]
    fn the_own_navis_facts_are_kept_aside() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "falzar").unwrap();
        let program = saito_match::ids::entry(&content, "exe6", "navicust_programs", "suprarmr").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program, color: 0, x: 3, y: 3, rotation: 0, compressed: true }], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab")).unwrap();
        let megaman = side.navi(&content);
        let with = Build::new(&content, "Heat", side);
        let stated = |b: &Build, field: &str| !b.side.facts.is_default(&content, field);
        assert!(stated(&with, "navicust_programs") && stated(&with, "patch_cards"));

        // Left in the side, the rules refuse the NaviCust on a navi that compiles none.
        let tenguman = saito_match::ids::navi(&content, "exe6", "tenguman").unwrap();
        let mut left = with.side.clone();
        builds.set_navi(&content, &mut left, tenguman).unwrap();
        assert!(Checked::of(&content, "exe6", &left).problems.iter().any(|p| p.what == Wrong::NotTaken));

        let mut build = with.clone();
        build.set_navi(&content, &builds, tenguman, &fields()).unwrap();
        assert!(!build.own_navi(&content));
        assert!(!stated(&build, "navicust_programs") && !stated(&build, "patch_cards"), "outside the side");
        assert!(build.keeps_aside(&content, "navicust_programs") && build.keeps_aside(&content, "patch_cards"));
        assert!(Checked::of(&content, "exe6", &build.side).ready());
        let text = text(&content, "exe6", &build);
        assert!(text.contains("\"reserve\""), "{text}");
        assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
        // (Another navi again: what is aside stays.)
        let protoman = saito_match::ids::navi(&content, "exe6", "protoman").unwrap();
        build.set_navi(&content, &builds, protoman, &fields()).unwrap();
        assert!(build.keeps_aside(&content, "navicust_programs"));

        build.set_navi(&content, &builds, megaman, &fields()).unwrap();
        assert_eq!(build, with, "back as it was");
        assert!(!super::text(&content, "exe6", &build).contains("\"reserve\""));
    }

    /// A build read with a navi that takes none of what its side states (written by hand) is settled: those
    /// facts aside.
    #[test]
    fn a_build_is_settled() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab")).unwrap();
        let heatman = saito_match::ids::navi(&content, "exe6", "heatman").unwrap();
        builds.set_navi(&content, &mut side, heatman).unwrap();
        let mut build = Build::new(&content, "By hand", side);
        assert!(build.settle(&content, &builds, &fields()));
        assert!(build.side.facts.is_default(&content, "patch_cards") && build.keeps_aside(&content, "patch_cards"));
        assert!(!build.settle(&content, &builds, &fields()), "once is enough");
    }

    /// The selector's choices keep the rest of the build: EXE6's Cybeast changed, the build keeps its Crosses, its
    /// folder, its NaviCust and its patch cards (a card's variant its own), and its file reads back as it.
    #[test]
    fn a_chosen_form_keeps_the_build() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let program = saito_match::ids::entry(&content, "exe6", "navicust_programs", "suprarmr").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program, color: 0, x: 3, y: 3, rotation: 0, compressed: true }], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "skarab,bass-cross-megaman/gregar")).unwrap();
        let gregar = Build::new(&content, "Gregar", side);
        let mut build = gregar.clone();
        let falzar_beast = saito_match::ids::form(&content, "exe6", "falzar-beast").unwrap();
        assert!(builds.choose_form(&content, &mut build.side, "cybeast", falzar_beast));
        assert_eq!(build.side.version(&content), Some("falzar"));
        for kept in ["navi", "folder", "crosses", "navicust_programs", "navicust_expansions", "patch_cards", "bug_frags", "beast_out"] {
            assert_eq!(build.side.facts.get(&content, kept), gregar.side.facts.get(&content, kept), "{kept} is kept");
        }
        assert_eq!(read(&content, "exe6", &text(&content, "exe6", &build)), Ok(build.clone()));
        let gregar_beast = saito_match::ids::form(&content, "exe6", "gregar-beast").unwrap();
        assert!(builds.choose_form(&content, &mut build.side, "cybeast", gregar_beast));
        assert_eq!(build, gregar, "back as it was");
    }

    /// The Navi tab's ALL and NONE: a version's Crosses taken out at once, the other's put in up to the Cross
    /// window's five, the rest refused for want of room; nothing done is no change.
    #[test]
    fn all_and_none_of_a_version() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let version_of = |f: &saito_content_api::FormHandle| content.defs.forms[f.index()].record.version.clone();
        let crosses = builds.form_choices(&content, &side).into_iter().find(|c| c.fact == "crosses").unwrap();
        let of = |v: &str| -> Vec<_> { crosses.forms.iter().copied().filter(|f| version_of(f).as_deref() == Some(v)).collect() };
        let (gregar, falzar) = (of("gregar"), of("falzar"));
        assert_eq!(builds.chosen_forms(&content, &side, "crosses"), gregar);
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &falzar, true), (false, 5), "no room");
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &gregar, false), (true, 0));
        assert!(builds.chosen_forms(&content, &side, "crosses").is_empty());
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &gregar, false), (false, 0));
        assert!(builds.choose_form(&content, &mut side, "crosses", gregar[4]));
        assert_eq!(builds.choose_forms(&content, &mut side, "crosses", &falzar, true), (true, 1));
        // (In the battle's order: Gregar's, then Falzar's first four.)
        assert_eq!(builds.chosen_forms(&content, &side, "crosses"), [&gregar[4..], &falzar[..4]].concat());
        let cybeast = builds.chosen_forms(&content, &side, "cybeast");
        assert_eq!(builds.choose_forms(&content, &mut side, "cybeast", &cybeast, false), (false, 0), "one form is no list");
    }

    /// A build names the ruleset it is made for in its file, by kind, and reads back made for it; a file that names
    /// none (written before builds named theirs) reads as made for the game's first preset (none named), and one
    /// that names it in no known way isn't a build.
    #[test]
    fn a_build_names_its_ruleset() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let plain = Build::new(&content, "Heat", builds.new_build(&content, "falzar").unwrap());
        let text_of = |b: &Build| text(&content, "exe6", b);
        assert!(!text_of(&plain).contains("ruleset_ref"), "{}", text_of(&plain));
        assert_eq!(read(&content, "exe6", &text_of(&plain)).unwrap().ruleset, None);
        for name in [Name::Preset("senior".into()), Name::Own("Gregar \"cup\"".into())] {
            let build = plain.clone().made_for(Some(name.clone()));
            let text = text_of(&build);
            let want = serde_json::to_string(&crate::rulesets::name_text(&name)).unwrap();
            assert!(text.contains("\"ruleset_ref\"") && text.contains(&want), "{text}");
            assert_eq!(read(&content, "exe6", &text), Ok(build.clone()));
            assert_eq!(read(&content, "exe6", &text).unwrap().named("Copy").ruleset, Some(name), "a duplicate keeps it");
        }
        let senior = text_of(&plain.clone().made_for(Some(Name::Preset("senior".into()))));
        assert!(read(&content, "exe6", &senior.replacen("preset:senior", "senior", 1)).unwrap_err().contains("ruleset"));
    }

    /// The player's own ruleset renamed is renamed in the files of the builds made for it, and in no other.
    #[test]
    fn a_renamed_ruleset_is_followed() {
        let dir = tempfile::tempdir().unwrap();
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let side = builds.new_build(&content, "gregar").unwrap();
        let (cup, renamed, other) = (Name::Own("Cup".into()), Name::Own("Spring cup".into()), Name::Own("Other".into()));
        let made = [("For the cup", Some(cup.clone())), ("For another", Some(other.clone())), ("For the first", None)];
        for (at, (name, ruleset)) in made.iter().enumerate() {
            let build = Build::new(&content, name, side.clone()).made_for(ruleset.clone());
            write(&dir.path().join(format!("{at}.{EXTENSION}")), &content, "exe6", &build).unwrap();
        }
        assert!(rename_ruleset_in(dir.path(), &content, "exe6", &cup, &renamed).is_empty());
        let now: Vec<(String, Option<Name>)> = list_in(dir.path(), "exe6").iter().map(|l| (l.name.clone(), read(&content, "exe6", &l.text).unwrap().ruleset)).collect();
        assert_eq!(now, [("For another".into(), Some(other)), ("For the cup".into(), Some(renamed)), ("For the first".into(), None)]);
    }

    #[test]
    fn new_names_and_paths() {
        assert_eq!(new_name("Falzar", &["falzar".into(), "Falzar 2".into()]), "Falzar 3");
        assert_eq!(new_name("Gregar", &["falzar".into()]), "Gregar");
        let path = new_path("exe6", "  Heat / Elec!! ");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("heat-elec.saitotactic"));
        assert_eq!(clean_name("  A\tname\n"), "A name ");
        assert_eq!(clean_name(&"x".repeat(80)).chars().count(), NAME_LENGTH);
    }

    /// EXE5 per-ABD-mode profiles (GBA, LC, 5DS) independently store folder and auto-battle overrides,
    /// fall back to GBA when uncustomized, keep shared facts synced, and round-trip cleanly through JSON.
    #[test]
    fn exe5_abd_profiles_resolve_and_roundtrip() {
        use saito_battle::content::ChipCode;
        use saito_battle::custom::FolderChip;
        use saito_battle::rules::Fact;
        use saito_content_api::{Registry, Value};

        let content = exe5_content();
        let builds = Builds::of(&content, "exe5").unwrap();
        let mut build = Build::new(&content, "Dark Multi-ABD", builds.new_build(&content, "protoman").unwrap());

        // Initially LC and 5DS inherit GBA completely.
        assert!(!build.has_custom_folder(AbdMode::LegacyCollection));
        assert!(!build.has_custom_auto_battle(AbdMode::LegacyCollection));
        assert_eq!(build.side_for(&content, AbdMode::LegacyCollection), build.side);
        assert_eq!(build.side_for(&content, AbdMode::DoubleTeamDs), build.side);
        assert!(!text(&content, "exe5", &build).contains("\"abd_profiles\""));

        let sword = saito_match::ids::chip(&content, "exe5", "sword").unwrap();
        let cannon = saito_match::ids::chip(&content, "exe5", "cannon").unwrap();
        let darkinvs = saito_match::ids::chip(&content, "exe5", "darkinvs").unwrap();

        // Customize LC folder (swap slot 0 to sword) without touching LC auto_battle.
        let mut lc_side = build.side_for(&content, AbdMode::LegacyCollection);
        let mut lc_folder = lc_side.folder(&content);
        lc_folder.chips[0] = Some(FolderChip::new(sword, ChipCode::ASTERISK));
        lc_side.set_folder(&content, &lc_folder).unwrap();
        build.apply_edited_side(&content, AbdMode::LegacyCollection, &lc_side);

        assert!(build.has_custom_folder(AbdMode::LegacyCollection));
        assert!(!build.has_custom_auto_battle(AbdMode::LegacyCollection));
        assert_eq!(build.side_for(&content, AbdMode::LegacyCollection).folder(&content).chips[0], Some(FolderChip::new(sword, ChipCode::ASTERISK)));
        assert_ne!(build.side.folder(&content).chips[0], Some(FolderChip::new(sword, ChipCode::ASTERISK)));

        // Customize 5DS folder (slot 0 = darkinvs) and 5DS auto_battle_places (slot 3 = cannon).
        let mut ds_side = build.side_for(&content, AbdMode::DoubleTeamDs);
        let mut ds_folder = ds_side.folder(&content);
        ds_folder.chips[0] = Some(FolderChip::new(darkinvs, ChipCode::from_letter('I').unwrap()));
        ds_side.set_folder(&content, &ds_folder).unwrap();
        let mut ds_places: Vec<Fact> = (0..4).map(|_| Fact::Record(Vec::new())).collect();
        ds_places[3] = Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, cannon.0)))]);
        ds_side.set_fact(&content, "auto_battle_places", &ds_places).unwrap();
        build.apply_edited_side(&content, AbdMode::DoubleTeamDs, &ds_side);

        assert!(build.has_custom_folder(AbdMode::DoubleTeamDs));
        assert!(build.has_custom_auto_battle(AbdMode::DoubleTeamDs));

        // Round-trip through .saitotactic JSON.
        let json = text(&content, "exe5", &build);
        assert!(json.contains("\"abd_profiles\"") && json.contains("\"lc\"") && json.contains("\"ds\""), "{json}");
        let back = read(&content, "exe5", &json).unwrap();
        assert_eq!(back, build);
        assert_eq!(back.side_for(&content, AbdMode::Original), build.side);
        assert_eq!(back.side_for(&content, AbdMode::LegacyCollection), lc_side);
        assert_eq!(back.side_for(&content, AbdMode::DoubleTeamDs), ds_side);

        // Explicitly customized empty LC auto_battle_places stays empty after round-trip even if GBA has places.
        let mut gba_places: Vec<Fact> = (0..4).map(|_| Fact::Record(Vec::new())).collect();
        gba_places[3] = Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, sword.0)))]);
        build.side.set_fact(&content, "auto_battle_places", &gba_places).unwrap();
        let mut lc_cleared = build.side_for(&content, AbdMode::LegacyCollection);
        lc_cleared.set_fact(&content, "auto_battle_places", &[]).unwrap();
        build.apply_edited_side(&content, AbdMode::LegacyCollection, &lc_cleared);
        let back2 = read(&content, "exe5", &text(&content, "exe5", &build)).unwrap();
        assert!(back2.has_custom_auto_battle(AbdMode::LegacyCollection));
        assert!(back2.side_for(&content, AbdMode::LegacyCollection).facts.is_default(&content, "auto_battle_places"));
    }
}
