//! Architecture ratchet guard: prevent new hardcoded game IDs (`"exe4"`,
//! `"exe5"`, `"exe6"`, etc.) from spreading across game-agnostic crates.
//!
//! Companion to `crates/saito-battle/tests/no_compat.rs`.
//! Game-specific behavior should be declared by content packs / capabilities
//! rather than branched on literal game IDs in Rust code.
//!
//! Existing occurrences are tracked in [`ALLOWLIST`] with their exact line
//! counts:
//! - Adding a new hardcoded game ID anywhere outside `*-compat` fails the test.
//! - Removing an existing one fails with a prompt to tighten [`ALLOWLIST`], so
//!   coupling only ever moves downward.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Literal game ID tokens forbidden in game-agnostic production code.
const GAME_TOKENS: &[&str] = &[
    "\"exe1\"", "\"exe1:",
    "\"exe2\"", "\"exe2:",
    "\"exe3\"", "\"exe3:",
    "\"exe4\"", "\"exe4:",
    "\"exe4.5\"", "\"exe4.5:",
    "\"exe5\"", "\"exe5:",
    "\"exe6\"", "\"exe6:",
];

/// Current baseline of production files containing hardcoded game IDs and their
/// exact matching line counts (relative to the workspace root, `/` separators).
///
/// Shrink or remove entries as each site is migrated to capability-driven logic.
const ALLOWLIST: &[(&str, usize)] = &[
    // Centralized capability & simulation-epoch registry (single source of truth)
    ("crates/saito-match/src/capabilities.rs", 9),
    // Extraction / HD art (Vol. 1 + Vol. 2 base hashes, DP2)
    ("crates/saito-extract/src/hd_art.rs", 9),
    ("crates/saito-extract/src/lib.rs", 1),
    // Remaining game-specific rule/data tables (DP5 — Coupling C)
    ("crates/saito-match/src/abd_mode.rs", 5),
    ("crates/saito-match/src/cs_prob_fix.rs", 7),
    ("crates/saito-match/src/random_battle.rs", 6),
    // Remaining UI shell entries (DP7 — Coupling C)
    ("crates/saito/src/games.rs", 1),
    ("crates/saito/src/rulesets/mod.rs", 12),
    ("crates/saito/src/screens/play/stage.rs", 6),
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Collect module files and directories declared behind `#[cfg(test)] mod <name>;`.
fn collect_cfg_test_modules(files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut test_paths = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(file).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let parent = file.parent().expect("source parent dir");
        for pair in lines.windows(2) {
            if !pair[0].trim().starts_with("#[cfg(test)]") {
                continue;
            }
            let next = pair[1].trim();
            let rest = next
                .strip_prefix("pub mod ")
                .or_else(|| next.strip_prefix("mod "));
            if let Some(rest) = rest
                && let Some(mod_name) = rest.strip_suffix(';')
            {
                let mod_name = mod_name.trim();
                test_paths.insert(parent.join(format!("{mod_name}.rs")));
                test_paths.insert(parent.join(mod_name));
            }
        }
    }
    test_paths
}

fn is_test_only_file(path: &Path, cfg_test_mods: &BTreeSet<PathBuf>) -> bool {
    if matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("tests.rs" | "testing.rs")
    ) {
        return true;
    }
    cfg_test_mods
        .iter()
        .any(|tp| path == tp || path.starts_with(tp))
}

fn scan_production_hits(path: &Path) -> Vec<(usize, String)> {
    let text = std::fs::read_to_string(path).expect("read source");
    let mut hits = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        let code = line.split("//").next().unwrap_or("");
        if GAME_TOKENS.iter().any(|tok| code.contains(tok)) {
            hits.push((idx + 1, trimmed.to_string()));
        }
    }
    hits
}

fn rel_slash(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path under workspace root")
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[test]
fn no_new_hardcoded_game_ids_in_production_code() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let crates_dir = workspace_root.join("crates");

    let mut all_files = Vec::new();
    for entry in std::fs::read_dir(&crates_dir).expect("read crates/") {
        let crate_dir = entry.expect("crate entry").path();
        if !crate_dir.is_dir() {
            continue;
        }
        let crate_name = crate_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if crate_name.ends_with("-compat") {
            continue;
        }
        collect_rs_files(&crate_dir.join("src"), &mut all_files);
    }
    all_files.sort();
    assert!(all_files.len() > 100, "unexpectedly few source files: {}", all_files.len());

    let cfg_test_mods = collect_cfg_test_modules(&all_files);

    let mut actual: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for file in &all_files {
        if is_test_only_file(file, &cfg_test_mods) {
            continue;
        }
        let hits = scan_production_hits(file);
        if !hits.is_empty() {
            actual.insert(rel_slash(&workspace_root, file), hits);
        }
    }

    let allowed: BTreeMap<&str, usize> = ALLOWLIST.iter().copied().collect();
    let mut errors = Vec::new();

    // 1. Disallow new files or increased hit counts.
    for (file, hits) in &actual {
        match allowed.get(file.as_str()) {
            None => {
                errors.push(format!(
                    "NEW hardcoded game ID in `{file}` ({} hits):\n{}",
                    hits.len(),
                    hits.iter()
                        .map(|(ln, txt)| format!("    L{ln}: {txt}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
            Some(&max) if hits.len() > max => {
                errors.push(format!(
                    "INCREASED hardcoded game IDs in `{file}`: {actual_n} > allowed {max}:\n{}",
                    hits.iter()
                        .map(|(ln, txt)| format!("    L{ln}: {txt}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    actual_n = hits.len(),
                ));
            }
            Some(_) => {}
        }
    }

    // 2. Ratchet downward: prompt to tighten ALLOWLIST when hits are removed.
    for (&file, &max) in &allowed {
        let count = actual.get(file).map_or(0, Vec::len);
        if count < max {
            errors.push(format!(
                "TIGHTEN ALLOWLIST for `{file}`: actual {count} < allowlisted {max} (nice! update ALLOWLIST in no_game_hardcoding.rs)"
            ));
        }
    }

    assert!(
        errors.is_empty(),
        "Hardcoded game ID ratchet check failed:\n\n{}",
        errors.join("\n\n")
    );
}
