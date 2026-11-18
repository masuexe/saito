//! `saito-content`: check a game's content and its assets, and publish it.
//!
//!     saito-content check <game> [--content DIR] [--roms DIR]
//!     saito-content publish <game> [--dry-run]
//!     saito-content lock [game ...]

#![forbid(unsafe_code)]

mod publish;

use saito_content::index;
use saito_content::report::{Level, Report};
use std::path::PathBuf;

struct Args {
    command: String,
    /// The game.
    game: String,
    /// The content root whose definitions `check` loads.
    content: PathBuf,
    /// The folder the game's ROMs are read from (none: `RomSource::from_env`).
    roms: Option<PathBuf>,
}

const USAGE: &str = "usage:
  saito-content check <game> [--content DIR] [--roms DIR]
                                                   define the game's content (default: $SAITO_CONTENT,
                                                   else a content/ beside the program, else this
                                                   repository's) and check its definitions, strings,
                                                   library order and look; then extract the game's
                                                   assets from its ROMs by its recipe (the folder
                                                   given: the Legacy Collection's, or one of ROM
                                                   files; default: $SAITO_ROMS, else the collection
                                                   where Steam installed it) and check them against
                                                   it: every asset it names, each sprite with its
                                                   animations and frames, its sprites timed as the
                                                   ROMs time them, its strings' backgrounds
";

fn parse() -> Result<Args, String> {
    let usage = || format!("{USAGE}{}\n{}", publish::USAGE, publish::LOCK_USAGE);
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or_else(usage)?;
    let game = it.next().ok_or_else(usage)?;
    let mut a = Args { command, game, content: index::content(), roms: None };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--content" => a.content = it.next().ok_or("--content needs a directory")?.into(),
            "--roms" => a.roms = Some(it.next().ok_or("--roms needs a directory")?.into()),
            f => return Err(format!("unknown argument {f}\n{}", usage())),
        }
    }
    Ok(a)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn print_report(r: &Report, verbose: bool) {
    for i in &r.issues {
        if verbose || i.level != Level::Note {
            eprintln!("{i}");
        }
    }
    let notes = r.count(Level::Note);
    eprintln!("{} errors, {} warnings, {notes} notes", r.count(Level::Error), r.count(Level::Warning));
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("publish") {
        publish::run(std::env::args().skip(2)).unwrap_or_else(|e| fail(e));
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("lock") {
        publish::lock(std::env::args().skip(2)).unwrap_or_else(|e| fail(e));
        return;
    }
    let a = parse().unwrap_or_else(|e| fail(e));
    match a.command.as_str() {
        "check" => {
            let mut r = Report::default();
            let game = &a.game;
            // The define phase: every module of the game loads, and what the
            // modules define fits together.
            let t = std::time::Instant::now();
            match saito_content::load::load_battle(&a.content, game) {
                Ok((c, loaded)) => {
                    r.issues.extend(loaded.issues);
                    r.note(
                        a.content.display().to_string(),
                        format!(
                            "{} modules define {} definitions ({:.1?})",
                            c.scripts.modules.len(),
                            c.defs.definitions.defs.len(),
                            t.elapsed()
                        ),
                    );
                    saito_content::lint::definitions(&c, &mut r);
                    // The game's assets, extracted from its ROMs by its recipe, which loaded with it.
                    let assets = extract(&a, &c, &mut r);
                    // The strings tables, against the definitions (and the backgrounds against the assets').
                    saito_content::locale::check_games(&a.content, &c, assets.as_ref().map(|e| &e.index), &mut r);
                    // The library order, against the definitions.
                    saito_content::library::check_games(&a.content, &c, &mut r);
                    // The look the app draws the game's menus in.
                    saito_content::look::check_games(&a.content, &c, &mut r);
                    if let Some(assets) = &assets {
                        check_assets(&c, assets, &mut r);
                    }
                }
                Err(failed) => r.issues.extend(failed.issues),
            }
            print_report(&r, true);
            if r.has_errors() {
                std::process::exit(1);
            }
        }
        _ => fail(format!("{USAGE}{}\n{}", publish::USAGE, publish::LOCK_USAGE)),
    }
}

/// The game's assets, extracted from its ROMs (the ROMs `--roms` names, else `RomSource::from_env`'s) by the recipe
/// of its content `c`, defined, with the content's custom assets (`saito_content::load::assets`); what the
/// extraction noticed as notes. None, and why as an error: the content gives no recipe that reads, or the ROMs aren't
/// there, or don't extract, or a custom asset's file isn't what its kind takes.
fn extract(a: &Args, c: &saito_battle::Content, r: &mut Report) -> Option<saito_extract::Extraction> {
    let t = std::time::Instant::now();
    let at = format!("{}'s assets", a.game);
    let roms = match &a.roms {
        Some(dir) => saito_extract::RomSource::at(dir).map_err(|e| e.to_string()),
        None => saito_extract::RomSource::from_env().map_err(|e| e.to_string()),
    };
    let extracted = roms.and_then(|roms| {
        let recipe = saito_content::load::recipe(c)?;
        saito_content::load::assets(c, &recipe, &roms)
    });
    match extracted {
        Ok(e) => {
            for w in &e.warnings {
                r.note(&at, w.clone());
            }
            let (g, sound) = (&e.graphics, &e.sound);
            let songs = sound.songs.iter().flatten().count();
            r.note(&at, format!("extracted {} sprites, {} backgrounds, {songs} songs ({:.1?})", g.sprites.len(), g.backgrounds.iter().flatten().count(), t.elapsed()));
            Some(e)
        }
        Err(e) => {
            r.error(at, e.to_string());
            None
        }
    }
}

/// The game's assets against its content `c`: every asset the content names is there
/// (`saito_content::load::asset_numbers`: else the game doesn't load).
fn check_assets(c: &saito_battle::Content, assets: &saito_extract::Extraction, r: &mut Report) {
    let at = format!("{}'s assets", c.game());
    match saito_content::load::asset_numbers(c, &assets.index) {
        Ok((_, found)) => {
            r.issues.extend(found.issues);
            r.note(at, format!("every asset the content names is there ({} sprites)", c.assets.sprites.len()));
        }
        Err(found) => r.issues.extend(found.issues),
    }
}
