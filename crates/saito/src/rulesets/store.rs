//! The player's own rulesets on disk: each a JSON file of its own in the rulesets folder
//! (`crate::paths::rulesets`), a folder per game (`rulesets/exe6/<name>.saitoruleset`), as tactics are kept. A
//! ruleset's file names the saito format identifier, schema version, game and title, whether each build must be one
//! version's, and what each category holds back, by the category's fact and the things' names in the game
//! (`saito_match::ruleset::Ruleset::banned_names`):
//!
//! ```json
//! {
//!   "format": "top.exessrhub.saito.ruleset.v1",
//!   "schema": 1,
//!   "game_id": "exe6",
//!   "title": "Tournament",
//!   "version_mode": "single_version",
//!   "restrictions": {
//!     "folder": ["bass", "deltaray"],
//!     "patch_cards": ["starman"]
//!   }
//! }
//! ```
//!
//! Files without `format == "top.exessrhub.saito.ruleset.v1"` and `schema == 1` (including legacy TOML ruleset files)
//! files, even if renamed to `.saitoruleset`) are strictly refused.
//!
//! A category kept per record by a field of its records (`saito_match::ruleset::Category::by`: EXE5's partners'
//! programs, by each partner's navi) has what it holds back for one key's records alone in `restrictions_by`, by the
//! category's fact, then the key's name, besides what `restrictions` holds back for every record.
//!
//! What it doesn't name is allowed. The presets are the game's (its rules/rulesets) and have no file.

use saito_battle::Content;
use saito_match::ruleset::{Name, Ruleset, Rulesets};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A ruleset file's extension (strict JSON format, incompatible with saito).
pub const EXTENSION: &str = "saitoruleset";

/// Required format identifier inside `.saitoruleset` files.
const FORMAT: &str = "top.exessrhub.saito.ruleset.v1";

/// Required schema version inside `.saitoruleset` files.
const SCHEMA: u32 = 1;

/// Each build one version's, as a file says it.
const ONE_PER_BUILD: &str = "single_version";
/// The versions mixed, as a file says it.
const MIXED: &str = "mixed_versions";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesetFile {
    format: String,
    schema: u32,
    game_id: String,
    title: String,
    version_mode: String,
    #[serde(default)]
    restrictions: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    restrictions_by: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    #[serde(default, skip_serializing_if = "saito_match::ruleset::Limits::is_empty")]
    limits: saito_match::ruleset::Limits,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    allow_ds: bool,
}

/// Check that the parsed file identifies saito's ruleset format and schema.
fn check_format(f: &RulesetFile) -> Result<(), String> {
    if f.format != FORMAT || f.schema != SCHEMA {
        return Err(format!("not a saito ruleset (format {:?}, schema {})", f.format, f.schema));
    }
    Ok(())
}

/// A ruleset's file, as it is listed: its path, its name, and its text (read against its game's rulesets once
/// they are at hand: [`read`]).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// The folder of game `game`'s rulesets.
pub fn folder(game: &str) -> PathBuf {
    crate::paths::rulesets().join(game)
}

/// Game `game`'s own rulesets, by name: each file of its folder that is a ruleset of it (a file that isn't one is
/// left out).
pub fn list(game: &str) -> Vec<Listed> {
    let Ok(dir) = saito_files::read_dir(folder(game)) else { return Vec::new() };
    let mut out: Vec<Listed> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| {
            let text = saito_files::read_to_string(&path).ok()?;
            let f: RulesetFile = serde_json::from_str(&text)
                .ok()
                .filter(|f: &RulesetFile| check_format(f).is_ok() && f.game_id == game)?;
            Some(Listed { path, name: f.title, text })
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.path.cmp(&b.path)));
    out
}

/// A ruleset file's text read against its game's rulesets: the ruleset (named as the player's own), or what keeps
/// it from being one (a category or a name the game hasn't), in the log's words.
pub fn read(rulesets: &Rulesets, content: &Content, text: &str) -> Result<Ruleset, String> {
    let f: RulesetFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    check_format(&f)?;
    if f.game_id != rulesets.game() {
        return Err(format!("a ruleset of {}, not of {}", f.game_id, rulesets.game()));
    }
    let one_per_build = match f.version_mode.as_str() {
        ONE_PER_BUILD => true,
        MIXED => false,
        other => return Err(format!("version_mode {other:?}: \"{ONE_PER_BUILD}\" or \"{MIXED}\"")),
    };
    let banned: Vec<(String, Vec<String>)> = f.restrictions.into_iter().collect();
    let banned_by: Vec<(String, Vec<(String, Vec<String>)>)> = f.restrictions_by.into_iter().map(|(fact, keys)| (fact, keys.into_iter().collect())).collect();
    let mut ruleset = Ruleset::of_names(rulesets, content, Name::Own(f.title), one_per_build, &banned, &banned_by)?;
    ruleset.limits = f.limits;
    ruleset.allow_ds = f.allow_ds;
    Ok(ruleset)
}

/// A ruleset as its `.saitoruleset` file's JSON text: the player's own, named as it is (a preset's id where it is one).
pub fn text(content: &Content, game: &str, ruleset: &Ruleset) -> String {
    let name = match &ruleset.name {
        Name::Own(name) | Name::Preset(name) => name.clone(),
        // (The other player's is never saved as it is: a copy of it is the player's own, named.)
        Name::Theirs => String::new(),
    };
    let f = RulesetFile {
        format: FORMAT.to_string(),
        schema: SCHEMA,
        game_id: game.to_string(),
        title: name,
        version_mode: if ruleset.one_per_build { ONE_PER_BUILD } else { MIXED }.to_string(),
        restrictions: ruleset.banned_names(content).into_iter().collect(),
        restrictions_by: ruleset.banned_by_names(content).into_iter().map(|(fact, keys)| (fact, keys.into_iter().collect())).collect(),
        limits: ruleset.limits.clone(),
        allow_ds: ruleset.allow_ds,
    };
    serde_json::to_string_pretty(&f).expect("a ruleset serializes") + "\n"
}

/// Write a ruleset to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, ruleset: &Ruleset) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        saito_files::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    saito_files::write(path, text(content, game, ruleset)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A file's name for a ruleset named `name`, without its extension: the name's letters and digits (`tournament`).
pub fn stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if stem.is_empty() { "ruleset".into() } else { stem }
}

/// A path for a new ruleset of `game` named `name` that no file has: its [`stem`] (`tournament.saitoruleset`), numbered if
/// it is taken.
pub fn new_path(game: &str, name: &str) -> PathBuf {
    let stem = stem(name);
    let dir = folder(game);
    let mut path = dir.join(format!("{stem}.{EXTENSION}"));
    for n in 2.. {
        if !saito_files::exists(&path) {
            break;
        }
        path = dir.join(format!("{stem}-{n}.{EXTENSION}"));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_match::testing::{exe6_content, rulesets};

    /// A ruleset's file reads back as the ruleset it was written from, by names; one of another game, or naming
    /// what the game hasn't, isn't one.
    #[test]
    fn a_ruleset_reads_back() {
        let content = exe6_content();
        let r = rulesets("exe6");
        let mut own = Ruleset::open(Name::Own("Gregar \"cup\"".into()));
        own.one_per_build = true;
        for c in r.categories.clone() {
            own.allow_groups(&r, &c, &["gregar"]);
        }
        own.set_allowed("folder", &saito_match::ruleset::Item::of(saito_match::ids::chip(&content, "exe6", "bass").unwrap().0), false);
        let text = text(&content, "exe6", &own);
        assert!(text.contains(FORMAT) && text.contains("\"version_mode\": \"single_version\"") && text.contains("bass"), "{text}");
        assert!(text.contains("tengucross"), "Falzar's Crosses are held back by name: {text}");
        assert_eq!(read(&r, &content, &text), Ok(own.clone()));
        assert!(read(&rulesets("exe5"), &saito_match::testing::exe5_content(), &text).unwrap_err().contains("not of exe5"));
        assert!(read(&r, &content, &text.replacen("bass", "nobody", 1)).is_err());
        assert!(read(&r, &content, &text.replacen("single_version", "sometimes", 1)).is_err());
        let mut mixed = r.preset("masters_mixed").unwrap().clone();
        mixed.name = Name::Own("Open".into());
        assert_eq!(read(&r, &content, &super::text(&content, "exe6", &mixed)), Ok(mixed));
    }

    /// Legacy TOML ruleset files and JSON payloads missing the exact saito format/schema are strictly rejected,
    /// even if given the `.saitoruleset` extension.
    #[test]
    fn a_legacy_ruleset_file_is_rejected() {
        let content = exe6_content();
        let r = rulesets("exe6");
        let legacy_toml = "game = \"exe6\"\nname = \"Legacy\"\nversions = \"one_per_build\"\n\n[banned]\nfolder = [\"bass\"]\n";
        assert!(read(&r, &content, legacy_toml).is_err());

        let mut own = r.preset("masters_mixed").unwrap().clone();
        own.name = Name::Own("Open".into());
        let valid = text(&content, "exe6", &own);
        let wrong_format = valid.replacen(FORMAT, "com.example.legacy.ruleset.v1", 1);
        assert!(read(&r, &content, &wrong_format).is_err());
        let wrong_schema = valid.replacen("\"schema\": 1", "\"schema\": 2", 1);
        assert!(read(&r, &content, &wrong_schema).is_err());

        let legacy_json = serde_json::json!({
            "game": "exe6",
            "name": "Legacy",
            "versions": "one_per_build",
            "banned": { "folder": ["bass"] },
        })
        .to_string();
        assert!(read(&r, &content, &legacy_json).is_err());
    }

    /// EXE5's partners' programs held back for every partner and for one navi's: `restrictions` and `restrictions_by`
    /// read back as they were; a navi the party can't hold (MegaMan) isn't a file's.
    #[test]
    fn a_ruleset_reads_back_its_bans_by_navi() {
        use saito_match::ids;
        use saito_match::ruleset::Item;
        let content = saito_match::testing::exe5_content();
        let r = rulesets("exe5");
        let mut own = r.preset("masters_mixed").unwrap().clone();
        own.name = Name::Own("Masters".into());
        let program = |n: &str| Item::of(ids::entry(&content, "exe5", "partycust_programs", n).unwrap().0);
        own.set_allowed("party.programs", &program("p-hp-300"), false);
        own.set_allowed_for(r.category("party.programs").unwrap(), ids::navi(&content, "exe5", "protoman").unwrap().0, &program("p-btlpk2"), false);
        let text = text(&content, "exe5", &own);
        assert!(text.contains("\"restrictions_by\"") && text.contains("\"protoman\"") && text.contains("\"party.programs\""), "{text}");
        assert_eq!(read(&r, &content, &text), Ok(own));
        assert!(read(&r, &content, &text.replacen("\"protoman\":", "\"megaman\":", 1)).is_err());
        assert!(!super::text(&content, "exe5", r.preset("masters_5ds_mixed").unwrap()).contains("restrictions_by"));
    }

    #[test]
    fn a_ruleset_reads_back_its_custom_limits() {
        let content = exe6_content();
        let r = rulesets("exe6");
        let mut own = r.preset("masters_mixed").unwrap().clone();
        own.name = Name::Own("Custom Limits".into());
        own.limits.mb_tier_copies[0] = Some(6);
        own.limits.mega_copies = Some(2);
        own.limits.mega_total = Some(8);
        own.limits.giga_copies = Some(2);
        own.limits.giga_total = Some(2);
        own.limits.patch_card_mb = Some(100);
        let saved = text(&content, "exe6", &own);
        assert!(saved.contains("\"limits\""), "{saved}");
        assert_eq!(read(&r, &content, &saved), Ok(own));
        assert!(!text(&content, "exe6", r.preset("masters_mixed").unwrap()).contains("\"limits\""));
    }
}
