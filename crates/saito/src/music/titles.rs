//! Official Legacy Collection / ROCKMAN.EXE SOUND BOX localized BGM titles for
//! the full EXE4, EXE5, EXE5DS, and EXE6 soundtracks (`S-135`).
//!
//! Follows the *Mega Man Battle Network Legacy Collection* (ROCKMAN EXE 合集)
//! Music Player convention: tracks whose official Sound Box / Legacy Collection
//! title is in English (such as `Theme of ROCKMAN EXE4`, `Invisible Wing`,
//! `Seek a Cord`, `Digital Strider`, `Battle Field`, `Two of Braves`, etc.)
//! remain verbatim in English across `ja`, `zh-Hans`, and `ko`, while
//! Japanese-titled tracks use the official localized names.

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
        // 0x00: Title
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 4",
            "Theme of ROCKMAN EXE4",
            "Theme of ROCKMAN EXE4",
            "Theme of ROCKMAN EXE4",
        ),
        // 0x01..0x09: Overworld & Tournament Intro
        "silent-darkness" => pick(lang, "Silent Darkness", "静かなる闇", "寂静之暗", "고요한 어둠"),
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "故乡", "홈타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "elec-town" => pick(lang, "Electric Town", "電気街", "电气街", "전자상가"),
        "den-dome" => pick(lang, "Densan Stadium", "デンサンスタジアム", "电算体育馆", "덴산 스타디움"),
        "castillo" => pick(lang, "Cielo Castillo", "シェロ・カスティロ", "天空之城", "시엘로 카스티요"),
        "foreign-town" => pick(lang, "Determined Journey", "決意の旅", "决意之旅", "결의의 여정"),
        "naxa" => "ANSA",
        "player-admission" => pick(lang, "Player Admission", "選手入場", "选手入场", "선수 입장"),
        // 0x0A = 優勝おめでとう! (Great Victory!), 0x0B = 事件発生! (Incident Occurrence!)
        "event-occurance" => pick(lang, "Great Victory!", "優勝おめでとう!", "恭喜夺冠!", "우승 축하해!"),
        "crises" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "sad" => pick(lang, "Sad Rain", "悲しい雨", "悲伤之雨", "슬픈 비"),
        "heroism" => pick(lang, "Under Justice", "正義のもとに", "在正义之下", "정의 아래"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输!", "트랜스미션!"),
        // 0x0F..0x13: Cyberworld (untranslated English in Legacy Collection)
        "elec-tower-comp" => "Invisible Wing",
        "toyrobo-comp" => "Story of Wonder",
        "asteroid-comp" => "Save Our Planet",
        "network" => "Global Network",
        "undernet" => pick(lang, "Heel's Parade", "Heel's Paradise", "Heel's Paradise", "Heel's Paradise"),
        // 0x14..0x17: Battles
        "virus-battle" => pick(lang, "Cyber Battle", "サイバーバトル", "赛博战斗", "사이버 배틀"),
        "boss-battle" => pick(lang, "Fighting Oneself", "自分との戦い", "与自己战斗", "자신과의 싸움"),
        "final-battle" => pick(lang, "VS. Duo", "VS.デューオ", "VS.迪欧", "VS. 듀오"),
        "tournament-battle" => pick(lang, "Battle Pressure", "バトルプレッシャー", "战斗压力", "배틀 프레셔"),
        // 0x18 = Enemy Deleted! (looping battle result), 0x1E = Enemy Deleted! (short jingle)
        "winner-special" => "Enemy Deleted!",
        "loser" => "Loser",
        "game-over" => "Game Over",
        "boss-prelude" => pick(lang, "Versus", "バーサス", "对决", "버서스"),
        "credits" => "Your Answer",
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个性化装置", "내비 커스터마이저"),
        "winner" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        "vampire-mansion" => pick(lang, "Dracula's Mansion", "ドラッキーの館", "德拉基之馆", "드라키의 저택"),
        "vampire" => "Vampire",
        "virus-battle-alt" => pick(lang, "Cyber Battle (Alt)", "サイバーバトル (別版)", "赛博战斗 (备用版)", "사이버 배틀 (별도 버전)"),
        _ => return None,
    })
}

fn exe5_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        // 0x01: Title
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 5",
            "Theme of ROCKMAN EXE5",
            "Theme of ROCKMAN EXE5",
            "Theme of ROCKMAN EXE5",
        ),
        // 0x02..0x09: Overworld
        "nebula-theme" => pick(lang, "Theme of Nebula", "ネビュラのテーマ", "星云的主题", "네뷸라의 테마"),
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "故乡", "홈타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "scilab" => pick(lang, "Scientific Forefront", "科学最前線", "科学最前线", "과학 최전선"),
        "oran-island" => pick(lang, "Oran Island", "オラン島", "欧兰岛", "오란 섬"),
        "queen-bohemia" => pick(lang, "Luxury Liner", "豪華客船", "豪华客船", "호화 여객선"),
        "end-city" => pick(lang, "End City", "エンドシティ", "安多市", "엔드 시티"),
        "dark-chip-factory" => pick(lang, "Dark Chip Factory", "ダークチップファクトリー", "黑暗芯片工厂", "다크 칩 팩토리"),
        // 0x0B = 廃坑 (Abandoned Mine, SEQ 11 BGM_EVENT2), 0x0C = 事件発生! (SEQ 12 BGM_HAPPENING)
        "event-occurance" => pick(lang, "Abandoned Mine", "廃坑", "废坑", "폐광"),
        "crises" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "sad" => pick(lang, "A Heart's Shadow", "心の影", "心之影", "마음의 그림자"),
        "you-are-not-alone" => pick(lang, "You're Not Alone!", "一人じゃないんだ!", "不是孤身一人!", "혼자가 아니야!"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输!", "트랜스미션!"),
        // 0x10..0x14: Cyberworld (untranslated English in Legacy Collection)
        "main-comp" => "Seek a Cord",
        "drill-comp" => "Drill Crush!",
        "factory-comp" => "A Total War",
        "network" => "Network Space",
        "undernet" => "Depth",
        // 0x15..0x18: Battles & Liberation
        "virus-battle" => pick(lang, "Battle Start!", "バトルスタート!", "战斗开始!", "배틀 스타트!"),
        "boss-battle" => pick(lang, "Powerful Enemy", "パワフルエネミー", "强敌", "파워풀 에너미"),
        "final-battle" => pick(lang, "VS. Nebula Gray", "VS.ネビュラグレイ", "VS.星云灰暗", "VS. 네뷸라 그레이"),
        "liberate-boss-fanfare" => pick(lang, "Liberation Complete!", "リベレート完了!", "解放完成!", "리버레이트 완료!"),
        // 0x19..0x1F: Results, Boss Prelude (0x1C = Black Power), Credits, NaviCust
        "winner-0" => "Enemy Deleted!",
        "loser" => "Loser",
        "game-over" => "Game Over",
        "boss-prelude" => "Black Power",
        "credits" => "Chain of Wish",
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个性化装置", "내비 커스터마이저"),
        "winner-1" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        // 0x20..0x24: Ship/Castle Comps, Liberation Mission, Trial, 1 Turn Liberate
        "ship-comp" => "Quest in the Water",
        "castle-comp" => "Castle of NINJA!",
        "liberate" => pick(lang, "Liberate Mission", "リベレートミッション", "解放任务", "리버레이트 미션"),
        "trial" => "Trial",
        "1turn-delete" => pick(lang, "1 Turn Liberate!", "1ターンリベレート!", "1回合解放!", "1턴 리버레이트!"),
        _ => return None,
    })
}

fn ds_title(base: &str, lang: &str) -> Option<&'static str> {
    match base {
        "exe1-battle" => Some(pick(lang, "Net Battle (BN1)", "ネットバトル (EXE1)", "网络对战 (EXE1)", "넷 배틀 (EXE1)")),
        "exe2-battle" => Some(pick(lang, "Virus Busting (BN2)", "ウイルスバスティング (EXE2)", "病毒清除 (EXE2)", "바이러스 버스팅 (EXE2)")),
        "exe3-battle" => Some(pick(lang, "Shooting Enemy (BN3)", "シューティングエネミー (EXE3)", "射击敌人 (EXE3)", "슈팅 에너미 (EXE3)")),
        "exe4-battle" => Some(pick(lang, "Cyber Battle (BN4)", "サイバーバトル (EXE4)", "赛博战斗 (EXE4)", "사이버 배틀 (EXE4)")),
        "exe45-battle" => Some(pick(lang, "Battle Field (BN4.5)", "バトルフィールド (EXE4.5)", "Battle Field (EXE4.5)", "Battle Field (EXE4.5)")),
        other => exe5_title(other, lang),
    }
}

fn exe6_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        // 0x01: Title
        "title-screen" => pick(
            lang,
            "Theme of MEGA MAN BATTLE NETWORK 6",
            "Theme of ROCKMAN EXE6",
            "Theme of ROCKMAN EXE6",
            "Theme of ROCKMAN EXE6",
        ),
        // 0x02..0x08: Overworld
        "www-theme" => pick(lang, "Organization", "組織", "组织", "조직"),
        "cyber-city" => pick(lang, "Central Town", "セントラルタウン", "中央镇", "센트럴 타운"),
        "indoors" => pick(lang, "In the Room", "室内", "室内", "실내"),
        "school" => pick(lang, "Cyber Academy", "才葉学園", "才叶学园", "사이바 학원"),
        "seaside-town" => pick(lang, "Seaside Town", "シーサイドタウン", "海边镇", "시사이드 타운"),
        "sky-town" => pick(lang, "Sky Town", "スカイタウン", "天空镇", "스카이 타운"),
        "green-town" => pick(lang, "Green Town", "グリーンタウン", "绿镇", "그린 타운"),
        // 0x09..0x0A: Graveyard & Mr. Weather Comp
        "graveyard-area" => "Graveyard",
        "mr-weather-comp" => "Break the Storm",
        // 0x0B..0x0F: Story Events (0x0C = 疑惑 Doubt, 0x0D = 約束 Promise, 0x0E = ヒーロー! Hero!)
        "event-occurance" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生!", "사건 발생!"),
        "crises" => pick(lang, "Doubt", "疑惑", "疑惑", "의혹"),
        "sad" => pick(lang, "Promise", "約束", "约定", "약속"),
        "happy" => pick(lang, "Hero!", "ヒーロー!", "英雄!", "히어로!"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输!", "트랜스미션!"),
        // 0x10..0x16: Comps, Internet, Undernet & Battles (untranslated English in Legacy Collection)
        "robo-control-comp" => "Blast Speed",
        "aquarium-comp" => "Aquarium Question",
        "judge-tree-comp" => "Tree of Judgement",
        "network" => "Digital Strider",
        "undernet" => "Danger Zone!",
        "virus-battle" => "Battle Field",
        "boss-battle" => "Surge of Power!",
        // 0x17..0x1F: Final Boss, Secret Base (0x18), Results, Distortion! (0x1C), Future (0x1D), NaviCust
        "final-battle" => pick(lang, "Decisive Battle, Cyber Beasts!", "決戦、電脳獣!", "决战，电脑兽!", "결전, 전뇌수!"),
        "pavilion" => pick(lang, "Secret Base", "秘密の本拠地", "秘密据点", "비밀 본거지"),
        "winner-0" => "Enemy Deleted!",
        "loser" => "Loser",
        "game-over" => "Game Over",
        "boss-prelude" => "Distortion!",
        "credits" => pick(lang, "Future", "未来", "未来", "미래"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员个性化装置", "내비 커스터마이저"),
        "winner-1" => pick(
            lang,
            "Enemy Deleted! (Short)",
            "Enemy Deleted! (ショート)",
            "Enemy Deleted! (短版)",
            "Enemy Deleted! (단축)",
        ),
        // 0x20..0x25: Two of Braves, Evil Spirit, Count, Shark Panic!, Hometown, Parade!
        "pavilion-comp" => "Two of Braves",
        "cyberbeasts" => "Evil Spirit",
        "crossover-battle" => pick(
            lang,
            "Count -crossover ver.-",
            "伯爵 -crossover ver.-",
            "伯爵 -crossover ver.-",
            "백작 -crossover ver.-",
        ),
        "shark-chase" => "Shark Panic!",
        "acdc-town" => pick(lang, "Hometown", "ホームタウン", "故乡", "홈타운"),
        "expo" => pick(lang, "Parade!", "パレード!", "游行!", "퍼레이드!"),
        _ => return None,
    })
}
