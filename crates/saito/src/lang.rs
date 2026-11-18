//! The languages (docs/app.md §3): the app's own strings come from the catalogs bundled into the UI (`saito-ui`'s
//! `build.rs`: Slint's `@tr`), and the content's names from its strings tables (its locales), in the same
//! language (with Korean battle strings falling back to Japanese `ja`). Only CJK languages (`zh-Hans`, `ja`,
//! `ko`) are bundled; the default fallback is `zh-Hans`.

pub use saito_ui::LANGUAGES;

/// The fallback language when a system locale or setting doesn't match any bundled CJK language.
pub const DEFAULT: &str = "zh-Hans";

/// The languages Settings offers while `shown` is the one shown.
pub fn offered(_shown: &str) -> Vec<(&'static str, &'static str)> {
    LANGUAGES.to_vec()
}

/// The language the system asks for, if it is bundled (`zh*` -> `zh-Hans`, `ja*` -> `ja`, `ko*` -> `ko`), else
/// [`DEFAULT`] (`zh-Hans`).
pub fn system() -> &'static str {
    of_locale(&sys_locale::get_locale().unwrap_or_default())
}

fn of_locale(wanted: &str) -> &'static str {
    let primary = wanted.split(['-', '_', '.', '@']).next().unwrap_or("").to_ascii_lowercase();
    let code = match primary.as_str() {
        "zh" => "zh-Hans",
        "ja" => "ja",
        "ko" => "ko",
        _ => DEFAULT,
    };
    bundled(code).unwrap_or(DEFAULT)
}

/// The bundled language of code `code`, if there is one.
pub fn bundled(code: &str) -> Option<&'static str> {
    LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == code)
}

/// The first language shown: `SAITO_LANG`/`SAITO_LANG`'s, else the settings', else the system's.
pub fn initial(kept: Option<&str>) -> &'static str {
    let asked = std::env::var("SAITO_LANG").ok().or_else(|| std::env::var("SAITO_LANG").ok());
    first(asked.as_deref(), kept)
}

fn first(asked: Option<&str>, kept: Option<&str>) -> &'static str {
    asked.and_then(bundled).or_else(|| kept.and_then(bundled)).unwrap_or_else(system)
}

/// A language's writing rules, made once when it is chosen: its code, its capitals and its lists.
pub struct I18n {
    /// The language's code.
    pub code: &'static str,
    language: icu_locale_core::LanguageIdentifier,
    caps: icu_casemap::CaseMapperBorrowed<'static>,
    lists: icu_list::ListFormatter,
}

impl I18n {
    /// The rules of the language `code`.
    pub fn new(code: &'static str) -> I18n {
        use icu_list::options::{ListFormatterOptions, ListLength};
        let language = code.parse::<icu_locale_core::LanguageIdentifier>().unwrap_or(icu_locale_core::LanguageIdentifier::UNKNOWN);
        // CLDR's "and" list at its narrow length: it has no "and" before the last ("A, B, C", "A、B、C").
        let narrow = ListFormatterOptions::default().with_length(ListLength::Narrow);
        // (The data built in has every language's lists, and the one every language shares under them.)
        let lists = icu_list::ListFormatter::try_new_and((&language).into(), narrow).expect("the lists' data is built in");
        I18n { code, language, caps: icu_casemap::CaseMapper::new(), lists }
    }

    /// `text` in capitals by the language's rules (Unicode's).
    pub fn upper(&self, text: &str) -> String {
        self.caps.uppercase_to_string(text, &self.language).into_owned()
    }

    /// `items` one after the other by the language's own separator.
    pub fn list<S: AsRef<str>>(&self, items: &[S]) -> String {
        self.lists.format(items.iter().map(AsRef::as_ref)).to_string()
    }
}

/// Show the app in the language `code`: its rules made (`I18n`) and given to the UI's `I18n`, then its
/// strings from its catalog.
pub fn select(ui: &saito_ui::AppWindow, code: &'static str) -> std::rc::Rc<I18n> {
    use slint::ComponentHandle;
    let i18n = std::rc::Rc::new(I18n::new(code));
    let rules = i18n.clone();
    let ui_i18n = ui.global::<saito_ui::I18n>();
    ui_i18n.on_uppercased(move |text| rules.upper(&text).into());
    let rules = i18n.clone();
    ui_i18n.on_joined(move |items| rules.list(&slint::Model::iter(&items).collect::<Vec<_>>()).into());
    let _ = slint::select_bundled_translation(code);
    let theme = ui.global::<saito_ui::Theme>();
    theme.set_cjk(cjk(code));
    theme.set_body_face(saito_render::vfont::Cut::of(code).family().into());
    theme.set_cjk_display(display_face(code).into());
    ui_i18n.set_language(ui_i18n.get_language().wrapping_add(1));
    i18n
}

/// Whether `code`'s script is CJK (every bundled language is CJK: `zh-Hans`, `ja`, `ko`).
pub fn cjk(code: &str) -> bool {
    matches!(code, "ja" | "zh-Hans" | "ko")
}

/// The display face of a CJK language: Corporate Logo for Japanese; for Chinese and Korean, the body's family
/// at its heaviest (Noto Sans SC, Noto Sans KR).
pub fn display_face(code: &str) -> &'static str {
    match code {
        "zh-Hans" | "ko" => saito_render::vfont::Cut::of(code).family(),
        _ => "Corporate Logo ver3",
    }
}

/// Give the UI the bundled Noto Sans CJK (JP, SC, KR) as `body-face` and fallbacks for hanzi, kana, and
/// hangul, plus M PLUS 1 Code (`mono`).
pub fn register_fonts() {
    use saito_render::vfont::Cut;
    use slint::fontique_011::fontique;
    let mut collection = slint::fontique_011::shared_collection();
    let mut families = Vec::new();
    for cut in [Cut::Japanese, Cut::Simplified, Cut::Korean] {
        let blob = fontique::Blob::new(std::sync::Arc::new(cut.data()));
        families.extend(collection.register_fonts(blob, None).into_iter().map(|(family, _)| family));
    }
    for script in ["Hani", "Hira", "Kana", "Hang"] {
        collection.append_fallbacks(fontique::FallbackKey::new(fontique::Script::from_str_unchecked(script), None), families.iter().copied());
    }
    collection.register_fonts(fontique::Blob::new(std::sync::Arc::new(saito_render::bundled::m_plus_1_code())), None);
}

/// The language's own name.
pub fn name(code: &str) -> &'static str {
    LANGUAGES.iter().find(|(c, _)| *c == code).map_or("简体中文", |(_, n)| n)
}

/// The language after (or before) `code` in the list Settings offers, around from its end.
pub fn step(code: &str, by: i32) -> &'static str {
    let list = offered(code);
    let at = list.iter().position(|(c, _)| *c == code).unwrap_or(0) as i32;
    list[(at + by).rem_euclid(list.len() as i32) as usize].0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only CJK languages (`zh-Hans`, `ja`, `ko`) are bundled; English, Traditional Chinese, and
    /// pseudo-locales are absent.
    #[test]
    fn only_cjk_languages_are_bundled() {
        let codes: Vec<&str> = LANGUAGES.iter().map(|(c, _)| *c).collect();
        assert_eq!(codes, ["zh-Hans", "ja", "ko"]);
        assert!(bundled("en").is_none());
        assert!(bundled("zh-Hant").is_none());
        assert!(bundled("pseudo").is_none());
        assert_eq!(name("zh-Hans"), "简体中文");
        assert_eq!(name("ja"), "日本語");
        assert_eq!(name("ko"), "한국어");
    }

    /// Capitals and lists follow ICU rules for each CJK language.
    #[test]
    fn capitals_and_lists_by_language() {
        let (zh, ja, ko) = (I18n::new("zh-Hans"), I18n::new("ja"), I18n::new("ko"));
        assert_eq!(zh.upper("ab"), "AB");
        assert_eq!(ja.upper("ファイルを表示"), "ファイルを表示");
        assert_eq!(ko.upper("saito"), "SAITO");
        assert_eq!(ja.list(&["スーパーアーマー", "エアシューズ"]), "スーパーアーマー、エアシューズ");
        assert_eq!(zh.list(&["超级盔甲", "浮空鞋"]), "超级盔甲、浮空鞋");
        assert_eq!(ko.list(&["A", "B", "C"]), "A, B 및 C");
    }

    /// The system's locale maps to `zh-Hans`, `ja`, or `ko`, defaulting to `zh-Hans` for non-CJK locales.
    #[test]
    fn the_systems_language() {
        assert_eq!(of_locale("zh-CN"), "zh-Hans");
        assert_eq!(of_locale("zh-TW"), "zh-Hans");
        assert_eq!(of_locale("zh-Hant-HK"), "zh-Hans");
        assert_eq!(of_locale("zh"), "zh-Hans");
        assert_eq!(of_locale("ja-JP"), "ja");
        assert_eq!(of_locale("ko-KR"), "ko");
        assert_eq!(of_locale("en-US"), DEFAULT);
        assert_eq!(of_locale("fr-FR"), DEFAULT);
        assert_eq!(of_locale(""), DEFAULT);
        assert!(cjk("zh-Hans") && cjk("ja") && cjk("ko") && !cjk("en"));
        assert_eq!(display_face("zh-Hans"), "Noto Sans SC");
        assert_eq!(display_face("ko"), "Noto Sans KR");
        assert_eq!(display_face("ja"), "Corporate Logo ver3");
        assert_eq!(first(None, Some("en")), of_locale(&sys_locale::get_locale().unwrap_or_default()));
        assert_eq!(first(None, Some("ko")), "ko");
        assert_eq!(step("zh-Hans", 1), "ja");
        assert_eq!(step("ja", 1), "ko");
        assert_eq!(step("ko", 1), "zh-Hans");
    }
}
