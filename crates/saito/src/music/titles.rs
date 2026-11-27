//! Official *Mega Man Battle Network Legacy Collection* (ROCKMAN EXE 合集)
//! Music Player localized BGM titles for EXE4, EXE5, EXE5DS, and EXE6 (`S-135`).
//!
//! Simplified Chinese (`zh-Hans`) titles and track ordering match the official
//! Legacy Collection Vol. 2 Music Player screenshots verbatim (including
//! untranslated English titles such as `Theme of ROCKMAN EXE 4`, `Hometown`,
//! `Invisible Wing`, `Heel's Parade`, `Castle of Ninja!`, `Battle Field`, etc.).

/// Localized display title for a single track (without the game prefix), or `None` for custom/unknown keys.
pub fn track_title(game: &str, song: &str, lang: &str) -> Option<&'static str> {
    if let Some(ds_base) = song.strip_prefix("ds-") {
        return ds_title(ds_base, lang);
    }
    match game {
        "exe4" => exe4_title(song, lang),
        "exe5" => exe5_title(song, lang),
        "exe6" => exe6_title(song, lang),
        _ => None,
    }
}

fn pick<'a>(lang: &str, en: &'a str, ja: &'a str, zh: &'a str, ko: &'a str) -> &'a str {
    match lang {
        "ja" => ja,
        "zh-Hans" => zh,
        "ko" => ko,
        _ => en,
    }
}

fn exe4_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        // Legacy Collection Vol. 2 — Rockman EXE 4 track order:
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 4",
            "Theme of ROCKMAN EXE 4",
            "Theme of ROCKMAN EXE 4",
            "Theme of ROCKMAN EXE 4",
        ),
        "naxa" => "ANSA",
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "Hometown", "홈타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "elec-town" => pick(lang, "Electric Town", "電気街", "电器街", "전자상가"),
        "crises" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "程序传输!", "트랜스미션!"),
        "elec-tower-comp" => "Invisible Wing",
        "virus-battle" => pick(lang, "Cyber Battle", "サイバーバトル", "赛博对战", "사이버 배틀"),
        "winner-special" => "Enemy Deleted!",
        "silent-darkness" => pick(lang, "Silent Darkness", "静かなる闇", "静默之暗", "고요한 어둠"),
        "den-dome" => pick(lang, "Densan Stadium", "デンサンスタジアム", "电算体育馆", "덴산 스타디움"),
        "player-admission" => pick(lang, "Player Admission", "選手入場", "选手入场", "선수 입장"),
        "tournament-battle" => pick(lang, "Battle Pressure", "バトルプレッシャー", "战斗的压迫感", "배틀 프레셔"),
        "event-occurance" => pick(lang, "Great Victory!", "優勝おめでとう!", "大获全胜!", "우승 축하해!"),
        "network" => "Global Network",
        "castillo" => pick(lang, "Cielo Castillo", "シェロ・カスティロ", "天空之城", "시엘로 카스티요"),
        "vampire-mansion" => pick(lang, "Dracula's Mansion", "ドラッキーの館", "多朗基馆", "드라키의 저택"),
        "toyrobo-comp" => "Story of Wonder",
        "vampire" => "Vampire",
        "boss-prelude" => pick(lang, "Versus", "バーサス", "对抗", "버서스"),
        "boss-battle" => pick(lang, "Fighting Oneself", "自分との戦い", "与自身的战斗", "자신과의 싸움"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个人化装置", "내비 커스터마이저"),
        "foreign-town" => pick(lang, "Determined Journey", "決意の旅", "坚定的旅程", "결의의 여정"),
        "undernet" => "Heel's Parade",
        "sad" => pick(lang, "Sad Rain", "悲しい雨", "悲伤的雨", "슬픈 비"),
        "heroism" => pick(lang, "Under Justice", "正義のもとに", "以正义之名", "정의 아래"),
        "asteroid-comp" => "Save Our Planet",
        "game-over" => "Game Over",
        "loser" => "Loser",
        "final-battle" => pick(lang, "VS. Duo", "VS.デューオ", "对决迪欧", "VS. 듀오"),
        "credits" => "Your Answer",
        // ROM-only extras:
        "winner" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        "virus-battle-alt" => pick(lang, "Cyber Battle (Alt)", "サイバーバトル (別版)", "赛博对战 (备用版)", "사이버 배틀 (별도 버전)"),
        _ => return None,
    })
}

fn exe5_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        // Legacy Collection Vol. 2 — Rockman EXE 5 track order:
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 5",
            "Theme of ROCKMAN EXE 5",
            "Theme of ROCKMAN EXE 5",
            "Theme of ROCKMAN EXE 5",
        ),
        "nebula-theme" => pick(lang, "Theme of Nebula", "ネビュラのテーマ", "星云", "네뷸라의 테마"),
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "Hometown", "홈타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "scilab" => pick(lang, "Scientific Forefront", "科学最前線", "科学最前线", "과학 최전선"),
        "crises" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "程序传输!", "트랜스미션!"),
        "main-comp" => "Seek a Cord",
        "virus-battle" => pick(lang, "Battle Start!", "バトルスタート!", "来对战吧!", "배틀 스타트!"),
        "winner-0" => "Enemy Deleted!",
        "oran-island" => pick(lang, "Oran Island", "オラン島", "欧兰岛", "오란 섬"),
        "event-occurance" => pick(lang, "Abandoned Mine", "廃坑", "废矿坑", "폐광"),
        "drill-comp" => "Drill Crush!",
        "network" => "Network Space",
        "boss-prelude" => "Black Power",
        "liberate" => pick(lang, "Liberate Mission", "リベレートミッション", "解放任务", "리버레이트 미션"),
        "boss-battle" => pick(lang, "Powerful Enemy", "パワフルエネミー", "遭遇强敌", "파워풀 에너미"),
        "liberate-boss-fanfare" => pick(lang, "Liberation Complete!", "リベレート完了!", "解放终了!", "리버레이트 완료!"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个人化装置", "내비 커스터마이저"),
        "queen-bohemia" => pick(lang, "Luxury Liner", "豪華客船", "豪华客船", "호화 여객선"),
        "ship-comp" => "Quest in the Water",
        "end-city" => pick(lang, "End City", "エンドシティ", "安多市", "엔드 시티"),
        "castle-comp" => "Castle of Ninja!",
        "undernet" => "Depth",
        "sad" => pick(lang, "A Heart's Shadow", "心の影", "心之影", "마음의 그림자"),
        "you-are-not-alone" => pick(lang, "You're Not Alone!", "一人じゃないんだ!", "你不孤单!", "혼자가 아니야!"),
        "dark-chip-factory" => pick(lang, "Dark Chip Factory", "ダークチップファクトリー", "黑暗芯片工厂", "다크 칩 팩토리"),
        "factory-comp" => "A Total War",
        "trial" => "Trial",
        "1turn-delete" => pick(lang, "1 Turn Liberate!", "1ターンリベレート!", "一回合的解放!", "1턴 리버레이트!"),
        "game-over" => "Game Over",
        "loser" => "Loser",
        "final-battle" => pick(lang, "VS. Nebula Gray", "VS.ネビュラグレイ", "对决星云灰暗", "VS. 네뷸라 그레이"),
        "credits" => "Chain of Wish",
        // ROM-only short fanfare variant:
        "winner-1" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        _ => return None,
    })
}

fn ds_title(base: &str, lang: &str) -> Option<&'static str> {
    match base {
        "exe1-battle" => Some(pick(lang, "Net Battle (BN1)", "ネットバトル (EXE1)", "网络对战 (EXE1)", "넷 배틀 (EXE1)")),
        "exe2-battle" => Some(pick(lang, "Virus Busting (BN2)", "ウイルスバスティング (EXE2)", "病毒清除 (EXE2)", "바이러스 버스팅 (EXE2)")),
        "exe3-battle" => Some(pick(lang, "Shooting Enemy (BN3)", "シューティングエネミー (EXE3)", "射击敌人 (EXE3)", "슈팅 에너미 (EXE3)")),
        "exe4-battle" => Some(pick(lang, "Cyber Battle (BN4)", "サイバーバトル (EXE4)", "赛博对战 (EXE4)", "사이버 배틀 (EXE4)")),
        "exe45-battle" => Some(pick(lang, "Battle Field (BN4.5)", "バトルフィールド (EXE4.5)", "Battle Field (EXE4.5)", "Battle Field (EXE4.5)")),
        other => exe5_title(other, lang),
    }
}

fn exe6_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        // Legacy Collection Vol. 2 — Rockman EXE 6 track order:
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 6",
            "Theme of ROCKMAN EXE 6",
            "Theme of ROCKMAN EXE 6",
            "Theme of ROCKMAN EXE 6",
        ),
        "www-theme" => pick(lang, "Organization", "組織", "组织", "조직"),
        "cyber-city" => pick(lang, "Central Town", "セントラルタウン", "中央镇", "센트럴 타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "school" => pick(lang, "Cyber Academy", "才葉学園", "才叶学园", "사이바 학원"),
        "event-occurance" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "程序传输!", "트랜스미션!"),
        "robo-control-comp" => "Blast Speed",
        "seaside-town" => pick(lang, "Seaside Town", "シーサイドタウン", "海边镇", "시사이드 타운"),
        "aquarium-comp" => "Aquarium Question",
        "shark-chase" => "Shark Panic!",
        "virus-battle" => "Battle Field",
        "winner-0" => "Enemy Deleted!",
        "green-town" => pick(lang, "Green Town", "グリーンタウン", "绿镇", "그린 타운"),
        "crises" => pick(lang, "Doubt", "疑惑", "疑惑", "의혹"),
        "judge-tree-comp" => "Tree of Judgement",
        "boss-prelude" => "Distortion!",
        "boss-battle" => "Surge of Power!",
        "network" => "Digital Strider",
        "sky-town" => pick(lang, "Sky Town", "スカイタウン", "天空镇", "스카이 타운"),
        "sad" => pick(lang, "Promise", "約束", "约定", "약속"),
        "mr-weather-comp" => "Break the Storm",
        "cyberbeasts" => "Evil Spirit",
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "Hometown", "홈타운"),
        "happy" => pick(lang, "Hero!", "ヒーロー!", "英雄!", "히어로!"),
        "undernet" => "Danger Zone!",
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个人化装置", "내비 커스터마이저"),
        "expo" => pick(lang, "Parade!", "パレード!", "游行!", "퍼레이드!"),
        "graveyard-area" => "Graveyard",
        "game-over" => "Game Over",
        "loser" => "Loser",
        "pavilion" => pick(lang, "Secret Base", "秘密の本拠地", "秘密总部", "비밀 본거지"),
        "pavilion-comp" => "Two of Braves",
        "final-battle" => pick(lang, "Decisive Battle, Cyber Beasts!", "決戦、電脳獣!", "决战、电脑兽!", "결전, 전뇌수!"),
        "credits" => pick(lang, "Future ~ Epilogue", "未来 ~ Epilogue", "未来 ~ Epilogue", "미래 ~ Epilogue"),
        // ROM-only extras:
        "crossover-battle" => pick(
            lang,
            "Count -crossover ver.-",
            "伯爵 -crossover ver.-",
            "伯爵 -crossover ver.-",
            "백작 -crossover ver.-",
        ),
        "winner-1" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        _ => return None,
    })
}
