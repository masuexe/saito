//! Settings (docs/design/app-ui.md §6): General, Video, Audio, Online, Controls, About, on one scrolling page.
//! Every change is applied and kept at once (`crate::settings`). The About group's SHOW LICENSES opens About in
//! the page's place (`about`), and Back there comes back to the page.
//!
//! The match menu's Settings and Controls pages (`crate::screens::play`) change some of the same in a match: they
//! show them by `SettingsView` too ([`show_values`]), change them by the same functions ([`set_language`] and the
//! rest) and bind the controls by a [`Binding`] of their own.

mod about;

use crate::input::{Action, Bindings, KeyPress, Scroll, bind_cap, key_name, mouse_name, pad_cap, pad_name};
use crate::screens::{Screen, ScreenId};
use crate::settings::{DEFAULT_SIGNAL_SERVER, MAX_INPUT_DELAY, clean_name, clean_signaling_server, clean_signaling_server_draft};
use crate::shell::Shell;
use crate::sound::Cue;
use saito_frontend::game::Enhancements;
use saito_gamepad::Button;
use saito_ui::{PadBinding, PadButton, SettingsView};
use i_slint_backend_winit::winit::event::MouseButton;
use i_slint_backend_winit::winit::keyboard::KeyCode;
use slint::{ComponentHandle, ModelRc, VecModel};

/// Where a font's or a package's row is in About's list (the screenshot pass reads Slint's license).
#[cfg(any(test, feature = "tour"))]
pub(crate) use about::place_of;

/// Each group's first control, and the room over it for what the group has above it (its title; in Controls the
/// columns' heads, in About the version's row): where the list at the left jumps to.
const GROUPS: [(&str, f32); 6] = [("set.name", 40.0), ("set.picture", 40.0), ("set.music", 40.0), ("set.delay", 40.0), ("set.key", 68.0), ("set.folder", 92.0)];

/// The picture sizes offered besides the largest that fits: 1 to 8 times 240 by 160.
const SIZES: u32 = 8;

/// How far past a group's title a scroll reaches before marking that group, and how close to the page's end
/// marks About (whose section is shorter than a screen).
const SCROLL_TOP_LEAD: f32 = 24.0;
const SCROLL_BOTTOM_MARGIN: f32 = 16.0;

fn laid_out(view_height: f32, content_height: f32, tops: &[f32]) -> bool {
    !tops.is_empty() && view_height > 0.0 && content_height > 0.0 && !(tops.len() > 1 && tops.last().copied().unwrap_or(0.0) <= 0.0)
}

/// Which group the list at the left marks for a scroll `scroll` down a page `view_height` tall holding
/// `content_height` with group titles at `tops`.
pub fn group_for_scroll(scroll: f32, view_height: f32, content_height: f32, tops: &[f32]) -> i32 {
    if !laid_out(view_height, content_height, tops) {
        return 0;
    }
    let max_scroll = (content_height - view_height).max(0.0);
    if max_scroll > 0.5 && scroll >= max_scroll - SCROLL_BOTTOM_MARGIN {
        return (tops.len() - 1) as i32;
    }
    tops.iter().rposition(|&top| top <= scroll + SCROLL_TOP_LEAD).unwrap_or(0) as i32
}

/// Update `SettingsView.group` from the page's scroll position when Settings is on screen and laid out.
pub fn scrolled(ui: &saito_ui::AppWindow, scroll: f32, view_height: f32, content_height: f32, tops: &[f32]) {
    let view = ui.global::<SettingsView>();
    if view.get_about() || view.get_touch_editor() || !laid_out(view_height, content_height, tops) {
        return;
    }
    let group = group_for_scroll(scroll, view_height, content_height, tops);
    if view.get_group() != group {
        view.set_group(group);
    }
}

/// Mark the group `id` belongs to in the left list when interacting with a setting directly.
fn mark_group(shell: &Shell, id: &str) {
    if let Some(group) = group_of(id) {
        shell.ui().global::<SettingsView>().set_group(group);
    }
}

/// The group a control of the page is in.
fn group_of(id: &str) -> Option<i32> {
    Some(match id {
        "set.name" | "set.language" | "set.updates" | "set.check-now" => 0,
        "set.picture" | "set.fullscreen" | "set.status-bar" | "set.motion" | "set.reduce-flashes" | "set.hide-aura-number" | "set.text" | "set.hud" | "set.smooth" | "set.fill" | "set.hd-chip-art" | "set.hd-chip-art-import" => 1,
        "set.music" | "set.effects" => 2,
        "set.delay" | "set.server" | "set.auto-upload" => 3,
        "set.key" | "set.button" | "set.reset" | "set.keyboard-only" | "set.landscape-pad" | "set.touch-layout" => 4,
        "set.folder" | "set.abd-format" | "set.abd-export" | "set.nearby" | "set.nearby.send" | "set.about" => 5,
        _ => return None,
    })
}

/// The graphics enhancement a switch is of, by its control's last part (`set.text`, `menu.set.fill`).
fn enhancement<'a>(e: &'a mut Enhancements, id: &str) -> Option<&'a mut bool> {
    Some(match id.rsplit_once('.').map_or(id, |(_, last)| last) {
        "text" => &mut e.text,
        "hud" => &mut e.hud,
        "smooth" => &mut e.motion,
        "fill" => &mut e.fill,
        "reduce-flashes" => &mut e.reduce_flashes,
        "hide-aura-number" => &mut e.hide_aura_number,
        "hd-chip-art" => &mut e.hd_chip_art,
        _ => return None,
    })
}

/// Turn the graphics enhancement switch `id` is on or off (none: the other way, as Enter does), as the Settings
/// screen and the match's menu do.
pub fn set_enhancement(shell: &mut Shell, id: &str, on: Option<bool>) {
    let mut e = shell.settings.enhancements();
    if let Some(switch) = enhancement(&mut e, id) {
        *switch = on.unwrap_or(!*switch);
        shell.set_enhancements(e);
    }
}

/// A picture size as the list of sizes says it (1 to [`SIZES`]).
fn size_label(size: u32) -> String {
    format!("{size}× ({} × {})", 240 * size, 160 * size)
}

/// What the settings' controls show of the settings (`SettingsView`), and the bindings as `binding` asks.
pub fn show_values(shell: &Shell, binding: &Binding) {
    let ui = shell.ui();
    let view = ui.global::<SettingsView>();
    let s = &shell.settings;
    view.set_name(s.name.as_str().into());
    view.set_language(crate::lang::name(shell.i18n.code).into());
    view.set_picture_size(if s.picture_size == 0 { Default::default() } else { size_label(s.picture_size).into() });
    view.set_full_screen(s.full_screen);
    view.set_is_android(cfg!(target_os = "android"));
    view.set_hide_landscape_status_bar(s.hide_landscape_status_bar);
    view.set_reduce_motion(shell.reduces_motion());
    view.set_reduce_flashes(s.reduce_flashes);
    view.set_hide_aura_number(s.hide_aura_number);
    view.set_check_updates(s.check_updates);
    view.set_can_update(saito_updater::supported());
    view.set_keyboard_only_custom(s.keyboard_only_custom);
    view.set_landscape_touch_pad(s.landscape_touch_pad);
    view.set_sharp_text(s.sharp_text);
    view.set_sharp_hud(s.sharp_hud);
    view.set_smooth_motion(s.smooth_motion);
    view.set_fill_window(s.fill_window);
    view.set_hd_chip_art(s.hd_chip_art);
    view.set_has_hd_art(shell.games.has_vol2_dat());
    view.set_music(s.music as i32);
    view.set_effects(s.effects as i32);
    view.set_input_delay(s.input_delay as i32);
    view.set_signaling_server(s.signaling_server.as_str().into());
    view.set_default_signaling_server(DEFAULT_SIGNAL_SERVER.into());
    view.set_auto_upload_replays(s.auto_upload_replays);
    view.set_data_folder(crate::paths::data().display().to_string().into());
    view.set_can_send_nearby(saito_nearby::can_offer() && shell.games.rom_source().is_some());
    binding.show(shell);
}

/// The language's choices for its select (control `id`), the one shown marked.
pub fn pick_language(shell: &mut Shell, id: &str) {
    let offered = crate::lang::offered(shell.i18n.code);
    let options: Vec<String> = offered.iter().map(|(_, name)| name.to_string()).collect();
    let chosen = offered.iter().position(|(code, _)| *code == shell.i18n.code).unwrap_or(0);
    shell.pick(id, 0, options, chosen);
}

/// The language chosen from [`pick_language`]'s choices.
pub fn choose_language(shell: &mut Shell, choice: i32) {
    if let Some((code, _)) = crate::lang::offered(shell.i18n.code).get(choice.max(0) as usize) {
        set_language(shell, code);
    }
}

pub fn set_language(shell: &mut Shell, code: &'static str) {
    shell.settings.language = Some(code.to_string());
    shell.save_settings();
    shell.set_language(code);
}

/// The picture sizes for their select (control `id`): the largest that fits, then 1 to [`SIZES`] times.
pub fn pick_picture_size(shell: &mut Shell, id: &str) {
    let largest = shell.ui().global::<SettingsView>().get_largest().to_string();
    let options = std::iter::once(largest).chain((1..=SIZES).map(size_label)).collect();
    shell.pick(id, 0, options, shell.settings.picture_size as usize);
}

/// The picture size a step on from the one set, around from the largest to the last.
pub fn step_picture_size(shell: &mut Shell, by: i32) {
    set_picture_size(shell, (shell.settings.picture_size as i32 + by).rem_euclid(SIZES as i32 + 1) as u32);
}

pub fn set_picture_size(shell: &mut Shell, size: u32) {
    shell.settings.picture_size = size.min(SIZES);
    shell.save_settings();
    // (The picture on screen is fitted again at the next frame.)
    if let Some(b) = &mut shell.battle {
        b.stage.surface.stale = true;
    }
}

pub fn set_full_screen(shell: &mut Shell, on: bool) {
    shell.settings.full_screen = on;
    shell.save_settings();
    shell.ui().window().set_fullscreen(on);
}

pub fn set_keyboard_only_custom(shell: &mut Shell, on: bool) {
    shell.settings.keyboard_only_custom = on;
    shell.save_settings();
    if on {
        let ui = shell.ui();
        let s = ui.global::<saito_ui::Shell>();
        s.set_screen_pointer(false);
        s.set_picture_hand(false);
    }
}

/// The controls' bindings being changed: the button whose key or pad button is asked for, its box saying so. Enter
/// on a box asks; the next key (or pad button) pressed is bound to it, taking it from the button that had it, which
/// takes the one this had (`Bindings::bind_key`); Esc, a pad's button for a key or a key for a pad's button, or the
/// focus moving, asks no more.
#[derive(Default)]
pub struct Binding {
    /// The button whose key (false) or pad button (true) is being asked for.
    asking: Option<(usize, bool)>,
}

impl Binding {
    /// Ask for the key (`pad`: the pad's button) of the button at `index`.
    pub fn ask(&mut self, shell: &Shell, index: i32, pad: bool) {
        self.asking = Some((asked(index), pad));
        self.show(shell);
    }

    /// Ask no more. Whether it was asking.
    pub fn stop(&mut self, shell: &Shell) -> bool {
        let was = self.asking.take().is_some();
        if was {
            self.show(shell);
        }
        was
    }

    /// The bindings back to their defaults.
    pub fn reset(&mut self, shell: &mut Shell) {
        self.asking = None;
        shell.set_bindings(Bindings::default());
        self.show(shell);
    }

    /// The bindings, as the controls' boxes show them.
    pub fn show(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        let keys: Vec<slint::SharedString> = shell.bindings.keys.iter().map(|k| bind_cap(*k).into()).collect();
        let buttons: Vec<PadBinding> = shell
            .bindings
            .pad
            .iter()
            .map(|b| {
                let face = match b {
                    Button::South => PadButton::South,
                    Button::East => PadButton::East,
                    Button::West => PadButton::West,
                    Button::North => PadButton::North,
                    _ => PadButton::None,
                };
                PadBinding { face, text: pad_cap(*b).into() }
            })
            .collect();
        view.set_keys(ModelRc::new(VecModel::from(keys)));
        view.set_buttons(ModelRc::new(VecModel::from(buttons)));
        view.set_asking_key(self.asking.filter(|a| !a.1).map_or(-1, |a| a.0 as i32));
        view.set_asking_button(self.asking.filter(|a| a.1).map_or(-1, |a| a.0 as i32));
    }

    /// A key pressed while asking: bound, or the asking given up. False: it isn't asking.
    pub fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        let Some((button, pad)) = self.asking else { return false };
        if key.repeat {
            return true;
        }
        if key.code == KeyCode::Escape || pad {
            self.asking = None;
        } else if key_name(key.code).is_some() {
            let mut bindings = shell.bindings.clone();
            bindings.bind_key(button, key.code);
            shell.set_bindings(bindings);
            self.asking = None;
            shell.sound.play(Cue::Pick);
        } else {
            // (A key that can't be bound: the question stands.)
            shell.sound.play(Cue::Refused);
        }
        self.show(shell);
        true
    }

    /// A mouse button pressed while asking: bound, or asks no more if asking for a pad button.
    pub fn mouse(&mut self, shell: &mut Shell, button: MouseButton) -> bool {
        let Some((at, pad)) = self.asking else { return false };
        if pad {
            self.asking = None;
            self.show(shell);
            return false;
        }
        if mouse_name(button).is_some() {
            let mut bindings = shell.bindings.clone();
            bindings.bind_key(at, button);
            shell.set_bindings(bindings);
            self.asking = None;
            shell.sound.play(Cue::Pick);
        } else {
            shell.sound.play(Cue::Refused);
        }
        self.show(shell);
        true
    }

    /// A pad's button pressed while asking: bound, or the asking given up. False: it isn't asking.
    pub fn button(&mut self, shell: &mut Shell, button: Button) -> bool {
        let Some((at, pad)) = self.asking else { return false };
        if pad && pad_name(button).is_some() {
            let mut bindings = shell.bindings.clone();
            bindings.bind_pad(at, button);
            shell.set_bindings(bindings);
            shell.sound.play(Cue::Pick);
        }
        self.asking = None;
        self.show(shell);
        true
    }
}

/// The button a controls' row asks for: its place in [`crate::input::BUTTONS`], every one of them (the navi changes
/// after SELECT: the user, "changing the navi change buttons ends up changing the select button??", when the place
/// was kept to the GBA's ten).
fn asked(index: i32) -> usize {
    index.clamp(0, crate::input::BUTTONS.len() as i32 - 1) as usize
}

#[derive(Default)]
pub struct Settings {
    /// The controls' bindings being changed.
    binding: Binding,
    /// About, shown in the page's place while it is open.
    about: about::About,
    /// Interactive virtual touch pad layout editor, shown in the page's place while it is open.
    touch_editor: bool,
    /// Target `IP:port` entered for manual nearby transfer.
    nearby_target: String,
    /// Background `vol2.dat` compaction job started from Settings.
    importing_hd_art: Option<std::sync::mpsc::Receiver<Result<usize, String>>>,
    /// Selected export format for persistent ABD usage stats (`LOG` / `CSV` / `JSONL`).
    abd_format: crate::abd_stats::AbdExportFormat,
}

impl Settings {
    fn open_touch_editor(&mut self, shell: &mut Shell) {
        self.touch_editor = true;
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        view.set_touch_selected(0);
        view.set_touch_editor(true);
        shell.nav.enter("settings.touch", "");
    }

    fn close_touch_editor(&mut self, shell: &mut Shell) {
        if std::mem::take(&mut self.touch_editor) {
            shell.ui().global::<SettingsView>().set_touch_editor(false);
        }
    }

    fn try_send_nearby(&self, shell: &mut Shell) {
        if !crate::nearby::send_to(shell, &self.nearby_target) {
            shell.sound.play(Cue::Refused);
        }
    }

    fn set_reduce_motion(&mut self, shell: &mut Shell, reduce: bool) {
        shell.set_reduce_motion(reduce);
        self.show(shell);
    }

    /// The check for updates on or off: turned on, both game content and app updates are checked right away.
    fn set_check_updates(&mut self, shell: &mut Shell, on: bool) {
        shell.settings.check_updates = on;
        shell.save_settings();
        if on {
            shell.games.download(true);
            shell.check_updates_now(true);
        }
        self.show(shell);
    }

    fn set_enhancement(&mut self, shell: &mut Shell, id: &str, on: Option<bool>) {
        set_enhancement(shell, id, on);
        self.show(shell);
    }

    fn set_levels(&mut self, shell: &mut Shell, music: u32, effects: u32) {
        (shell.settings.music, shell.settings.effects) = (music.min(100), effects.min(100));
        shell.set_sound_levels(shell.settings.music, shell.settings.effects);
        shell.save_settings();
        self.show(shell);
    }

    fn show_abd_stats(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        let records = crate::abd_stats::load_all();
        let (total, gba, lc, nds) = crate::abd_stats::counts_summary(&records);
        view.set_abd_stats_format(self.abd_format.as_index());
        view.set_abd_stats_has_records(total > 0);
        let summary = if total == 0 {
            view.get_abd_stats_empty()
        } else {
            view.invoke_abd_stats_note(total as i32, gba as i32, lc as i32, nds as i32)
        };
        view.set_abd_stats_summary(summary);
    }

    fn export_abd_stats(&self, shell: &mut Shell) {
        let fmt = self.abd_format;
        let kind = shell.ui().global::<SettingsView>().get_abd_stats_files().to_string();
        let Some(path) = shell.choose_save(&kind, &[fmt.extension()], fmt.default_filename()) else { return };
        match crate::abd_stats::export_to(&path, fmt) {
            Ok(_) => shell.saved(&path),
            Err(why) => {
                crate::errors::log("exporting ABD usage stats", &why);
                shell.fail(crate::errors::UserError::Other);
            }
        }
    }
}

impl Screen for Settings {
    fn main(&self) -> &'static str {
        "set.name"
    }

    fn show(&mut self, shell: &mut Shell) {
        show_values(shell, &self.binding);
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        view.set_nearby_target(self.nearby_target.as_str().into());
        view.set_importing_hd_art(self.importing_hd_art.is_some());
        self.show_abd_stats(shell);
        if self.about.is_open() {
            self.about.show(shell);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        self.binding.stop(shell);
        self.close_touch_editor(shell);
        // (Shown again, Settings show their page, the focus on SHOW LICENSES.)
        if self.about.is_open() {
            self.about.close(shell);
        }
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if self.touch_editor {
            return;
        }
        if self.about.is_open() {
            return self.about.activate(shell, id, index);
        }
        mark_group(shell, id);
        match id {
            "group" => shell.nav.focus(GROUPS[index.clamp(0, GROUPS.len() as i32 - 1) as usize].0, 0),
            "set.language" => pick_language(shell, id),
            "set.picture" => pick_picture_size(shell, id),
            "set.fullscreen" => {
                set_full_screen(shell, !shell.settings.full_screen);
                self.show(shell);
            }
            "set.status-bar" => {
                shell.set_hide_landscape_status_bar(!shell.settings.hide_landscape_status_bar);
                self.show(shell);
            }
            "set.motion" => self.set_reduce_motion(shell, !shell.reduces_motion()),
            "set.updates" => self.set_check_updates(shell, !shell.settings.check_updates),
            "set.auto-upload" => {
                shell.settings.auto_upload_replays = !shell.settings.auto_upload_replays;
                shell.save_settings();
                self.show(shell);
            }
            "set.check-now" => {
                shell.games.download(true);
                shell.check_updates_now(true);
            }
            "set.reduce-flashes" | "set.hide-aura-number" | "set.text" | "set.hud" | "set.smooth" | "set.fill" | "set.hd-chip-art" => self.set_enhancement(shell, id, None),
            "set.hd-chip-art-import" => {
                if self.importing_hd_art.is_none() {
                    shell.choose_files("vol2.dat", &["dat"], "hd-art");
                }
            }
            "set.key" | "set.button" => self.binding.ask(shell, index, id == "set.button"),
            "set.reset" => self.binding.reset(shell),
            "set.keyboard-only" => {
                set_keyboard_only_custom(shell, !shell.settings.keyboard_only_custom);
                self.show(shell);
            }
            "set.landscape-pad" => {
                shell.set_landscape_touch_pad(!shell.settings.landscape_touch_pad);
                self.show(shell);
            }
            "set.touch-layout" => self.open_touch_editor(shell),
            "set.folder" => show_folder(&crate::paths::data()),
            "set.abd-format" => {
                self.abd_format = crate::abd_stats::AbdExportFormat::from_index((self.abd_format.as_index() + 1) % 3);
                self.show_abd_stats(shell);
            }
            "set.abd-export" => self.export_abd_stats(shell),
            "set.nearby.send" => self.try_send_nearby(shell),
            "set.about" => self.about.open(shell),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) {
        mark_group(shell, id);
        match id {
            "set.language" => set_language(shell, crate::lang::step(shell.i18n.code, by)),
            "set.picture" => {
                step_picture_size(shell, by);
                self.show(shell);
            }
            "set.fullscreen" => {
                set_full_screen(shell, by > 0);
                self.show(shell);
            }
            "set.status-bar" => {
                shell.set_hide_landscape_status_bar(by > 0);
                self.show(shell);
            }
            "set.motion" => self.set_reduce_motion(shell, by > 0),
            "set.updates" => self.set_check_updates(shell, by > 0),
            "set.auto-upload" => {
                shell.settings.auto_upload_replays = by > 0;
                shell.save_settings();
                self.show(shell);
            }
            "set.reduce-flashes" | "set.hide-aura-number" | "set.text" | "set.hud" | "set.smooth" | "set.fill" | "set.hd-chip-art" => self.set_enhancement(shell, id, Some(by > 0)),
            "set.music" => self.set_levels(shell, (shell.settings.music as i32 + 5 * by).clamp(0, 100) as u32, shell.settings.effects),
            "set.effects" => {
                self.set_levels(shell, shell.settings.music, (shell.settings.effects as i32 + 5 * by).clamp(0, 100) as u32);
                shell.sound.play(Cue::Cursor);
            }
            "set.delay" => {
                shell.settings.input_delay = (shell.settings.input_delay as i32 + by).clamp(0, MAX_INPUT_DELAY as i32) as u32;
                shell.save_settings();
                self.show(shell);
            }
            "set.keyboard-only" => {
                set_keyboard_only_custom(shell, by > 0);
                self.show(shell);
            }
            "set.landscape-pad" => {
                shell.set_landscape_touch_pad(by > 0);
                self.show(shell);
            }
            "set.abd-format" => {
                self.abd_format = crate::abd_stats::AbdExportFormat::from_index((self.abd_format.as_index() + by).rem_euclid(3));
                self.show_abd_stats(shell);
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        mark_group(shell, id);
        match id {
            "set.language" => choose_language(shell, choice),
            "set.picture" => {
                set_picture_size(shell, choice.max(0) as u32);
                self.show(shell);
            }
            "set.fullscreen" => {
                set_full_screen(shell, choice == 1);
                self.show(shell);
            }
            "set.status-bar" => {
                shell.set_hide_landscape_status_bar(choice == 1);
                self.show(shell);
            }
            "set.motion" => self.set_reduce_motion(shell, choice == 1),
            "set.updates" => self.set_check_updates(shell, choice == 1),
            "set.auto-upload" => {
                shell.settings.auto_upload_replays = choice == 1;
                shell.save_settings();
                self.show(shell);
            }
            "set.reduce-flashes" | "set.hide-aura-number" | "set.text" | "set.hud" | "set.smooth" | "set.fill" | "set.hd-chip-art" => self.set_enhancement(shell, id, Some(choice == 1)),
            "set.music" => self.set_levels(shell, choice.clamp(0, 100) as u32, shell.settings.effects),
            "set.effects" => self.set_levels(shell, shell.settings.music, choice.clamp(0, 100) as u32),
            "set.keyboard-only" => {
                set_keyboard_only_custom(shell, choice == 1);
                self.show(shell);
            }
            "set.landscape-pad" => {
                shell.set_landscape_touch_pad(choice == 1);
                self.show(shell);
            }
            "set.abd-format" => {
                self.abd_format = crate::abd_stats::AbdExportFormat::from_index(choice);
                self.show_abd_stats(shell);
            }
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, _index: i32, text: &str, done: bool) {
        mark_group(shell, id);
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        match id {
            "set.name" => {
                let name = clean_name(text);
                view.set_name(name.as_str().into());
                if done {
                    shell.set_name(&name);
                }
            }
            "set.server" => {
                let server = if done { clean_signaling_server(text) } else { clean_signaling_server_draft(text) };
                view.set_signaling_server(server.as_str().into());
                if done {
                    shell.settings.signaling_server = server;
                    shell.save_settings();
                }
            }
            "set.nearby" => {
                let clean: String = text.chars().filter(|c| !c.is_control() && !c.is_whitespace()).collect();
                self.nearby_target = clean;
                view.set_nearby_target(self.nearby_target.as_str().into());
            }
            _ => {}
        }
    }

    fn submitted(&mut self, shell: &mut Shell, id: &str, _index: i32) {
        if id == "set.nearby" {
            self.try_send_nearby(shell);
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if self.touch_editor {
            return;
        }
        if self.about.is_open() {
            return self.about.focused(shell, id, index);
        }
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        if id == "group" {
            // The list's row under the cursor: its group comes to the page's top.
            view.set_group(index);
            let (first, room) = GROUPS[index.clamp(0, GROUPS.len() as i32 - 1) as usize];
            shell.nav.reveal_at_top(&ui, first, 0, room);
        } else if let Some(group) = group_of(id) {
            view.set_group(group);
        }
        self.binding.stop(shell);
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        self.binding.key(shell, key)
    }

    fn mouse(&mut self, shell: &mut Shell, button: MouseButton) -> bool {
        self.binding.mouse(shell, button)
    }

    fn scroll(&mut self, shell: &mut Shell, action: Action) -> bool {
        self.about.is_open() && self.about.scroll(shell, action)
    }

    fn scroll_view(&mut self, shell: &mut Shell, by: Scroll) -> Option<bool> {
        self.about.is_open().then(|| self.about.scroll_view(shell, by)).flatten()
    }

    /// Back in About or TouchLayoutEditor: within it, else to the settings.
    fn back(&mut self, shell: &mut Shell) -> bool {
        if self.touch_editor {
            self.close_touch_editor(shell);
            shell.save_settings();
            shell.nav.enter(ScreenId::Settings.name(), self.main());
            return true;
        }
        if !self.about.is_open() {
            return false;
        }
        if !self.about.back(shell) {
            self.about.close(shell);
            shell.nav.enter(ScreenId::Settings.name(), self.main());
        }
        true
    }

    fn button(&mut self, shell: &mut Shell, button: Button) -> bool {
        self.binding.button(shell, button)
    }

    fn chosen(&mut self, shell: &mut Shell, purpose: &str, paths: &[std::path::PathBuf]) {
        if purpose != "hd-art" || self.importing_hd_art.is_some() {
            return;
        }
        let Some(src) = paths.first().cloned() else { return };
        let dst = crate::games::copies().join(saito_extract::hd_art::VOL2_FILE);
        let cleanup_picked = !saito_dialogs::FOLDERS_READ_AGAIN;
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = saito_threads::spawn("importing vol2.dat for HD chip art", move || {
            let result = saito_extract::hd_art::compact_vol2(&src, &dst).map_err(|e| e.to_string());
            if cleanup_picked {
                if let Some(parent) = src.parent() {
                    let _ = std::fs::remove_dir_all(parent);
                } else {
                    let _ = std::fs::remove_file(&src);
                }
            }
            let _ = tx.send(result);
        });
        match spawned {
            Ok(_) => {
                self.importing_hd_art = Some(rx);
                self.show(shell);
            }
            Err(why) => {
                crate::errors::log("importing vol2.dat", &why);
                shell.sound.play(Cue::Refused);
            }
        }
    }

    fn frame(&mut self, shell: &mut Shell, _now: web_time::Instant, shown: bool) {
        let Some(rx) = &self.importing_hd_art else { return };
        match rx.try_recv() {
            Ok(Ok(count)) => {
                self.importing_hd_art = None;
                crate::errors::log("importing vol2.dat", &format_args!("compacted {count} HD chip art entries"));
                shell.games.reload_all();
                shell.sound.play(Cue::Pick);
                if shown {
                    self.show(shell);
                }
            }
            Ok(Err(why)) => {
                self.importing_hd_art = None;
                crate::errors::log("importing vol2.dat", &why);
                shell.sound.play(Cue::Refused);
                if shown {
                    self.show(shell);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.importing_hd_art = None;
                shell.sound.play(Cue::Refused);
                if shown {
                    self.show(shell);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }
}

/// Show a folder in the system's file manager.
fn show_folder(path: &std::path::Path) {
    let _ = saito_files::create_dir_all(path);
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(why) = std::process::Command::new(program).arg(path).spawn() {
        crate::errors::log("showing the data folder", &why);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{BUTTONS, SWITCH_LEFT, SWITCH_RIGHT, button_index};
    use saito_battle::input::keys;

    /// Each controls' row asks for its own button: the navi changes theirs, not SELECT's.
    #[test]
    fn each_row_asks_for_its_own_button() {
        for (i, (bit, _)) in BUTTONS.iter().enumerate() {
            assert_eq!(asked(i as i32), button_index(*bit));
        }
        assert_eq!(BUTTONS[asked(10)].0, SWITCH_LEFT);
        assert_eq!(BUTTONS[asked(11)].0, SWITCH_RIGHT);
        assert_eq!(BUTTONS[asked(9)].0, keys::SELECT);
    }

    const SAMPLE_TOPS: [f32; 6] = [0.0, 234.0, 818.0, 962.0, 1106.0, 1882.0];
    const SAMPLE_CONTENT: f32 = 2120.0;

    #[test]
    fn scroll_spy_tracks_group_transitions() {
        let view = 650.0;
        assert_eq!(group_for_scroll(0.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(209.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(210.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 1);
        assert_eq!(group_for_scroll(793.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 1);
        assert_eq!(group_for_scroll(794.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 2);
        assert_eq!(group_for_scroll(938.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 3);
        assert_eq!(group_for_scroll(1082.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 4);
    }

    #[test]
    fn scroll_spy_marks_about_at_bottom_and_restores_controls_above_it() {
        let view = 650.0;
        let max_scroll = SAMPLE_CONTENT - view;
        assert_eq!(group_for_scroll(max_scroll - 20.0, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 4);
        assert_eq!(group_for_scroll(max_scroll - SCROLL_BOTTOM_MARGIN, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 5);
        assert_eq!(group_for_scroll(max_scroll, view, SAMPLE_CONTENT, &SAMPLE_TOPS), 5);
        // Short landscape viewport where About's title itself reaches top before the bottom margin.
        assert_eq!(group_for_scroll(1860.0, 200.0, SAMPLE_CONTENT, &SAMPLE_TOPS), 5);
    }

    #[test]
    fn scroll_spy_handles_non_scrollable_overscroll_and_unlaid_out_pages() {
        assert_eq!(group_for_scroll(0.0, 2500.0, SAMPLE_CONTENT, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(-12.0, 650.0, SAMPLE_CONTENT, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(3000.0, 650.0, SAMPLE_CONTENT, &SAMPLE_TOPS), 5);
        assert_eq!(group_for_scroll(0.0, 0.0, SAMPLE_CONTENT, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(100.0, 650.0, 0.0, &SAMPLE_TOPS), 0);
        assert_eq!(group_for_scroll(100.0, 650.0, SAMPLE_CONTENT, &[0.0, 0.0, 0.0]), 0);
        assert_eq!(group_for_scroll(100.0, 650.0, SAMPLE_CONTENT, &[]), 0);
    }
}
