//! Music library & favorites, Settings' Audio subscreen (`S-135`).
//!
//! Lists every BGM track across loaded games (EXE4, EXE5, EXE5DS, EXE6), lets the player preview
//! any track (including non-looping short jingles), star looping tracks for per-round battle
//! rotation, and pin a custom main-menu BGM.

use crate::input::{Action, KeyPress};
use crate::music::{self, CatalogEntry};
use crate::settings::FavoriteBgmMode;
use crate::shell::Shell;
use crate::sound::Cue;
use i_slint_backend_winit::winit::keyboard::KeyCode;
use saito_ui::{MusicTrackRow, MusicView, SettingsView, Tab};
use slint::{ComponentHandle, ModelRc, VecModel};

/// Focus view name for the music library subscreen.
const VIEW: &str = "settings.music";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Favorites,
    Exe4,
    Exe5,
    Exe5Ds,
    Exe6,
}

impl Filter {
    const ALL: [Filter; 6] = [
        Filter::All,
        Filter::Favorites,
        Filter::Exe4,
        Filter::Exe5,
        Filter::Exe5Ds,
        Filter::Exe6,
    ];

    fn as_index(self) -> usize {
        Self::ALL.iter().position(|&f| f == self).unwrap_or(0)
    }

    fn from_index(index: i32) -> Self {
        Self::ALL[index.rem_euclid(Self::ALL.len() as i32) as usize]
    }

    fn matches(self, entry: &CatalogEntry, shell: &Shell) -> bool {
        match self {
            Filter::All => true,
            Filter::Favorites => shell.settings.is_favorite_music(&entry.key),
            Filter::Exe4 => entry.game == "exe4" && !entry.is_ds,
            Filter::Exe5 => entry.game == "exe5" && !entry.is_ds,
            Filter::Exe5Ds => entry.game == "exe5" && entry.is_ds,
            Filter::Exe6 => entry.game == "exe6" && !entry.is_ds,
        }
    }
}

#[derive(Default)]
pub struct MusicLibrary {
    open: bool,
    filter: Filter,
    chosen: usize,
    /// Filtered entries currently displayed in `MusicView.rows`.
    entries: Vec<CatalogEntry>,
    /// Last preview key seen on screen, so UI refreshes automatically when a short jingle finishes.
    last_preview: Option<String>,
}

impl MusicLibrary {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, shell: &mut Shell) {
        self.open = true;
        shell.ui().global::<SettingsView>().set_music_library(true);
        shell.nav.enter(VIEW, "music.list");
        self.show(shell);
        if !self.entries.is_empty() {
            shell.nav.focus("music.list", self.chosen as i32);
        } else {
            shell.nav.focus("music.tab", self.filter.as_index() as i32);
        }
    }

    pub fn close(&mut self, shell: &mut Shell) {
        if self.open {
            shell.sound.stop_preview();
            self.last_preview = None;
            self.open = false;
            shell.ui().global::<SettingsView>().set_music_library(false);
        }
    }

    pub fn show(&mut self, shell: &Shell) {
        let all = music::catalog(&shell.games);
        let ui = shell.ui();
        let view = ui.global::<MusicView>();

        let count_for = |f: Filter| all.iter().filter(|e| f.matches(e, shell)).count();
        let fav_count = count_for(Filter::Favorites);
        let tab_all = view.get_tab_all().to_string();
        let tab_fav = view.get_tab_favorites().to_string();
        let tabs: Vec<Tab> = Filter::ALL
            .iter()
            .map(|&f| {
                let n = count_for(f);
                let label = match f {
                    Filter::All => format!("{tab_all} ({n})"),
                    Filter::Favorites => format!("★ {tab_fav} ({n})"),
                    Filter::Exe4 => format!("{} ({n})", music::game_badge("exe4", false)),
                    Filter::Exe5 => format!("{} ({n})", music::game_badge("exe5", false)),
                    Filter::Exe5Ds => format!("{} ({n})", music::game_badge("exe5", true)),
                    Filter::Exe6 => format!("{} ({n})", music::game_badge("exe6", false)),
                };
                let enabled = matches!(f, Filter::All | Filter::Favorites) || n > 0;
                Tab { text: label.into(), enabled, flagged: false }
            })
            .collect();

        // If current game filter has no loaded tracks, fall back to All.
        if !matches!(self.filter, Filter::All | Filter::Favorites) && count_for(self.filter) == 0 {
            self.filter = Filter::All;
        }

        self.entries = all.iter().filter(|e| self.filter.matches(e, shell)).cloned().collect();
        if self.entries.is_empty() {
            self.chosen = 0;
        } else {
            self.chosen = self.chosen.min(self.entries.len() - 1);
        }

        let preview_key = shell.sound.preview_key().map(str::to_string);
        self.last_preview = preview_key.clone();
        let menu_key = shell.settings.menu_bgm.as_deref();

        let rows: Vec<MusicTrackRow> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| MusicTrackRow {
                number: format!("{:02}", i + 1).into(),
                game: music::game_badge(&e.game, e.is_ds).into(),
                title: music::track_title(&e.game, &e.song).into(),
                looping: e.looping,
                favorite: shell.settings.is_favorite_music(&e.key),
                playing: preview_key.as_deref() == Some(e.key.as_str()),
                menu_bgm: menu_key == Some(e.key.as_str()),
            })
            .collect();

        view.set_tabs(ModelRc::new(VecModel::from(tabs)));
        view.set_filter(self.filter.as_index() as i32);
        view.set_rows(ModelRc::new(VecModel::from(rows)));
        view.set_chosen(if self.entries.is_empty() { -1 } else { self.chosen as i32 });
        view.set_total_count(all.len() as i32);
        view.set_favorite_count(fav_count as i32);
        view.set_favorite_battle_bgm(shell.settings.favorite_battle_bgm.as_index() as i32);
        view.set_has_custom_menu_bgm(menu_key.is_some());
        view.set_menu_bgm_label(
            menu_key
                .and_then(music::parse_track_key)
                .map(|(g, s)| music::track_full_name(g, s))
                .unwrap_or_default()
                .into(),
        );

        view.set_is_playing(preview_key.is_some());
        view.set_playing_label(
            preview_key
                .as_deref()
                .and_then(music::parse_track_key)
                .map(|(g, s)| music::track_full_name(g, s))
                .unwrap_or_default()
                .into(),
        );

        if let Some(sel) = self.entries.get(self.chosen) {
            view.set_has_selected(true);
            view.set_selected_game(music::game_badge(&sel.game, sel.is_ds).into());
            view.set_selected_title(music::track_title(&sel.game, &sel.song).into());
            view.set_selected_looping(sel.looping);
            view.set_selected_favorite(shell.settings.is_favorite_music(&sel.key));
            view.set_selected_menu_bgm(menu_key == Some(sel.key.as_str()));
            view.set_selected_playing(preview_key.as_deref() == Some(sel.key.as_str()));
        } else {
            view.set_has_selected(false);
            view.set_selected_game(Default::default());
            view.set_selected_title(Default::default());
            view.set_selected_looping(false);
            view.set_selected_favorite(false);
            view.set_selected_menu_bgm(false);
            view.set_selected_playing(false);
        }
    }

    fn toggle_play_at(&mut self, shell: &mut Shell, index: usize) {
        let Some(entry) = self.entries.get(index).cloned() else { return };
        self.chosen = index;
        if shell.sound.preview_key() == Some(entry.key.as_str()) {
            shell.sound.stop_preview();
        } else if let Some((theme, looping)) = music::resolve_preview(&shell.games, &entry.key) {
            shell.sound.start_preview(&entry.key, theme, looping);
        } else {
            shell.sound.play(Cue::Refused);
        }
        self.show(shell);
    }

    fn play_at(&mut self, shell: &mut Shell, index: usize) {
        let Some(entry) = self.entries.get(index).cloned() else { return };
        self.chosen = index;
        if let Some((theme, looping)) = music::resolve_preview(&shell.games, &entry.key) {
            shell.sound.start_preview(&entry.key, theme, looping);
        } else {
            shell.sound.play(Cue::Refused);
        }
        self.show(shell);
    }

    fn toggle_favorite_at(&mut self, shell: &mut Shell, index: usize) {
        let Some(entry) = self.entries.get(index).cloned() else { return };
        self.chosen = index;
        if !entry.looping {
            shell.sound.play(Cue::Refused);
            return;
        }
        shell.settings.toggle_favorite_music(&entry.key);
        shell.save_settings();
        shell.sound.play(Cue::Pick);
        self.show(shell);
    }

    fn toggle_menu_bgm_selected(&mut self, shell: &mut Shell) {
        let Some(entry) = self.entries.get(self.chosen).cloned() else { return };
        if shell.settings.menu_bgm.as_deref() == Some(entry.key.as_str()) {
            shell.settings.menu_bgm = None;
            shell.save_settings();
            shell.sync_menu_bgm();
            shell.sound.play(Cue::Pick);
            self.show(shell);
            return;
        }
        if !entry.looping {
            shell.sound.play(Cue::Refused);
            return;
        }
        shell.settings.menu_bgm = Some(entry.key);
        shell.save_settings();
        shell.sync_menu_bgm();
        shell.sound.play(Cue::Pick);
        self.show(shell);
    }

    fn set_filter(&mut self, shell: &mut Shell, filter: Filter) {
        let all = music::catalog(&shell.games);
        let enabled = matches!(filter, Filter::All | Filter::Favorites)
            || all.iter().any(|e| filter.matches(e, shell));
        if !enabled {
            shell.sound.play(Cue::Refused);
            return;
        }
        self.filter = filter;
        self.chosen = 0;
        self.show(shell);
    }

    fn step_filter(&mut self, shell: &mut Shell, by: i32) {
        let all = music::catalog(&shell.games);
        let cur = self.filter.as_index() as i32;
        for step in 1..=Filter::ALL.len() as i32 {
            let candidate = Filter::from_index(cur + by.signum() * step);
            let enabled = matches!(candidate, Filter::All | Filter::Favorites)
                || all.iter().any(|e| candidate.matches(e, shell));
            if enabled {
                self.filter = candidate;
                self.chosen = 0;
                shell.sound.play(Cue::Cursor);
                self.show(shell);
                if !self.entries.is_empty() {
                    shell.nav.focus("music.list", 0);
                }
                return;
            }
        }
    }

    fn set_battle_mode(&mut self, shell: &mut Shell, mode: FavoriteBgmMode) {
        shell.settings.favorite_battle_bgm = mode;
        shell.save_settings();
        shell.sound.play(Cue::Pick);
        self.show(shell);
    }

    pub fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        match id {
            "music.tab" => {
                let next = Filter::from_index(index);
                self.set_filter(shell, next);
            }
            "music.list" => {
                if let Ok(idx) = usize::try_from(index) {
                    self.toggle_play_at(shell, idx);
                }
            }
            "music.star" => {
                if let Ok(idx) = usize::try_from(index) {
                    self.toggle_favorite_at(shell, idx);
                }
            }
            "music.play" => {
                if !self.entries.is_empty() {
                    self.toggle_play_at(shell, self.chosen);
                }
            }
            "music.prev" => {
                if !self.entries.is_empty() {
                    let prev = (self.chosen + self.entries.len() - 1) % self.entries.len();
                    self.play_at(shell, prev);
                    shell.nav.focus("music.list", prev as i32);
                }
            }
            "music.next" => {
                if !self.entries.is_empty() {
                    let next = (self.chosen + 1) % self.entries.len();
                    self.play_at(shell, next);
                    shell.nav.focus("music.list", next as i32);
                }
            }
            "music.fav" => {
                if !self.entries.is_empty() {
                    self.toggle_favorite_at(shell, self.chosen);
                }
            }
            "music.menu-bgm" => self.toggle_menu_bgm_selected(shell),
            "music.reset-menu" => {
                shell.settings.menu_bgm = None;
                shell.save_settings();
                shell.sync_menu_bgm();
                shell.sound.play(Cue::Pick);
                self.show(shell);
            }
            "music.battle-mode" => {
                let next = FavoriteBgmMode::from_index(shell.settings.favorite_battle_bgm.as_index() as i32 + 1);
                self.set_battle_mode(shell, next);
            }
            "music.fav-battles" => {
                let all = music::catalog(&shell.games);
                let mut added = false;
                for entry in all {
                    if entry.is_battle_theme && entry.looping && !shell.settings.is_favorite_music(&entry.key) {
                        shell.settings.favorite_music.push(entry.key);
                        added = true;
                    }
                }
                if added {
                    shell.save_settings();
                    shell.sound.play(Cue::Pick);
                } else {
                    shell.sound.play(Cue::Refused);
                }
                self.show(shell);
            }
            "music.clear-favs" => {
                if !shell.settings.favorite_music.is_empty() {
                    shell.settings.favorite_music.clear();
                    shell.save_settings();
                    shell.sound.play(Cue::Pick);
                }
                self.show(shell);
            }
            _ => {}
        }
    }

    pub fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) {
        if id == "music.battle-mode" {
            let next = FavoriteBgmMode::from_index(shell.settings.favorite_battle_bgm.as_index() as i32 + by);
            self.set_battle_mode(shell, next);
        }
    }

    pub fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        if id == "music.battle-mode" {
            self.set_battle_mode(shell, FavoriteBgmMode::from_index(choice));
        }
    }

    pub fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if (id == "music.list" || id == "music.star")
            && let Ok(at) = usize::try_from(index)
            && at < self.entries.len()
            && at != self.chosen
        {
            self.chosen = at;
            self.show(shell);
        }
    }

    pub fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if key.repeat {
            return false;
        }
        if key.code == KeyCode::KeyF && !self.entries.is_empty() {
            self.toggle_favorite_at(shell, self.chosen);
            return true;
        }
        false
    }

    pub fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        match action {
            Action::Previous => {
                self.step_filter(shell, -1);
                true
            }
            Action::Next => {
                self.step_filter(shell, 1);
                true
            }
            Action::Alternate => {
                if !self.entries.is_empty() {
                    self.toggle_favorite_at(shell, self.chosen);
                }
                true
            }
            _ => false,
        }
    }

    pub fn frame(&mut self, shell: &mut Shell) {
        if !self.open {
            return;
        }
        if self.last_preview.as_deref() != shell.sound.preview_key() {
            self.show(shell);
        }
    }
}
