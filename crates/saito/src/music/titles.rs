//! Localized BGM titles for the full EXE4, EXE5, EXE5DS, and EXE6 soundtracks (`S-135`).
//!
//! Covers English (`en`), Japanese (`ja`), Simplified Chinese (`zh-Hans`), and Korean (`ko`).

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
        "title-screen" => pick(lang, "Theme of Battle Network 4", "Theme of ROCKMAN EXE4", "EXE4 主题曲", "록맨 에그제 4 테마"),
        "silent-darkness" => pick(lang, "Silent Darkness", "サイレントダークネス", "静谧黑暗", "침묵의 어둠"),
        "acdc-town" => pick(lang, "Hometown (ACDC Town)", "ホームタウン", "故乡 (秋原镇)", "홈타운 (아키하라)"),
        "indoors" => pick(lang, "In the Room", "イン・ザ・ルーム", "室内", "실내"),
        "elec-town" => pick(lang, "Electric Town", "エレキタウン", "电气镇", "일렉 타운"),
        "den-dome" => pick(lang, "Densan Stadium", "デンサンドーム", "电算巨蛋", "덴산 돔"),
        "castillo" => pick(lang, "Cielo Castillo", "シエロ・カスティロ", "卡斯提罗游乐园", "카스티요"),
        "foreign-town" => pick(lang, "Determined Journey", "ディターミンド・ジャーニー", "坚定的旅途", "결의의 여정"),
        "naxa" => pick(lang, "NAXA", "ANSA", "NAXA 宇宙中心", "NAXA"),
        "player-admission" => pick(lang, "Player Admission", "選手入場", "选手入场", "선수 입장"),
        "event-occurance" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生！", "사건 발생!"),
        "crises" => pick(lang, "Sudden Crisis", "危機迫る", "危机迫近", "갑작스러운 위기"),
        "sad" => pick(lang, "Sad Rain", "悲しみの雨", "悲伤之雨", "슬픈 비"),
        "heroism" => pick(lang, "Under Justice", "アンダー・ジャスティス", "正义之下", "정의 아래"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输！", "트랜스미션!"),
        "elec-tower-comp" => pick(lang, "Invisible Wing", "インビジブル・ウィング", "隐形之翼 (电塔电脑)", "인비저블 윙"),
        "toyrobo-comp" => pick(lang, "Story of Wonder", "ストーリー・オブ・ワンダー", "奇幻物语 (玩具机器人电脑)", "스토리 오브 원더"),
        "asteroid-comp" => pick(lang, "Save Our Planet", "セイブ・アワー・プラネット", "拯救星球 (小行星电脑)", "세이브 아워 플래닛"),
        "network" => pick(lang, "Global Network", "グローバルネットワーク", "全球网络", "글로벌 네트워크"),
        "undernet" => pick(lang, "Heel's Paradise", "ヒールズ・パラダイス", "暗网乐园", "힐즈 파라다이스"),
        "virus-battle" => pick(lang, "Virus Battle (Cyber Battle)", "ウイルスバトル", "病毒战", "바이러스 배틀"),
        "boss-battle" => pick(lang, "Boss Battle (Fighting Oneself)", "ボスバトル", "头目战", "보스 배틀"),
        "final-battle" => pick(lang, "VS Duo", "VS デューオ", "VS 迪欧", "VS 듀오"),
        "tournament-battle" => pick(lang, "Tournament Battle (Battle Pressure)", "トーナメントバトル", "锦标赛战斗", "토너먼트 배틀"),
        "winner-special" => pick(lang, "Great Victory!", "グレートビクトリー!", "大胜利！", "위대한 승리!"),
        "loser" => pick(lang, "Loser", "ルーザー", "败北", "패배"),
        "game-over" => pick(lang, "Game Over", "ゲームオーバー", "游戏结束", "게임 오버"),
        "boss-prelude" => pick(lang, "Versus", "バーサス", "对峙前奏", "버서스"),
        "credits" => pick(lang, "Your Answer", "ユア・アンサー", "你的答案 (通关职员表)", "유어 앤서"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员改装系统", "내비 커스터마이저"),
        "winner" => pick(lang, "Enemy Deleted!", "エネミーデリート!", "敌方删除！", "에너미 딜리트!"),
        "vampire-mansion" => pick(lang, "Dracula's Mansion", "ドラキュラの館", "吸血鬼古堡", "드라큘라의 저택"),
        "vampire" => pick(lang, "Vampire", "ヴァンパイア", "吸血鬼 (古堡电脑)", "뱀파이어"),
        "virus-battle-alt" => pick(lang, "Cyber Battle (Alt)", "サイバーバトル (別版)", "病毒战 (备用版)", "사이버 배틀 (별도 버전)"),
        _ => return None,
    })
}

fn exe5_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        "title-screen" => pick(lang, "Theme of Battle Network 5", "Theme of ROCKMAN EXE5", "EXE5 主题曲", "록맨 에그제 5 테마"),
        "nebula-theme" => pick(lang, "Theme of Nebula", "ネビュラのテーマ", "星云主题曲", "네뷸라의 테마"),
        "acdc-town" => pick(lang, "Hometown (ACDC Town)", "ホームタウン", "故乡 (秋原镇)", "홈타운 (아키하라)"),
        "indoors" => pick(lang, "In the Room", "イン・ザ・ルーム", "室内", "실내"),
        "scilab" => pick(lang, "Scientific Forefront (SciLab)", "科学の最前線", "科学省", "과학의 최전선"),
        "oran-island" => pick(lang, "Oran Island", "オラン島", "奥兰岛", "오란 섬"),
        "queen-bohemia" => pick(lang, "Luxury Liner (Queen Bohemia)", "豪華客船", "豪华客轮", "호화 여객선"),
        "end-city" => pick(lang, "End City", "エンドシティ", "终结市", "엔드 시티"),
        "dark-chip-factory" => pick(lang, "Dark Chip Factory", "ダークチップ工場", "暗黑芯片工厂", "다크 칩 공장"),
        "event-occurance" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生！", "사건 발생!"),
        "crises" => pick(lang, "Black Power", "ブラックパワー", "暗黑之力", "블랙 파워"),
        "sad" => pick(lang, "A Heart's Shadow", "心の影", "心之影", "마음의 그림자"),
        "you-are-not-alone" => pick(lang, "You're Not Alone", "キミはひとりじゃない", "你并不孤单", "너는 혼자가 아니야"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输！", "트랜스미션!"),
        "main-comp" => pick(lang, "Seek a Cord (Main Comp)", "シーク・ア・コード", "寻找线索 (主系统电脑)", "시크 어 코드"),
        "drill-comp" => pick(lang, "Drill Crush! (Mine Comp)", "ドリルクラッシュ!", "钻头粉碎 (采矿场电脑)", "드릴 크래시!"),
        "factory-comp" => pick(lang, "A Total War (Factory Comp)", "総力戦", "总力战 (最终迷宫)", "총력전"),
        "network" => pick(lang, "Network Space", "ネットワーク空間", "网络空间", "네트워크 공간"),
        "undernet" => pick(lang, "Depth (Undernet)", "深淵 (ウラインターネット)", "深渊 (暗网)", "심연 (언더넷)"),
        "virus-battle" => pick(lang, "Virus Battle (Battle Start!)", "ウイルスバトル", "病毒战", "바이러스 배틀"),
        "boss-battle" => pick(lang, "Boss Battle (Powerful Enemy)", "ボスバトル", "头目战", "보스 배틀"),
        "final-battle" => pick(lang, "VS Nebula Gray", "VS ネビュラグレイ", "VS 星云灰暗", "VS 네뷸라 그레이"),
        "liberate-boss-fanfare" => pick(lang, "Liberation Complete!", "リベレート完了!", "解放完成！", "리버레이트 완료!"),
        "winner-0" => pick(lang, "Victory! (Result)", "バトルリザルト", "战斗胜利", "배틀 리절트"),
        "loser" => pick(lang, "Loser", "ルーザー", "败北", "패배"),
        "game-over" => pick(lang, "Game Over", "ゲームオーバー", "游戏结束", "게임 오버"),
        "boss-prelude" => pick(lang, "Boss Prelude", "ボス戦前", "头目战前奏", "보스전 전주"),
        "credits" => pick(lang, "Chain of Wishes", "願いの鎖", "愿望之链 (通关职员表)", "소원의 사슬"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员改装系统", "내비 커스터마이저"),
        "winner-1" => pick(lang, "Enemy Deleted!", "エネミーデリート!", "敌方删除！", "에너미 딜리트!"),
        "ship-comp" => pick(lang, "Quest in the Water (Ship Comp)", "クエスト・イン・ザ・ウォーター", "水中探索 (客轮电脑)", "퀘스트 인 더 워터"),
        "castle-comp" => pick(lang, "Castle of NINJA! (Gargoyle Comp)", "キャッスル・オブ・NINJA!", "忍者之城 (城堡电脑)", "캐슬 오브 닌자!"),
        "liberate" => pick(lang, "Liberation Mission", "リベレートミッション", "解放任务", "리버레이트 미션"),
        "trial" => pick(lang, "Trial (100-Foe Challenge)", "百人斬り", "百人斩试炼", "백인 베기"),
        "1turn-delete" => pick(lang, "1-Turn Liberate!", "1ターンリベレート!", "1回合解放！", "1턴 리버레이트!"),
        _ => return None,
    })
}

fn ds_title(base: &str, lang: &str) -> Option<&'static str> {
    match base {
        "exe1-battle" => Some(pick(lang, "BN1 Battle Remix", "EXE1 バトルアレンジ", "EXE1 战斗重编", "EXE1 배틀 어레인지")),
        "exe2-battle" => Some(pick(lang, "BN2 Battle Remix", "EXE2 バトルアレンジ", "EXE2 战斗重编", "EXE2 배틀 어레인지")),
        "exe3-battle" => Some(pick(lang, "BN3 Battle Remix", "EXE3 バトルアレンジ", "EXE3 战斗重编", "EXE3 배틀 어레인지")),
        "exe4-battle" => Some(pick(lang, "BN4 Battle Remix", "EXE4 バトルアレンジ", "EXE4 战斗重编", "EXE4 배틀 어레인지")),
        "exe45-battle" => Some(pick(lang, "BN4.5 Battle Remix", "EXE4.5 バトルアレンジ", "EXE4.5 战斗重编", "EXE4.5 배틀 어레인지")),
        other => exe5_title(other, lang),
    }
}

fn exe6_title(song: &str, lang: &str) -> Option<&'static str> {
    Some(match song {
        "title-screen" => pick(lang, "Theme of Battle Network 6", "Theme of ROCKMAN EXE6", "EXE6 主题曲", "록맨 에그제 6 테마"),
        "www-theme" => pick(lang, "Theme of WWW", "WWWのテーマ", "WWW 主题曲", "WWW의 테마"),
        "cyber-city" => pick(lang, "Central Town", "セントラルタウン", "中央镇", "센트럴 타운"),
        "indoors" => pick(lang, "In the Room", "イン・ザ・ルーム", "室内", "실내"),
        "school" => pick(lang, "Cyber Academy", "才葉学園", "才叶学园", "사이바 학원"),
        "seaside-town" => pick(lang, "Seaside Town", "シーサイドタウン", "海滨镇", "시사이드 타운"),
        "sky-town" => pick(lang, "Sky Town", "スカイタウン", "天空镇", "스카이 타운"),
        "green-town" => pick(lang, "Green Town", "グリーンタウン", "绿荫镇", "그린 타운"),
        "graveyard-area" => pick(lang, "Graveyard Area", "グレイブヤード", "墓地区域", "그레이브야드"),
        "mr-weather-comp" => pick(lang, "Weather Control (Mr. Weather Comp)", "ウェザーコントロール", "气象控制 (天气君电脑)", "웨더 컨트롤"),
        "event-occurance" => pick(lang, "Incident Occurrence!", "事件発生!", "事件发生！", "사건 발생!"),
        "crises" => pick(lang, "Crisis!", "危機迫る", "危机迫近！", "위기!"),
        "sad" => pick(lang, "Sadness", "悲しみ", "悲伤", "슬픔"),
        "happy" => pick(lang, "Joy", "喜び", "喜悦", "기쁨"),
        "transmission" => pick(lang, "Transmission!", "トランスミッション!", "传输！", "트랜스미션!"),
        "robo-control-comp" => pick(lang, "Blast Speed (RoboControl Comp)", "ブラストスピード", "爆炎疾速 (机器控制电脑)", "블래스트 스피드"),
        "aquarium-comp" => pick(lang, "Shark Panic! (Aquarium Comp)", "シャークパニック!", "鲨鱼危机 (水族馆电脑)", "샤크 패닉!"),
        "judge-tree-comp" => pick(lang, "Judge Tree", "ジャッジツリー", "审判之树 (法院电脑)", "저지 트리"),
        "network" => pick(lang, "Digital Strider (Internet)", "デジタルストライダー", "数码漫步者 (网络)", "디지털 스트라이더"),
        "undernet" => pick(lang, "Undernet", "ウラインターネット", "暗网", "언더넷"),
        "virus-battle" => pick(lang, "Virus Battle (Battle Field)", "ウイルスバトル", "病毒战", "바이러스 배틀"),
        "boss-battle" => pick(lang, "Boss Battle (Surge of Power!)", "ボスバトル", "头目战", "보스 배틀"),
        "final-battle" => pick(lang, "VS Cybeast", "VS 電脳獣", "VS 电脑兽", "VS 전뇌수"),
        "pavilion" => pick(lang, "Expo Site", "万博会場", "博览会会场", "박람회장"),
        "winner-0" => pick(lang, "Victory! (Result)", "バトルリザルト", "战斗胜利", "배틀 리절트"),
        "loser" => pick(lang, "Loser", "ルーザー", "败北", "패배"),
        "game-over" => pick(lang, "Game Over", "ゲームオーバー", "游戏结束", "게임 오버"),
        "boss-prelude" => pick(lang, "Boss Prelude", "ボス戦前", "头目战前奏", "보스전 전주"),
        "credits" => pick(lang, "Two of Braves", "トゥー・オブ・ブレイブス", "双勇者 (通关职员表)", "투 오브 브레이브스"),
        "navi-customizer" => pick(lang, "Navi Customizer", "ナビカスタマイザー", "领航员改装系统", "내비 커스터마이저"),
        "winner-1" => pick(lang, "Enemy Deleted!", "エネミーデリート!", "敌方删除！", "에너미 딜리트!"),
        "pavilion-comp" => pick(lang, "Pavilion Comp", "パビリオンの電脳", "博览馆电脑", "파빌리온 전뇌"),
        "cyberbeasts" => pick(lang, "Cybeasts", "電脳獣", "电脑兽", "전뇌수"),
        "crossover-battle" => pick(lang, "Crossover Battle", "クロスオーバーバトル", "交错对战", "크로스오버 배틀"),
        "shark-chase" => pick(lang, "Shark Chase", "シャークチェイス", "鲨鱼追逐", "샤크 체이스"),
        "acdc-town" => pick(lang, "ACDC Town", "秋原町", "秋原镇", "아키하라 마을"),
        "expo" => pick(lang, "Welcome to Expo", "万博へようこそ", "欢迎来到博览会", "박람회에 어서 오세요"),
        _ => return None,
    })
}
