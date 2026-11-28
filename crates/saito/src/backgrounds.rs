//! The battle's background (docs/app.md §1, "The battle's background"): one pool of every game's backgrounds,
//! any of which a battle of any game may be drawn on. The user: "is it possible to use e.g. exe4 backgrounds in
//! exe5? as in if they all came from the same pool of backgrounds?", then "sure do that"; and, asked whether
//! both players of an online match see the round's one or each their own, "Shared, as now".
//!
//! What the pool is is the content's (`Content::background_pool`, from each game's locales, the same whatever
//! packs are installed), and a round's background is picked and carried by the library (`saito_match`'s
//! `BackgroundPick::Pool`, the lobby's settings, a replay's match); this names each for a list: its game's name
//! in the language shown, then its own ("BN4 ACDC Area", "EXE4 秋原エリア"), from the content's locales alone, so
//! a game whose pack isn't there has its names too.

use crate::shell::Shell;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// The backgrounds' names in the language shown, by game and name in its game: [`name_backgrounds`]'.
    static NAMES: RefCell<HashMap<(String, String), String>> = RefCell::new(HashMap::new());
}

/// Read the backgrounds' names of `games` in `lang`, for [`name`]: from the content's locales alone, at start-up
/// and when the language changes (`Shell::read_game_names`).
///
/// Non-playable Legacy Collection Vol. 1 titles (`saito_extract::EXTRA_BG_GAMES`, e.g. `exe1`, `exe2`) receive
/// built-in localized titles whenever no playable `content/<game>` pack already supplied names for that game.
pub fn name_backgrounds<'a>(games: impl IntoIterator<Item = &'a str>, lang: &str) {
    let mut names = HashMap::new();
    for game in games {
        let Some(dir) = crate::contents::dir(game) else { continue };
        for (background, name) in saito_content::locale::background_names(&dir, game, lang) {
            names.insert((game.to_string(), background), name);
        }
    }
    for &(extra_game, specs) in saito_extract::EXTRA_BG_GAMES {
        for spec in specs {
            names
                .entry((extra_game.to_string(), spec.key.to_string()))
                .or_insert_with(|| extra_background_name(extra_game, spec.key, lang).unwrap_or(spec.key).to_string());
        }
    }
    NAMES.with(|n| *n.borrow_mut() = names);
}

fn pick<'a>(lang: &str, en: &'a str, ja: &'a str, zh: &'a str, ko: &'a str) -> &'a str {
    match lang {
        "ja" => ja,
        "zh-Hans" => zh,
        "ko" => ko,
        _ => en,
    }
}

/// Built-in localized display title for an optional non-playable game's background (`exe1` / `exe2`).
fn extra_background_name(game: &str, key: &str, lang: &str) -> Option<&'static str> {
    match game {
        "exe1" => Some(match key {
            "comp" => pick(lang, "Comp", "電脳", "电脑", "전뇌"),
            "school-comp" => pick(lang, "School Comp", "学校の電脳", "学校的电脑", "학교의 전뇌"),
            "waterworks-comp" => pick(lang, "Waterworks Comp", "水道局の電脳", "自来水局的电脑", "수도국의 전뇌"),
            "traffic-comp" => pick(lang, "Traffic Light Comp", "しんごうきの電脳", "信号灯的电脑", "신호등의 전뇌"),
            "power-plant-comp" => pick(lang, "Power Plant Comp", "はつでんしょの電脳", "发电厂的电脑", "발전소의 전뇌"),
            "www-comp" => pick(lang, "WWW Comp", "WWWの電脳", "WWW的电脑", "WWW의 전뇌"),
            "internet" => pick(lang, "Internet", "インターネット", "互联网", "인터넷"),
            "lans-pc" => pick(lang, "Lan's PC", "熱斗のPC", "热斗的PC", "넷토의 PC"),
            "dexs-pc" => pick(lang, "Dex's PC", "デカオのPC", "迪卡欧的PC", "데카오의 PC"),
            "mayls-pc" => pick(lang, "Mayl's PC", "メイルのPC", "美露的PC", "메이루의 PC"),
            "yais-pc" => pick(lang, "Yai's PC", "やいとのPC", "雅伊朵的PC", "야이토의 PC"),
            "sals-comp" => pick(lang, "Sal's Comp", "サロマの電脳", "萨洛马的电脑", "사로마의 전뇌"),
            "gov-comp" => pick(lang, "Govt Complex Comp", "官庁街の電脳", "政府大楼的电脑", "관청가의 전뇌"),
            "restaurant-comp" => pick(lang, "Restaurant Comp", "レストランの電脳", "餐厅的电脑", "레스토랑의 전뇌"),
            "antique-comp" => pick(lang, "Antique Comp", "ミユキの電脳", "美雪的电脑", "미유키의 전뇌"),
            "fish-stand-comp" => pick(lang, "Fish Stand Comp", "マサの電脳", "政叔的电脑", "마사의 전뇌"),
            "rocket-comp" => pick(lang, "Rocket Comp", "ロケットの電脳", "火箭的电脑", "로켓의 전뇌"),
            _ => return None,
        }),
        "exe2" => Some(match key {
            "comp" => pick(lang, "Comp", "電脳", "电脑", "전뇌"),
            "gas-comp" => pick(lang, "Gas Comp", "ガスゆわかしきの電脳", "燃气热水器的电脑", "가스온수기의 전뇌"),
            "bomb-comp" => pick(lang, "Bomb Comp", "ばくだんの電脳", "炸弹的电脑", "폭탄의 전뇌"),
            "mother-comp" => pick(lang, "Mother Comp", "マザーコンピューターの電脳", "母计算机的电脑", "마더 컴퓨터의 전뇌"),
            "mother-comp-dark" => pick(lang, "Mother Comp (Dark)", "マザーコンピューターの電脳(暗)", "母计算机的电脑（暗）", "마더 컴퓨터의 전뇌 (암)"),
            "castle-comp" => pick(lang, "Castle Comp", "おしろの電脳", "城堡的电脑", "성의 전뇌"),
            "castle-comp-2" => pick(lang, "Castle Comp 2", "おしろの電脳2", "城堡的电脑2", "성의 전뇌 2"),
            "castle-comp-3" => pick(lang, "Castle Comp 3", "おしろの電脳3", "城堡的电脑3", "성의 전뇌 3"),
            "airplane-comp" => pick(lang, "Airplane Comp", "ひこうきの電脳", "飞机的电脑", "비행기의 전뇌"),
            "lans-hp" => pick(lang, "Lan's HP", "熱斗のHP", "热斗的首页", "넷토의 HP"),
            "mayls-hp" => pick(lang, "Mayl's HP", "メイルのHP", "美露的首页", "메이루의 HP"),
            "yais-hp" => pick(lang, "Yai's HP", "やいとのHP", "雅伊朵的首页", "야이토의 HP"),
            "dexs-hp" => pick(lang, "Dex's HP", "デカオのHP", "迪卡欧的首页", "데카오의 HP"),
            "ribittas-comp" => pick(lang, "News Van Comp", "中継車の電脳", "转播车的电脑", "중계차의 전뇌"),
            "stereo-comp" => pick(lang, "Stereo Comp", "ステレオの電脳", "音响的电脑", "스테레오의 전뇌"),
            "vase-comp" => pick(lang, "Vase Comp", "ツボの電脳", "壶的电脑", "항아리의 전뇌"),
            "official-square" => pick(lang, "Official Square", "オフィシャルスクエア", "官方广场", "오피셜 스퀘어"),
            "official-comp" => pick(lang, "Official Comp", "オフィシャルの電脳", "官方的电脑", "오피셜의 전뇌"),
            "den-area" => pick(lang, "Den Area", "デンサンエリア", "电算区域", "덴산 에리어"),
            "koto-area" => pick(lang, "Koto Area", "コトブキエリア", "寿区域", "코토부키 에리어"),
            "koto-undernet" => pick(lang, "Koto Undernet", "コトブキウラ", "寿里区域", "코토부키 우라"),
            "yumland-area" => pick(lang, "Yumland Area", "アジーナエリア", "雅吉纳区域", "아지나 에리어"),
            "netopia-area" => pick(lang, "Netopia Area", "アメロッパエリア", "亚美罗帕区域", "아메로파 에리어"),
            "undernet" => pick(lang, "Undernet", "ウラインターネット", "里网络", "우라 인터넷"),
            "netopia-square" => pick(lang, "Netopia Square", "アメロッパスクエア", "亚美罗帕广场", "아메로파 스퀘어"),
            "den-square" => pick(lang, "Den Square", "デンサンスクエア", "电算广场", "덴산 스퀘어"),
            "apart-comp" => pick(lang, "Apartment Comp", "アパートの電脳", "公寓的电脑", "아파트의 전뇌"),
            "under-square" => pick(lang, "Under Square", "ウラスクエア", "里广场", "우라 스퀘어"),
            "gospel-server" => pick(lang, "Gospel Server", "ゴスペルサーバー", "戈斯巴鲁服务器", "고스펠 서버"),
            "gospel-server-dark" => pick(lang, "Gospel Server (Dark)", "ゴスペルサーバー(暗)", "戈斯巴鲁服务器（暗）", "고스펠 서버 (암)"),
            "bug-comp" => pick(lang, "Bug Space", "バグ空間", "漏洞空间", "버그 공간"),
            "www-area" => pick(lang, "WWW Area", "WWWエリア", "WWW区域", "WWW 에리어"),
            "gospel-core" => pick(lang, "Gospel Core", "ゴスペル最深部", "戈斯巴鲁最深部", "고스펠 최심부"),
            _ => return None,
        }),
        _ => None,
    }
}

/// Game `game`'s background `background` (its name in the game) as a list shows it, in the language
/// shown: the game's name, then the background's ("BN4 ACDC Area"); one its locales don't name, by its name in
/// the game's graphics.
pub fn name(game: &str, background: &str) -> String {
    let own = NAMES.with(|n| n.borrow().get(&(game.to_string(), background.to_string())).cloned()).unwrap_or_else(|| background.to_string());
    format!("{} {own}", Shell::game_name(game))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A background is named by its game and its own name in the language shown, from the content alone; the
    /// pool is every game's, the same whichever game's content it is read beside.
    #[test]
    fn a_background_is_named_by_its_game() {
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
        assert_eq!(name("exe1", "school-comp"), "EXE1 学校の電脳");
        assert_eq!(name("exe2", "gospel-core"), "EXE2 ゴスペル最深部");
        assert_eq!(name("exe4", "acdc-area"), "EXE4 秋原エリア");
        assert_eq!(name("exe5", "nebulagray"), "EXE5 ネビュラグレイ");
        assert_eq!(name("exe5", "no-such-area"), "EXE5 no-such-area");
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "zh-Hans");
        assert_eq!(name("exe1", "school-comp"), "EXE1 学校的电脑");
        assert_eq!(name("exe2", "gospel-core"), "EXE2 戈斯巴鲁最深部");
        assert_eq!(name("exe4", "acdc-area"), "EXE4 秋原区域");
        assert_eq!(name("exe6", "mr-weather-comp-storm"), "EXE6 气象君的电脑（台风）");
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
        assert_eq!(name("exe1", "school-comp"), "BN1 School Comp");
        assert_eq!(name("exe2", "gospel-core"), "BN2 Gospel Core");
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
        let pools = [saito_match::testing::exe4_content(), saito_match::testing::exe5_content(), saito_match::testing::exe6_content()]
            .map(|c| c.background_pool.keys().map(str::to_string).collect::<Vec<_>>());
        assert_eq!(pools[0], pools[1]);
        assert_eq!(pools[1], pools[2]);
        assert!(pools[0].contains(&"exe1:school-comp".to_string()));
        assert!(pools[0].contains(&"exe2:gospel-core".to_string()));
        let extra_count: usize = saito_extract::EXTRA_BG_GAMES.iter().map(|&(_, s)| s.len()).sum();
        assert_eq!(pools[0].len(), extra_count + 30 + 27 + 22 + 21, "every game's");
    }

    #[test]
    fn every_extra_background_has_localized_titles() {
        for lang in ["en", "ja", "zh-Hans", "ko"] {
            for &(game, specs) in saito_extract::EXTRA_BG_GAMES {
                for spec in specs {
                    assert!(
                        extra_background_name(game, spec.key, lang).is_some(),
                        "{game}/{} missing localized title for {lang}",
                        spec.key
                    );
                }
            }
        }
    }
}
