# EXE3 战斗芯片全量 ROM 对照审计报告（Standard / Mega / Giga / Program Advance）

> **审计日期**：2026-11-29  
> **审计范围**：`/home/harp/projects/saito/content/exe3/chips/`（全部 36 个子目录）、`content/exe3/lib/`、`content/exe3/compat/chips.toml`、`content/exe3/compat/assets.toml`  
> **对照基准 ROM**：`bn3.gba`（`A6BE` White US，主对照）与 `bn3b.gba`（`A3XE` Blue US，Blue 版独占 Giga 芯片对照）；所有结论均经 Thumb-1 实际反汇编与 ROM 数据表核验，未修改任何项目源码。

---

## 1. ROM 验证锚点与数据表核对

| 表 / 入口 | A6BE (`bn3.gba`) | A3XE (`bn3b.gba`) | 说明 |
| :--- | :--- | :--- | :--- |
| `ChipDataTable` | `0x08011510` | `0x08011510` | 步长 `0x20`（385 条）；`+0x06` element、`+0x07` family、`+0x08` sub、`+0x0A` MB、`+0x0B` 战斗/暗转标志（bit `0x40` = 暗转）、`+0x0C` u16 attack、`+0x13` `chips.toml` flags |
| `PlayerActionJumpTable` | `0x080B0DA4` | — | 按 `family * 4` 索引玩家芯片动作（`0x00..0x3D`） |
| `DimmingChipParamTable` (fam `0x0C`) | `0x080B1AF0` | `0x080B1BE0` | 按 `sub * 2` 查 `(effect_id, param)` |
| `NaviDimParamTable` (fam `0x14`) | `0x080473E4` | `0x080473CC` | 按 `sub * 2` 查 `(effect_id, param)`；`0x080474D8` 对 Bass/Bass+/Serenade 执行 `0x080B595C` 暗洞/暗执照校验 |
| `VirusDimParamTable` (fam `0x1F`) | `0x08047494` | — | 9 张病毒芯片召唤参数表，先经 `0x080076EA` 校验 Virus Breeder 捕获状态 |
| `DimmingEffectJumpTable` | `0x08008D00` | `0x08008D00` | `0x08008E54` 统一派发；Blue 独占：`FoldrBak` (`eff=0x34`, `0x080FC50C`)、`DeltaRay` (`0x46`, `0x080FD840`)、`Bass+` (`0x48`, `0x080FD970`)、`AlphArmΩ` (`0x4E`, `0x080FDDA0`) |
| `ActorObjJumpTable` (Pool 1) | `0x0800369C` | — | 0-indexed（经 `0x08002F8C` / `0x080030F4` 核实），领航员与病毒召唤体入口 |
| `AttackObjJumpTable` (Pool 3) | `0x080038C4` | — | `0x08003084`/`30A0`/`30BC` 攻击与置物对象入口 |
| `EffectObjJumpTable` (Pool 4) | `0x08003C18` | — | `0x080030D8` 特效对象入口 |
| `SpritePointersList` | `0x0802477C` | — | `[0x08002704]`；分类偏移 `0x00..0x20` |
| `PlaySoundEffect` / `gSongTable` | `0x080005B8` / `0x08148B6C` | — | 331 条音频索引 |

> **基础数据表核验结论**：`content/exe3/compat/chips.toml` 中全部 **344 张可用战斗芯片**（`0x001..0x15F`，不含空槽 `0x139..0x13F`）的 `element`、`family`、`subfamily`、`mb`、`power`、`flags` 与 `bn3.gba`/`bn3b.gba` 原始 `ChipDataTable` **0 差异**。差异全部集中在芯片行为脚本（`content/exe3/chips/`、`content/exe3/lib/`）与资源映射（`content/exe3/compat/assets.toml`）。

---

## 2. 总览统计

本次共审计 **344 张可用战斗芯片**（外加 `invalid` / `zeroed` 2 个内部占位芯片，总计 346 条实现，已对跨模块交叉核验项去重，每张可用芯片严格计入一次）：

### 2.1 按严重级别汇总（344 张可用芯片）

| 严重级别 | 定义 | 数量 | 占比 |
| :--- | :--- | ---: | ---: |
| **P0** | 严重机制不符 / 缺失原作场上置物或召唤体 / 暗转与非暗转错误 / 核心发动条件或指令系统缺失 / 严重判定错误 | **218** | **63.4%** |
| **P1** | 核心框架存在，但攻击范围、段数、伤害公式、`hit_mod` 无敌帧、属性/异常状态、破防/破格、计时或发动门槛与 ROM 不符 | **93** | **27.0%** |
| **P2** | 机制与数值基本正确，仅精灵 ID (`cc-ii`)、动画索引、调色板、朝向/阴影、姿势或 SFX ID 与 ROM 不符 | **22** | **6.4%** |
| **OK** | 行为、判定、数值与视听表现基本符合原作 ROM | **11** | **3.2%** |
| **合计** | 全部可用芯片（Standard + Mega + Giga + PA） | **344** | **100.0%** |

> 附：`invalid` 与 `zeroed` 2 个内部系统占位芯片均判定为 **OK**。

### 2.2 按芯片等级分类统计

| 芯片类别 | 总数 | P0 | P1 | P2 | OK |
| :--- | ---: | ---: | ---: | ---: | ---: |
| **Standard（标准）** | 200 | 99 | 72 | 21 | 8 |
| **Mega（百万）** | 86 | 69 | 14 | 1 | 2 |
| **Giga（十亿，含双版本独占 & V5）** | 26 | 23 | 3 | 0 | 0 |
| **Program Advance（程序强化）** | 32 | 27 | 4 | 0 | 1 |
| **合计** | **344** | **218** | **93** | **22** | **11** |

### 2.3 按审计模块分组统计（已去重）

| 分组 | 涵盖目录 / 文件 | 芯片数 | P0 | P1 | P2 | OK |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| **Group 1** | `cannon`, `airshot`, `spreader`, `bomb`, `punch`, `wave`, `panlout` | 36 | 9 | 27 | 0 | 0 |
| **Group 2** | `sword`, `zapring`, `yoyo`, `elec`, `magnum` | 50 | 20 | 22 | 8 | 0 |
| **Group 3** | `trap`, `anti`, `barrier`, `stealth`, `invis`, `poison`, `wind`, `rockcube`, `metagel`, `grab`, `stage`, `geddon`, `recov`, `gauge`, `plus`, `copydmg` *(另含 `invalid`/`zeroed` 2 项 OK)* | 69 | 24 | 21 | 14 | 10 |
| **Group 4** | `utility/init.luau`（含 9 张病毒芯片、`Fanfare`/`Discord`/`Timpani`、`Totem`、`Needler`、`Meteors`、`LavaCan`、`Team` 等） | 52 | 49 | 3 | 0 | 0 |
| **Group 5** | `mega/init.luau`, `giga/init.luau` (+ `bassgs`) | 26 | 23 | 3 | 0 | 0 |
| **Group 6** | `navis/part1.luau` + `helpers.luau`（9 位领航员 V1–V5/V3） | 43 | 30 | 13 | 0 | 0 |
| **Group 7** | `navis/part2.luau` + `helpers.luau`（8 位领航员 V1–V5/Punk） | 36 | 36 | 0 | 0 | 0 |
| **Group 8** | `pa/init.luau`（全部 32 张 Program Advance） | 32 | 27 | 4 | 0 | 1 |
| **合计** | **全部 36 个子目录** | **344** | **218** | **93** | **22** | **11** |

---

## 3. 跨模块系统性缺陷汇总

### 3.1 23 张「非暗转芯片」被错误实现为暗转（P0）
在 BN3 ROM 中，仅 `ChipData+0x0B` bit `0x40` 置位（对应 `family 0x0C / 0x14 / 0x1F / 0x35`）的芯片会进入时停暗转。当前实现中有 **23 张非暗转芯片**错误使用了 `"dimming"` 控制器与横幅冻结画面：
- **Standard（8 张）**：`Barrier`、`Barr100`、`Barr200`、`Aura`（`fam=3`）、`Invis`（`fam=17`）、`Spice1`、`Spice2`、`Spice3`（`fam=43`）
- **Mega（9 张）**：`LifeAura`（`fam=3`）、`FullCust`（`fam=45`）、`ZeusHamr`（`fam=47`）、`StandOut`、`Salamndr`（`fam=59`）、`WatrLine`、`Fountain`（`fam=60`）、`Ligtning`、`Bolt`（`fam=61`）
- **Giga（1 张）**：`DarkAura`（`fam=3 sub=8`）
- **Program Advance（5 张）**：`H-Burst`（`fam=24`）、`EvilCut`（`fam=41`）、`HyperRat`（`fam=29`）、`EverCrse`（`fam=5`）、`500Barr`（`fam=3`）

### 3.2 场上置物 / 召唤体缺失或无敌化（P0）
大量原作会在战场上放置可破坏实体、触发式机关或独立召唤体的芯片，当前被简化为瞬间判定或无敌假物体：
- **乐器置物（3）**：`Fanfare` / `Discord` / `Timpani`（ROM 放置 40 HP `04-1f` 置物，每 80/120/120f 周期生效，存活 6000f，且受同场单置物替换 `0x080B5712` 约束）。
- **炮台与辅助置物（10）**：`Needler1..3`（巡逻机枪 `04-1e`）、`Totem1..3`（50 HP `04-25`，每次回 **120 HP** 且按阶循环 1/2/3 次）、`Team1`/`Team2`（40 HP `04-26`，受击加速射弹 / 低血回 500 HP）、`Sensor1..3`（缺 40/60/80 HP、错误扇形扫描、Sensor3 未重置）。
- **陨石石碑与地形置物（11）**：`RndmMetr`/`HoleMetr`/`ShotMetr`（40/60/80 HP `0c-0f` 石碑，实时掉陨石）、`BlkBomb1..3`（100 HP 抛物线炸弹，**仅火属性触发引爆**而非自动定时爆）、`Prism`（`0xFFFF` 折射周围 8 格伤害而非普通 300 HP 墙）、`Anubis`/`PoisPhar`（300 HP 毒雕像而非无敌特效）、`Wind`/`Fan`（80 HP `04-0b` 而非无敌假物）、`Hole`（1 HP DarkHole `10-42` 而非破 3 格）。
- **棋子、雕像、毒面具与其他（12）**：`Pawn`（10 HP 且需按 A 触发挥砍）、`Knight`（300 HP 跳格子裂地置物，当前完全缺置物）、`PoisMask`/`PoisFace`（按住 A 持续喷毒置物 `10-10`）、`FireRatn`（200 HP 火属性引爆置物）、`Shake1..3`、`RockArm1..3`、`GodStone`/`OldWood`（需洞格前置）、`TimeBom+`（200 HP 可破坏炸弹）。
- **9 张病毒芯片 + 领航员实体（12+）**：全部 9 张病毒芯片（`Mettaur`..`Scuttlst`）共用假自动命中桩，缺 `Type1` 病毒实体与 Virus Breeder 捕获/喂食机制；`BowlMan` 缺保龄球瓶、`MistMan` 缺神灯与雾云、`DarkMan` 缺 16–24 只 1 HP 蝙蝠与 DarkHole、`DrillMan` 缺三阶段钻头与落石。

### 3.3 引擎级 / 跨芯片共性 Bug
1. **无符号计时器与暗转软锁体检**：
   - commit `cd6db97` 已修复暗转控制器的 `i16` 下溢软锁；本次静态扫描全库 18 处 `u8`/`u16` 倒计时字段，均在正值初始化后步进 `-1` 并立即判 `<= 0`，**当前未发现残留的暗转软锁或无符号下溢死循环**。
   - **发现 2 处 `u8` 正向计数回绕 Bug**：`content/exe3/chips/trap/init.luau:631`（`PawnState.tick: u8`，但 `life = 420`）与 `content/exe3/chips/utility/init.luau:550`（`TotemState.tick: u8`，但 `life = 360`），在第 256 帧发生 `255 -> 0` 回绕导致周期模运算提前触发。
2. **`AntiNavi` (`0x08008EA0`) 机制不完整**：当前仅取消对方领航员芯片，未实现原作的**夺取并反向释放**；且 6 张领航员 PA（`2xHero`/`DeuxHero`/`BigHeart`/`GtsShoot`/`PrixPowr`/`MstrStyl`）完全未接入 `AntiNavi` 检测。
3. **空洞脚下中止校验 (`IsPanelSolid` `0x0800BF4A`) 缺失**：17 位领航员中有 **12 位**在玩家悬停于洞格（AirShoes 等）时应直接中止不发动，当前 `navis/helpers.luau` 均未检查。
4. **暗芯片与属性风格发动门槛缺失**：`0x080B595C`（需场上激活 DarkHole 或装备 DarkLicense）对 `DarkAura`、`Bass`、`Bass+`、`Serenade` 均未校验；8 张属性 Mega 芯片（`StandOut`/`Salamndr` 等）均未校验对应 Style 属性。
5. **属性加攻芯片未过滤属性（P1）**：`Fire+30`、`Aqua+30`、`Elec+30`、`Wood+30` 共用通用 `attack_plus`，错误地对**任意属性**攻击芯片生效。
6. **`Shadow` 错误叠加 `flash` 无敌帧（P0）**：`stealth/init.luau:31` 在 `shadow_ticks` 外同时写入 `flash = 360`，在 `flash_timer_first` 规则下连非穿透的**剑类攻击**也无法命中 Shadow 状态。
7. **`@exelib/projectile` 飞行速度减半（P1）**：EXE3 `AttackObj #1`（`0x080D5D5E`）为 **1 格/帧**，当前共享库为 **1 格/2 帧**，影响 `Cannon` 线等投射物。
8. **指令输入与连打机制大面积缺失**：`VarSwrd` 缺第 6 指令且第 4 指令被覆盖（PVP Side-1 方向反转）、`GutPunch`/`GutStrgt`/`GutImpct` 缺蓄力 A 指令、`DeltaRay` 缺按键追击、`JapanMan` 缺 A/B 交替连打、`MetalMan` 缺 30f 瞄准、`FlamMan`/`DesrtMan`/`Punk` 缺隐藏指令、`Zeta PA` 12 张缺自由移动+真实子芯片无限连发。

### 3.4 反直觉 ROM 真相清单（谨防凭英文名或后作记忆修偏）
以下条目均由本次对 `bn3.gba` / `bn3b.gba` 反汇编直接证伪：
- **`HoleMetr` (`0x058`) 不破格**：`0x080D6C08[0]` 中 `r7[5]=0` 跳过破格函数；它只是按列横扫敌阵的陨石石碑，当前破格逻辑系按英文名臆造。
- **`RedWave` (`0x03E`) 不造岩浆格、`MudWave` (`0x03F`) 不破格**：ROM `0x080E40F4` 均不修改地形，且 `MudWave` 为 `hit_mod=1`（无闪烁无敌）。
- **`IceWave1..3` (`0x05A..0x05C`) 不造冰格**：ROM 仅发射 1 道在相邻两行间正弦振荡的可破坏冰波。
- **`Roll V1–V3` (`0x0DB..0x0DD`) 回血为 `30 / 50 / 80 HP`**：ROM `0x080B6F0C` 表值为 `[15, 25, 40]` 经 `0x080D8252` 乘 2（60/100/140 是 BN4–6 数值）。
- **`BubblMan V1–V5` (`0x0F2..0x0F6`) 无残血加发机制**：ROM `0x080BC3EE` 固定为 `tier + 6`（`6/7/8/9/10` 发）直线水泡。
- **`GaiaBlad` (`0x114`) 攻击范围是 2×1（Long）而非 2×3**：`bn3` 与 `bn3b` 的 `0x080D65CC` 第 12 行均为 `region 0x02`（Long）。
- **`FoldrBak` (`0x12F`) 不回血，但补满 Custom 槽**：`bn3b 0x080FC50C` 调用 `0x08010B20` 满槽 + `0x08011454` 复原芯片夹，完全无回血代码。
- **`DarkMan V1–V5` (`0x124..0x128`) 不附带任何异常状态**：蝙蝠无失明/混乱/麻痹，核心副效果是召唤 DarkHole（`10-42`）。
- **`GrabBack` / `GrabRvng` (`0x088`/`0x089`) 不夺回被抢的格子**：仅按被夺格数对全体敌人连续砸下对应次数的 `0c-0e` 攻击。
- **`Magnum3` (`0x053`) 不是十字爆**：与 Magnum1 一样是 3 格纵列爆破（从右向左每 5f 扫一列）。

### 3.5 `content/exe3/compat/assets.toml` 映射错位与缺漏汇总
- **已确认错位**：
  - `spreader = 0x74` 与 `cannon = 0x7d` 名称互换（原作 Cannon 用 `0x74`，Spreader/Bubbler/HeatShot 发射用 `0x7D`）。
  - `boomer = "0c-1e"` 实为 **DashAtk** 冲刺特效（Boomerang 精灵在 `10-08`/`10-1b` 等，详见 Group 2）。
  - `nobeam = "0c-35"` 实为 **StepCros** 交叉斩击特效（NoBeam 实际为 `0c-36`）。
  - `fan = "0c-23"` / `fan-alt = "0c-39"` 实为 **TimeBomb / TimeBom+**（Wind/Fan 实际为 `04-0b` pal 0/1）。
  - `mine` 实为 `0c-24`、`prism` 实为 `0c-2a`/`0c-2f`、`anubis`/`poisphar` 实为 `0c-29`（当前 `guardian = 0c-29` / `anubis = 0c-2b` 存在偏移错位，详见 Group 3/5）。
- **高频缺失精灵（需补充提取）**：`04-01..04-17`（9 病毒）、`04-0b`（Wind/Fan）、`04-1d`（Arrow）、`04-1e`（Needler）、`04-1f`（Fanfare/Discord/Timpani）、`04-21`（RockArm）、`04-25`（Totem）、`04-26`（Team1/2）、`08-07`（AlphaArm）、`0c-0f`（MeteorRock）、`0c-24`（Mine）、`0c-2c`（OldWood）、`0c-2f`（Prism throw）、`0c-36`（NoBeam）、`10-00`（ShockWav/GutsWave）、`10-0a`（Meteor）、`10-0f`（AirShot）、`10-10`（PoisMask）、`10-1f`（Guardian lightning）、`10-22`（FlamMan/Burner pillar）、`10-24`（Snake）、`10-28`/`10-29`（Team1 shot）、`10-2d`（CannBall/IceBall/LavaBall）、`10-35`（DrillMan/MomQuake boulder）、`10-3b`（Wave）、`10-3e`（KingMan CheckMate）、`10-40`/`10-43`（Punk）、`10-42`（DarkHole）、`10-46`（Sensor beam）、`10-49`（MistMan cloud）。
- **高频缺失 SFX**：`0x7A`、`0x7F`、`0x8D`、`0x8F`、`0x98`、`0xA9`、`0xB3`、`0xB4`、`0xBB`、`0xBD`、`0xCD`、`0xD2`、`0xEF`、`0xF5`–`0xF8`、`0x10D`、`0x111`、`0x114`、`0x117`、`0x11B`、`0x124` 等（完整清单见各模块详细表）。

---

## 4. 建议修复分期路线图

结合 **EXE3 PvP 联机高频芯片**、**缺陷严重度（P0 优先）** 与 **公共设施复用效率**，建议分 5 个批次推进：

| 批次 | 目标定位 | 涵盖核心内容与优先芯片 | 预期收益 |
| :--- | :--- | :--- | :--- |
| **Batch 0**<br>*(基础设施与低成本高收益速赢)* | 根治跨模块系统性错误与明显数值/标志位 Bug | 1. 修正 **23 张误暗转芯片**（`Barrier`/`Aura`/`LifeAura`/`DarkAura`/`Invis`/`FullCust`/`ZeusHamr`/6 张属性 Mega/`Spice1..3`/5 张非暗转 PA）<br>2. 修复 `Shadow` 误设 `flash`、`Fire/Aqua/Elec/Wood+30` 属性校验、`Mole1..3` 持续时间翻倍<br>3. 修复 `assets.toml` 错位映射、`Pawn`/`Totem` 两处 `u8` tick 回绕、`@exelib/projectile` 1 格/帧速度<br>4. 速修高频单点数值：`Muramasa` (2×1)、`BambSwrd` (`hit_mod=1`)、`Roll` (30/50/80)、`FoldrBak` (满槽不回血)、`Balance` (向上取整)、`DarkAura` (3000f/pal 2)、`500Barr` (pal 3)、`Recov` (15f 硬直) | 一次性消除 ~45 张芯片的 P0/P1 核心失真，立刻改善对战手感 |
| **Batch 1**<br>*(置物、陷阱与反制体系)* | 补齐 EXE3 联机博弈核心：可破坏置物与四大 Anti 陷阱 | 1. 统一可破坏置物基类（HP、同场单置物替换 `0x080B5712`、Wind/Breaking 交互、6000f 生命周期）<br>2. 重构四大反制：`AntiNavi`（真实夺取反打）、`AntiSwrd`（3 道 SonicBoom）、`AntiRecv`（2 倍治疗反伤）、`AntiDmg`（去假无敌帧）<br>3. 重构核心置物与陷阱：`Mine`（120f 轮换+6000f）、`TimeBomb`/`TimeBom+`、`BlkBomb1..3`（仅火引爆）、`Prism`（8 格折射）、`Pawn`/`Knight`、`Sensor1..3`（含 Sensor3 重置）、`Anubis`/`PoisPhar`、`Wind`/`Fan`、`Fanfare`/`Discord`/`Timpani`、`Hole`（DarkHole 10-42）、`Snake`、`Totem1..3`、`Needler1..3`、`Team1`/`Team2`、`RndmMetr`/`HoleMetr`/`ShotMetr`、`PoisMask`/`PoisFace` | 补齐 EXE3 战术控场与陷阱博弈的完整闭环 |
| **Batch 2**<br>*(高频标准攻击与指令芯片)* | 修复高出场率输出/连招芯片的判定、`hit_mod` 与指令系统 | 1. `VarSwrd`（补第 6 指令、修第 4 指令、修 PVP Side-1 方向、去 SonicBoom 穿透）<br>2. `StepSwrd`/`StepCros`/`Slasher`、`AirSwrd`（3 行吹风）、`Tornado`（沙地 16 连+十字）、`Bubbler`/`HeatShot` 线（`hit_mod=0`）、`ZapRing1..3`（`hit_mod=0`+真实麻痹）、`Plasma1..3`<br>3. `DublBomb`/`TrplBomb`（2/3 连发 wide `hit_mod=0`）、`SnglBomb`（wide）、`CannBall`（破格）、`GutPunch`/`GutStrgt`/`GutImpct`（蓄力指令与飞拳）<br>4. `Magnum1..3`（列/行扫射）、`NoBeam1..3`（后格严格判定）、`LavaCan1..3`/`Volcano`（吸岩浆）、`IceWave1..3`、`Arrow1..3`、`Burner`/`Burning`、`Rope1..3`、`BigWave`/`Wave` 线（去臆造地形改动）、`DashAtk`/`Condor`、`GrabBack`/`GrabRvng`、`Spice1..3`、`CrsShld1..3`、`Geyser`、`Panic`、`CopyDmg`、`Repair` | 全面恢复标准芯片连招（如 Plant+VarSwrd、Sand+Tornado、Bubbler 压制）与真实帧数据 |
| **Batch 3**<br>*(领航员芯片 V1–V5 体系)* | 补齐 `IsPanelSolid` 与实体级领航员演出/判定 | 1. 改造 `navis/helpers.luau`（接入 `AntiNavi` 夺取、`IsPanelSolid` 洞格中止、按领航员独立控制阴影与登场 SFX）<br>2. **第一梯队（对战高频）**：`PlantMan`（全场破防穿透 `hit_mod=0` + 120/180/240f 藤蔓）、`FlashMan`（全场 `hit_mod=0` + 阶梯麻痹）、`ProtoMan`（站位校验+穿盾）、`BowlMan`（保龄球瓶实体）、`MistMan`（神灯+雾云+前后双打）、`DarkMan`（16–24 蝙蝠+DarkHole）、`JapanMan`（原地 A/B 连打+非剑属性）、`MetalMan`（30f 瞄准）<br>3. **第二梯队**：`DrillMan`（三阶段）、`KingMan`（固定 3 格+CheckMate）、`Punk`（5 招指令）、`DesrtMan`（沙地化）、`FlamMan`（可控火柱）、`GutsMan`（3 行冲击波）、`BeastMan`（3 爪交汇）、`BubblMan`（6–10 直线水泡）、`Roll`（站位校验） | 彻底告别领航员芯片「全屏/单体瞬发假判定」现状，还原 BN3 标志性领航员生态 |
| **Batch 4**<br>*(Mega / Giga / Program Advance 与病毒芯片)* | 补齐终盘核弹芯片、风格/暗洞门槛与 PA 生态 | 1. 实装 `DarkHole/DarkLicense`（`0x080B595C`）与 4 属性 Style 门槛；修复 `NavRcycl` 全局槽、`Jealousy` 毁手牌、`GaiaSwrd`/`GaiaBlad` 全手牌吸收、`Poltrgst`/`ZeusHamr`/`GodStone`/`OldWood`/`Guardian`（电反击）/`Meteors`<br>2. 重构 Giga 核弹：`DeltaRay`（按键追击）、`Bass`/`Bass+`/`BassGS`、`Serenade`、`AlphArmΣ`/`AlphArmΩ`<br>3. 重构 12 张 **Zeta PA**（300f 自由移动真隐身 + 无限触发真实子芯片）与核心 PA（`BodyGrd` 18 镖、`2xHero`/`DeuxHero`、`BigHeart`、`GtsShoot`、`MomQuake`、`PrixPowr`、`MstrStyl`、`GelRain`、`ElemSwrd`、`H-Burst`/`EvilCut`/`HyperRat`/`EverCrse`），同步修正 PA 配方表（补 6 条 `BubSprd`/`HeatSprd` 顺子配方，删 `GelRain`/`PrixPowr` 伪配方）<br>4. 实装 9 张病毒芯片 `Type1` 召唤体与 Virus Breeder 捕获/喂食倍率 | 完成 EXE3 全 344 芯片的高还原度收官 |

---

## 5. 按芯片模块分组的全量 ROM 审计明细

> 说明：以下 8 个章节完整保留对全部 344 张可用芯片（+ 2 张内部占位）的逐芯片 ROM 反汇编核对表与技术证据；原 Batch 3 中与 `utility/init.luau` 重叠的 7 张芯片（`CrsShld1..3`、`GrabBack`、`GrabRvng`、`Team1`、`Team2`）已统一归入 **5.4 Group 4 (`utility/init.luau`)**，避免重复统计。


---

### 5.1 Group 1：基础投射、炸弹、拳击、冲击波与破格（36 张）

**Scope**: Basic Projectiles, Bombs, Punches, Waves, PanlOut (7 chip modules, 36 chips + shared helpers)
**ROM Reference**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US, `A6BE`)
**Audit Date**: 2026-11-28 (read-only audit; no workspace files modified)

---

#### Executive Summary

| Severity | Count | Chips |
| :--- | :---: | :--- |
| **P0** | **9** | `dublbomb`, `trplbomb`, `cannball`, `airswrd`, `gutpunch`, `gutstrgt`, `gutimpct`, `dashatk`, `bigwave` |
| **P1** | **27** | `cannon`, `hicannon`, `m-cannon`, `airshot1`, `airshot2`, `airshot3`, `shotgun`, `v-gun`, `sidegun`, `spreader`, `bubbler`, `bub-v`, `bublside`, `heatshot`, `heat-v`, `heatside`, `minibomb`, `snglbomb`, `hammer`, `shockwav`, `sonicwav`, `dynawave`, `wave`, `redwave`, `mudwave`, `panlout1`, `panlout3` |
| **P2** | **0** | *(Every chip with P2 presentation issues also had at least one P0 or P1 behavior/timing/`hit_mod` issue)* |
| **OK** | **0** | — |
| **Total** | **36** | *(Chip table metadata — codes, element, rarity, MB, power — matches ROM `0x08011510` 36/36 with 0 diffs)* |

##### Systemic Cross-Module Findings

1. **`AttackObj #1` Projectile Speed is Half Speed (`lib/projectile.luau` / `@exelib/projectile/projectile.luau`)**:
   - ROM `0x080D5D5E` / `0x080D5DCC` sets `timer = 1`, decrements `1 -> 0` on the next tick, branches on `bne`, and immediately advances `panel_x += dir` **every 1 tick**.
   - `@exelib/projectile/projectile.luau:32,125-129` (`WAIT = 1`, `left = timer - 1; if left >= 0 then return`) uses EXE5/6 speed (**1 panel every 2 ticks**), affecting Cannon/HiCannon/M-Cannon and any other EXE3 chips using `lib/projectile.luau`.
2. **Swapped & Missing Entries in `content/exe3/compat/assets.toml`**:
   - **Swapped SFX names**: `spreader = 0x74` is actually Cannon & AirShot's firing SFX (`0x080B129A`, `0x080B25BA`), while `cannon = 0x7d` is Spreader's firing SFX (`0x080B1554`). Because `lib/cannon.luau:36` uses `asset.sound("cannon")`, Cannons play Spreader's SFX (`0x7D`).
   - **Misnamed sprite**: `boomer = "0c-1e"` (`assets.toml:56`) is actually DashAtk's dash aura/shield attachment (`0x080B2FAA`, 3 anims `0/1/2`).
   - **Missing sprites in `assets.toml`**: `10-00` (ShockWav/SonicWav/DynaWave/BigWave), `10-0f` (Wind/AirSwrd gust), `10-2d` (CannBall/IceBall/LavaBall), `10-3b` (Wave/RedWave/MudWave).
   - **Missing SFX in `assets.toml`**: `0x7A` (ShockWav), `0x7F` (GutPunch/Strgt/Impct punch), `0xA9` (HeatShot burst), `0xB3` (Wave/RedWave/MudWave panel advance), `0xB4` (DashAtk windup), `0xBD` (GutPunch rocket fist launch), `0xCD` (Hammer empty-panel thud), `0x117` (GutPunch command input chime).
3. **Unused Helper**: `content/exe3/lib/airshot.luau` is dead code; `chips/airshot/init.luau` bypasses it with a bespoke instant hitscan implementation.
4. **ROM `hit_mod` Table Verification (`0x080AF3B0` / `0x080AF3B6`)**:
   - Enemy table (`0x080AF3B0`): `[0, 1, 0, 3, 5, 7]`. Player table (`0x080AF3B6`): `[0, 1, 3, 3, 5, 7]`.
   - Bit `0x01` = flinch (`0x080B4D5E`), bit `0x02` = mercy flash 60t invuln / 120t blink (`0x080B4D3A`), bit `0x04` = heavy stagger / `status |= 0x0200` (`0x080B4D7E`).

---

#### Complete Chip Audit Table (36 Chips)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| **Cannon** | `0x001` | `chips/cannon/init.luau` | **P1** | Projectile moves 1 panel/2t (ROM: 1 panel/1t); fires tick 16 / 34t total (ROM: tick 15 / 32t); `hit_mod=3` vs ROM `2` (PvP same, virus differs). **P2**: plays SFX `0x7D` instead of `0x74`; missing raise-flash SimpleEffect `0x08`. | Action `0x080B11E4` (fam 4 sub 0); Atk #1 `0x080D5D28` var `0x00` (`0x080D5CE4`); MegaMan `0x20`/`0x21`; Arm `0c-01` anim 0 pal 0; Raise FX `0c-01` anim 1 (Eff #0 `0x08`); Spark `14-01` anim 0 (`0x07`); SFX `0x74` (`0x080B1298`). |
| **HiCannon** | `0x002` | `chips/cannon/init.luau` | **P1** | Same as Cannon (half projectile speed, timing 16/34 vs 15/32, `hit_mod` 3 vs 2). **P2**: SFX `0x7D` vs `0x74`; missing raise-flash `0c-01` anim 1. | Action `0x080B11E4` (fam 4 sub 1); Atk #1 `0x080D5D28` var `0x00`; Arm `0c-01` anim 0 pal 1; Raise FX `0c-01` anim 1; Spark `14-01` anim 0; SFX `0x74`. |
| **M-Cannon** | `0x003` | `chips/cannon/init.luau` | **P1** | Same as Cannon (half projectile speed, timing 16/34 vs 15/32, `hit_mod` 3 vs 2). **P2**: SFX `0x7D` vs `0x74`; missing raise-flash `0c-01` anim 1. | Action `0x080B11E4` (fam 4 sub 2); Atk #1 `0x080D5D28` var `0x00`; Arm `0c-01` anim 0 pal 2; Raise FX `0c-01` anim 1; Spark `14-01` anim 0; SFX `0x74`. |
| **AirShot1** | `0x004` | `chips/airshot/init.luau` | **P1** | Instant hitscan loop instead of traveling Atk #1 (1 panel/tick); `WIND_ELEM` instead of Null `0`; `hit_mod=1` vs ROM `4` (heavy stagger); manual teleport push ignores hit outcome/invis and scans non-hittable objects; fires tick 5 / 20t total vs ROM tick 2 / 10t total. **P2**: `sparks.plain` vs `14-01` anim 0; extra `small-puff` FX; `z=0x14` vs `0x18`; SFX at tick 0 vs tick 2. | Action `0x080B2558` (fam 25 sub 0); Atk #1 `0x080D5D28` var `0x0C` (`0x080D5D14`: `col_self=0x07` knockback, `hit_mod=4`, spark `0x0B`); MegaMan `0x38` (10t, fire tick 2); Arm `0c-05` anim 0 pal 0; Spark `14-01` anim 0; SFX `0x74` (`0x080B25B8`). |
| **AirShot2** | `0x005` | `chips/airshot/init.luau` | **P1** | Same as AirShot1. | Action `0x080B2558` (fam 25 sub 1); Atk #1 var `0x0C`; Arm `0c-05` anim 0 pal 0; Spark `14-01` anim 0; SFX `0x74`. |
| **AirShot3** | `0x006` | `chips/airshot/init.luau` | **P1** | Same as AirShot1. | Action `0x080B2558` (fam 25 sub 2); Atk #1 var `0x0C`; Arm `0c-05` anim 0 pal 0; Spark `14-01` anim 0; SFX `0x74`. |
| **ShotGun** | `0x00A` | `chips/spreader/init.luau` | **P1** | Recovery 21t (`RECOVER_TICKS=0x14`) vs ROM 15t (`0x0F`, `0x080B159A`) + `SetPlayerEndlag(0x0F)`. **P2**: plays burst SFX `0x74` whereas ROM plays NO burst SFX (`0x00`). | Action `0x080B14D2` (fam 6 sub 0); Atk #2 `0x080D5E50` (`0x080D5FEA`); Region `0x02` (`long`), `hit_mod=0` (`0x080D5FE2`); Muzzle `0c-1c` anim 0 (`0x1F`); Burst `14-04` anim 0 (`0x0E`); Fire SFX `0x7D` (`0x080B1552`), Burst SFX none (`0x00`, `0x080D5FAF`). |
| **V-Gun** | `0x00B` | `chips/spreader/init.luau` | **P1** | Same as ShotGun (recovery 21t vs 15t + endlag 15; extra burst SFX `0x74`). | Action `0x080B14D2` (fam 6 sub 1); Atk #2 `0x080D5E50`; Region `0x0D` (`fork`), `hit_mod=0`; Burst `14-04` anim 0; Fire SFX `0x7D`, Burst SFX none. |
| **SideGun** | `0x00C` | `chips/spreader/init.luau` | **P1** | Same as ShotGun (recovery 21t vs 15t + endlag 15; extra burst SFX `0x74`). | Action `0x080B14D2` (fam 6 sub 2); Atk #2 `0x080D5E50`; Region `0x04` (`wide`), `hit_mod=0`; Burst `14-04` anim 0; Fire SFX `0x7D`, Burst SFX none. |
| **Spreader** | `0x00D` | `chips/spreader/init.luau` | **P1** | Same as ShotGun (recovery 21t vs 15t + endlag 15; extra burst SFX `0x74`). | Action `0x080B14D2` (fam 6 sub 3); Atk #2 `0x080D5E50`; Region `0x0F` (`square`), `hit_mod=0`; Burst `14-04` anim 0; Fire SFX `0x7D`, Burst SFX none. |
| **Bubbler** | `0x00E` | `chips/spreader/init.luau` | **P1** | **`hit_mod=3` vs ROM `hit_mod=0`** (`0x080D5FE2`, grants unintended flinch + mercy invincibility!); recovery 21t vs 15t + endlag 15. **P2**: burst SFX `0x74` instead of ROM `0xAE` (`freeze`). | Action `0x080B14D2` (fam 6 sub 4); Atk #2 `0x080D5E50`; Region `0x02` (`long`), `hit_mod=0`; Burst `0c-16` anim 0 (`0x0F`); Fire SFX `0x7D`, Burst SFX `0xAE` (`0x080D5FB0`). |
| **Bub-V** | `0x00F` | `chips/spreader/init.luau` | **P1** | Same as Bubbler (`hit_mod=3` vs `0`; recovery 21t vs 15t; burst SFX `0x74` vs `0xAE`). | Action `0x080B14D2` (fam 6 sub 5); Atk #2 `0x080D5E50`; Region `0x0D` (`fork`), `hit_mod=0`; Burst `0c-16` anim 0; Fire SFX `0x7D`, Burst SFX `0xAE`. |
| **BublSide** | `0x010` | `chips/spreader/init.luau` | **P1** | Same as Bubbler (`hit_mod=3` vs `0`; recovery 21t vs 15t; burst SFX `0x74` vs `0xAE`). | Action `0x080B14D2` (fam 6 sub 6); Atk #2 `0x080D5E50`; Region `0x04` (`wide`), `hit_mod=0`; Burst `0c-16` anim 0; Fire SFX `0x7D`, Burst SFX `0xAE`. |
| **HeatShot** | `0x011` | `chips/spreader/init.luau` | **P1** | **`hit_mod=3` vs ROM `hit_mod=0`** (`0x080D5FE2`); recovery 21t vs 15t + endlag 15. **P2**: burst SFX `0x74` instead of ROM `0xA9` (missing from `assets.toml`). | Action `0x080B14D2` (fam 6 sub 8); Atk #2 `0x080D5E50`; Region `0x02` (`long`), `hit_mod=0`; Burst `0c-15` anim 0 (`0x10`); Fire SFX `0x7D`, Burst SFX `0xA9` (`0x080D5FB1`). |
| **Heat-V** | `0x012` | `chips/spreader/init.luau` | **P1** | Same as HeatShot (`hit_mod=3` vs `0`; recovery 21t vs 15t; burst SFX `0x74` vs `0xA9`). | Action `0x080B14D2` (fam 6 sub 9); Atk #2 `0x080D5E50`; Region `0x0D` (`fork`), `hit_mod=0`; Burst `0c-15` anim 0; Fire SFX `0x7D`, Burst SFX `0xA9`. |
| **HeatSide** | `0x013` | `chips/spreader/init.luau` | **P1** | Same as HeatShot (`hit_mod=3` vs `0`; recovery 21t vs 15t; burst SFX `0x74` vs `0xA9`). | Action `0x080B14D2` (fam 6 sub 10); Atk #2 `0x080D5E50`; Region `0x04` (`wide`), `hit_mod=0`; Burst `0c-15` anim 0; Fire SFX `0x7D`, Burst SFX `0xA9`. |
| **MiniBomb** | `0x014` | `chips/bomb/init.luau` | **P1** | **`hit_mod=3` vs ROM `hit_mod=0`** (`0x080D607C`); `collision.thrown` vs ROM `0x01` (`attack`); throw duration 27t vs ROM 22t + `SetPlayerEndlag(0x3C)`; SFX at tick 0 vs tick 7. **P2**: hole splash `14-02` anim `0` vs ROM Eff #5 `14-02` anim `1` rising `z_vel=+2`. | Action `0x080B15C0` (fam 7 sub 0, count `1` @ `0x080B1604`); Atk #3 `0x080D600C` (`0x080D61D0`); Region `0x01` (`single`, `0x080D61C0`), `hit_mod=0`; Sprite `0c-02` anim 0->1 pal 0; Blast `14-00` anim 0; Splash `14-02` anim 1 (`0x080EE20C`); SFX `0x77` throw / `0x76` hit. |
| **SnglBomb** | `0x015` | `chips/bomb/init.luau` | **P1** | **Wrong blast region**: `regions.single` (1 panel) vs ROM **`0x04` (`regions.wide`, 3 vertical panels)** at `0x080D61C0[1]`; `hit_mod=3` vs `0`; `collision.thrown` vs `attack`; throw timing. **P2**: palette `1` vs ROM `0`; splash anim `0` vs `1`. | Action `0x080B15C0` (fam 7 sub 1, count `1`); Atk #3 `0x080D600C`; Region `0x04` (`wide`), `hit_mod=0`; Sprite `0c-02` anim 0->1 pal 0; SFX `0x77` / `0x76`. |
| **DublBomb** | `0x016` | `chips/bomb/init.luau` | **P0** | **Only throws 1 bomb instead of 2 + wrong blast region**: ROM throws **2 consecutive bombs** (`0x080B1604[2]=2`, 2×50=100 dmg), each with `regions.wide` (3 vertical panels, `0x080D61C0[2]=0x04`) and `hit_mod=0` so both hit; Luau throws 1 bomb with `regions.long` and `hit_mod=3`. **P2**: palette `2` vs `0`; splash anim `0` vs `1`. | Action `0x080B15C0` (fam 7 sub 2, count `2` @ `0x080B1606`); Atk #3 `0x080D600C`; Region `0x04` (`wide`), `hit_mod=0`; Sprite `0c-02` anim 0->1 pal 0; SFX `0x77` / `0x76`. |
| **TrplBomb** | `0x017` | `chips/bomb/init.luau` | **P0** | **Only throws 1 bomb instead of 3 + wrong blast region**: ROM throws **3 consecutive bombs** (`0x080B1604[3]=3`, 3×50=150 dmg), each with `regions.wide` (3 vertical panels, `0x080D61C0[3]=0x04`) and `hit_mod=0` so all 3 hit; Luau throws 1 bomb with `regions.line` and `hit_mod=3`. **P2**: palette `3` vs `0`; splash anim `0` vs `1`. | Action `0x080B15C0` (fam 7 sub 3, count `3` @ `0x080B1607`); Atk #3 `0x080D600C`; Region `0x04` (`wide`), `hit_mod=0`; Sprite `0c-02` anim 0->1 pal 0; SFX `0x77` / `0x76`. |
| **CannBall** | `0x018` | `chips/bomb/init.luau` | **P0** | **Wrong attack object & missing panel crack/break**: ROM spawns **Atk #5** (`0x080D6394`, NO held bomb in hand, ballistic arc `speed=0x2C000, grav=-0x2800` to `x+3*dir`) and calls `0x0800B64C` on solid landing to **break unoccupied / crack occupied panel**; Luau reuses Bomb #3 and never cracks/breaks panels. **P2**: sprite `0c-02` vs **`10-2D` anim 0 pal 0**; explosion `14-00` vs `small-puff` `14-02` anim 1 (`0x15`); hit SFX `0x76` vs **`0x7B`** (`panel-crack`). | Action `0x080B15C0` (fam 7 sub 4); Atk #5 `0x080D6394` (`0x080D65B2`), landing `0x080D6550` -> `0x0800B64C`; `col_self=0x0B` (`thrown_break`), `hit_mod=3`; Sprite `10-2D` anim 0 pal 0; Land FX `14-02` anim 1 (`0x15`); SFX `0x77` throw / `0x7B` land. |
| **AirSwrd** | `0x02A` | `chips/punch/init.luau` | **P0** | **Missing 3 full-row Wind gust objects (Atk #0x1E)**: ROM fires `hit_mod=0` wide slash (100 dmg, Null+Sword) AND spawns 3 invisible Atk #0x1E wind projectiles (`0x080D9078`, 10 px/t, `col_self=0x4A`, Wind elem, 0 dmg) across all 3 rows that push all targets to back column, strip Barriers/Auras, and clear Sand; Luau uses `hit_mod=3` and only teleports enemies 1 panel at `x+dir`; fires tick 5 / 18t total vs ROM tick 7 / 29t (`0x1D`->`0x1F`). **P2**: `sparks.plain` vs `14-05` anim 12; extra `0x74` SFX. | Action `0x080B16CE` (fam 8 sub 8); Atk #6 `0x080D6664` (`0x080D65CC[8]`: `0c-0c` anim 0, `reg=0x04`, `hit_mod=0`, `extra=1`) + 3× Atk #0x1E `0x080D9078` (`0x080D926C`, `r4=0x80`, `col_self=0x4A`); MegaMan `0x1D`->`0x1F`; Arm `0c-00` anim 0; SFX `0x75` only. |
| **GutPunch** | `0x02F` | `chips/punch/init.luau` | **P0** | **Missing command-code rocket punch + wrong base collision/`hit_mod`**: Hold-A `[Down, Down+Fwd, Fwd]` fires full-row breaking rocket fist Atk #0x1A var 2 (`speed=16, panels=8, col_self=0x49`); base punch is **NOT breaking** (`col_self=0x07` knockback-only), `hit_mod=4` (no flash) vs Luau `3`, strikes tick 4 of `0x2B` (30t) vs Luau tick 6 of `0x2C` (20t). **P2**: anim `0x2C` vs `0x2B`; spark `breaking` vs `heavy`; extra `IMPACT_FX`; SFX `0x7E` vs `0x7F` (command chime `0x117`, fist `0xBD`). | Action `0x080B25F2` (fam 26 sub 0); Cmd `0x080B269C`->Atk #0x1A `0x080D89D8` var 2; Base `0x080B27FE`->Atk #7 (`r4=0x07020901`, `hit_mod=4`); MegaMan `0x2B` (base) / `0x2C` (cmd); Arm `0c-12` anim 0; Spark `14-05` anim 8 (`0x09`); SFX `0x7F` / `0x117` / `0xBD`. |
| **GutStrgt** | `0x030` | `chips/punch/init.luau` | **P0** | **Missing flying fist Atk #0x1A + missing 3× rocket punch command code**: Base move spawns flying fist Atk #0x1A var 0 (`0c-12` anim 1, 8 px/t, 2 panels, `col_self=0x07` knockback-only, `hit_mod=4`, NOT stationary `regions.long` breaking hitbox); Hold-A `[B, rel, B, Down, Back, Fwd, B]` fires **3** full-row breaking rocket punches 15t apart. **P2**: missing `0c-12` anim 1 projectile sprite; wrong spark/FX/SFX (`0x7E` vs `0x7F`+`0xBD`); 20t vs 14t. | Action `0x080B25F2` (fam 26 sub 1); Base `0x080B2892`->Atk #0x1A `0x080D89D8` var 0 (`0x080D89C8`); Cmd `0x080B26A4`->3× Atk #0x1A var 2; MegaMan `0x2C` (14t); Arm `0c-12` anim 0 / Projectile `0c-12` anim 1; SFX `0x7F`, `0xBD`, `0x117`. |
| **GutImpct** | `0x031` | `chips/punch/init.luau` | **P0** | **Missing 5-hit breaking command combo (560 dmg) + wrong base mechanics**: Base punch is 1-panel `col_self=0x07` (1-panel knockback, NOT 2 panels!), NOT breaking, **does NOT crack panel**, `hit_mod=4`; Hold-A `[Back, B, Down, B, Down, B]` triggers 5-hit combo (4×100 breaking/stun hits + 160 breaking+knockback finisher). **P2**: anim `0x2C` vs `0x2B`; spark `breaking` vs `heavy`; extra `IMPACT_FX`; SFX `0x7E` vs `0x7F` (combo uses `0x7F`+`0x75`). | Action `0x080B25F2` (fam 26 sub 2); Base `0x080B27FE`->Atk #7 (`r4=0x07020901`, `hit_mod=4`); Cmd `0x080B26B4`->`0x080B28EE..0x080B2AD2` (4× `0x1B020901`/`r7=0x1000` + `0x08020901`/`r7=4`); MegaMan `0x2B` (base), `0x2C`/`0x39`/`0x1E` (cmd); Arm `0c-12` / `0c-00` anim 1. |
| **DashAtk** | `0x035` | `chips/punch/init.luau` | **P0** | **No 37t windup, MegaMan stays hittable mid-dash (collision leak), wrong movement/pierce**: ROM winds up 30t (`0x28`) + 7t (`0x29`), turns MegaMan intangible (`col=0`, `status|=0xA0`) flying smoothly at 10 px/t (4t/panel) over holes, checks `0x08000000` obstacle stop, spawns `col_self=0x01` (not piercing) Atk #7 per panel, warps back with `0x0A` + `SetPlayerEndlag(8)`; Luau dashes on tick 3, teleports every 3t while hittable, uses `piercing`. **P2**: missing **`0c-1e` anim 0/1/2** attachment; anim `0x39` vs `0x28/0x29/0x2A/0x0A`; SFX `0x74` vs `0xB4`+`0x94`; extra `DASH_FX`; spark `impact` vs `heavy`. | Action `0x080B2F62` (fam 33 sub 0); Atk #7 `0x080D684C` (`r4=0x01020901`, `hit_mod=3`); MegaMan `0x28` (30t)->`0x29` (7t)->`0x2A` (dash)->`0x0A` (return); Attachment `0c-1e` anim `0/1/2` pal 0; Spark `14-05` anim 8 (`0x09`); SFX `0xB4` windup, `0x94` (`burst`) launch. |
| **Hammer** | `0x040` | `chips/punch/init.luau` | **P1** | **Invented panel cracking & forced `obstacle.remove` + wrong timing**: ROM does NOT crack panels and does NOT forcibly delete obstacles (uses `col_self=0x1B` breaking + 100 dmg); strikes tick 16 of `0x24` (32t) + retracts `0x25` (5t) = 37t total (Luau: tick 6 / 20t total). **P2**: spark `breaking` vs `heavy`; extra `IMPACT_FX`; always plays `0x7E` instead of `0x7E` (occupied) / `0xCD` (empty); never switches `0c-13` to anim 1; extra camera shake. | Action `0x080B31BA` (fam 34 sub 0); Atk #7 `0x080D684C` (`r4=0x1B020901`, `hit_mod=3`, `z=0`); MegaMan `0x24` (32t, strike tick 16)->`0x25` (5t); Arm `0c-13` anim `0`->`1` pal 0; Spark `14-05` anim 8 (`0x09`); SFX `0x7E` (`flags & 0x3E000000`) / `0xCD` (empty). |
| **ShockWav** | `0x02C` | `chips/wave/init.luau` | **P1** | **1-tick hitbox per panel + wrong step speed**: ROM Atk #8 steps every **15t** (`0x000F`, Luau `12t`) and keeps each panel's hitbox active for the full 28t anim until hit (overlapping panels); Luau only checks hit on `s.tick==0` (1t); missing `SetPlayerEndlag(0x3C)`. **P2**: sprite `0c-32` anim `1` vs **`10-00` anim `0`**; flip `alliance` vs `alliance^1`; SFX `0x7B` vs **`0x7A`**; extra `IMPACT_FX`; spark `plain` vs `14-05` anim 12. | Action `0x080B1968` (fam 10 sub 0); Atk #8 `0x080D68E4` (`0x080D6A7A`, table `0x080D68A0[0]`: `step=15t`, `col_self=0x01`, `hit_mod=3`); MegaMan `0x24`->`0x25` + `0c-07` anim 0->1; Wave sprite `10-00` anim 0 (28t); Spark `14-05` anim 12 (`0x02`); SFX `0x7A`. |
| **SonicWav** | `0x02D` | `chips/wave/init.luau` | **P1** | Same 1-tick hitbox defect + wrong step speed: ROM **10t** (`0x000A`) vs Luau `9t`; missing `SetPlayerEndlag(0x3C)`. **P2**: sprite `0c-32` anim `1` vs **`10-00` anim `1`** (21t); flip; SFX `0x7B` vs `0x7A`; extra `IMPACT_FX`; spark `plain` vs `14-05` anim 12. | Action `0x080B1968` (fam 10 sub 1); Atk #8 `0x080D68E4` (`0x080D68A0[1]`: `step=10t`); Wave sprite `10-00` anim 1 (21t); Spark `14-05` anim 12; SFX `0x7A`. |
| **DynaWave** | `0x02E` | `chips/wave/init.luau` | **P1** | Same 1-tick hitbox defect (step speed `6t` matches ROM `0x0006`); missing `SetPlayerEndlag(0x3C)`. **P2**: sprite `0c-32` anim `1` vs **`10-00` anim `2`** (14t); flip; SFX `0x7B` vs `0x7A`; extra `IMPACT_FX`; spark `plain` vs `14-05` anim 12. | Action `0x080B1968` (fam 10 sub 2); Atk #8 `0x080D68E4` (`0x080D68A0[2]`: `step=6t`); Wave sprite `10-00` anim 2 (14t); Spark `14-05` anim 12; SFX `0x7A`. |
| **BigWave** | `0x071` | `chips/wave/init.luau` | **P0** | **Completely wrong geometry (3 rows vs 1 row) & missing panel cracking**: ROM is a **1-row** shockwave at **4t/panel** (`0x0004`, Luau `6t`) that **cracks/breaks every panel** (`0x0800B568`) with `col_self=0x09` / `col_target=0x0A` and 8t persistent hitbox; Luau spawns 3 non-cracking rows of `collision.attack` with 1t hitboxes. **P2**: sprite `0c-32` anim `1` vs **`10-00` anim `3`** (8t); SFX `0x7B` vs `0x7A`; extra `IMPACT_FX`; extra camera shake; wrong spark. | Action `0x080B1968` (fam 10 sub 3, `r7=1` @ `0x080B1A07`); Atk #8 `0x080D68E4` (`0x080D68A0[3]`: `step=4t`, `0x080D693C`->`0x0800B568`, `col_self=0x09`, `col_target=0x0A`, `hit_mod=3`); Wave sprite `10-00` anim 3 (8t); Spark `14-05` anim 12; SFX `0x7A`. |
| **Wave** | `0x03D` | `chips/wave/init.luau` | **P1** | **Pierces instead of stopping on hit**, ~3× too fast (7t vs ROM **20t/panel**), 1t hitbox vs ROM 25t persistent hitbox, fires tick 6 / 20t total vs ROM tick 11 / 34t + `SetPlayerEndlag(0x50)`. **P2**: sprite `0c-32` anim `1` vs **`10-3B` anim 0 pal 0**; extra `AQUA_FX`; SFX `0x74` vs **`0xB0` cast + `0xB3` per-panel**. | Action `0x080B3F14` (fam 51 sub 0, elem `3`=Aqua); 3× Atk #0x6B `0x080E35DA` (`0x080E3782`, `step=0x14`=20t, stops if `[col+1]==0`, `col_self=0x01`, `col_target=0x03`, `hit_mod=3`); MegaMan `0x23` (34t, fire tick 11); Sprite `10-3B` anim 0 pal 0 (25t); Spark `14-05` anim 3 (`0x05`); SFX `0xB0` + `0xB3`. |
| **RedWave** | `0x03E` | `chips/wave/init.luau` | **P1** | **Invented lava-panel conversion** (`field.set_type(..., "lava")` does NOT exist in ROM!) + stops on hit (no pierce), 20t/panel (not 7t), 25t hitbox (not 1t), 11t/34t cast timing. **P2**: sprite `0c-32` vs **`10-3B` anim 0 pal 1**; extra `FIRE_FX`; SFX `0x76` vs `0xB0`+`0xB3`. | Action `0x080B3F14` (fam 51 sub 1, elem `2`=Heat); 3× Atk #0x6B `0x080E35DA` (`step=20t`, `hit_mod=3`, NO panel change); Sprite `10-3B` anim 0 pal 1 (25t); Spark `14-05` anim 2 (`0x04`); SFX `0xB0` + `0xB3`. |
| **MudWave** | `0x03F` | `chips/wave/init.luau` | **P1** | **Invented panel breaking** (`field.break_panel` does NOT exist in ROM!) + **`hit_mod=3` vs ROM `hit_mod=1`** (flinch without mercy flash, `0x080E368E..0x080E3694`) + stops on hit, 20t/panel, 25t hitbox, 11t/34t timing. **P2**: sprite `0c-32` vs **`10-3B` anim 0 pal 3**; extra `WOOD_FX`; SFX `0x7B` vs `0xB0`+`0xB3`. | Action `0x080B3F14` (fam 51 sub 2, elem `4`=Wood); 3× Atk #0x6B `0x080E35DA` (`step=20t`, `hit_mod=1`, NO panel break); Sprite `10-3B` anim 0 pal 3 (25t); Spark `14-05` anim 5 (`0x06`); SFX `0xB0` + `0xB3`. |
| **PanlOut1** | `0x077` | `chips/panlout/init.luau` | **P1** | **`hit_mod=0` vs ROM `hit_mod=1`** (flinch without flash, `0x080B2374`); missing `SetPlayerEndlag(0x0A)` (10t). **P2**: `hit_effect=nil` vs ROM spark `0x09` (`sparks.heavy`). *(Anim 32t+5t, strike tick 16, `0c-07`, `small-puff` anim 1, SFX `0x7B` match ROM.)* | Action `0x080B22DC` (fam 22 sub 0); Atk #7 `0x080D684C` (`r4=0x090A0901`, `r7=1`); Break `0x0800B610`; MegaMan `0x24`->`0x25` + `0c-07` anim 0->1; Break FX `14-02` anim 1 (`0x15`); Spark `14-05` anim 8 (`0x09`); SFX `0x7B`. |
| **PanlOut3** | `0x078` | `chips/panlout/init.luau` | **P1** | **Missing center-panel solid precondition**: ROM `0x080B2344..0x080B234E` aborts ALL 3 panels if `(x+dir, y)` lacks `0x10` (solid); plus `hit_mod=0` vs ROM `1`, missing `SetPlayerEndlag(0x0A)`. **P2**: `hit_effect=nil` vs `sparks.heavy`. | Action `0x080B22DC` (fam 22 sub 1); Precondition `GetPanelFlags(x+dir, y) & 0x10` (`0x080B2344`); 3× Atk #7 (`r4=0x090A0901`, `r7=1`) + `0x0800B610`; Break FX `14-02` anim 1; Spark `14-05` anim 8; SFX `0x7B`. |

---

#### Detailed Findings by Module

##### Module 1: Cannon (`chips/cannon/init.luau`, `lib/cannon.luau`, `lib/projectile.luau`)

**ROM Anchors**: PlayerAction family 4 `0x080B11E4`..`0x080B12D3`; AttackObj #1 `0x080D5D28` (`0x080D5E28` spawner, variant table `0x080D5CE4` entry `0x00`); SimpleEffect #0 `0x080EDD90` (table `0x080EDB88` entry `0x08`).

- **P1 — Projectile flies at half speed** (`lib/projectile.luau:26-28`, `objects/projectile/init.luau:9`, `@exelib/projectile/projectile.luau:32,125-129`):
  - ROM `0x080D5D5E` initializes `[obj+0x20] = 1`; on every update tick (`0x080D5DCC`), `sub r0, #1; bne ...` reaches `0` immediately, resets `[obj+0x20] = 1`, and advances `panel_x += dir` **1 panel per tick**.
  - `@exelib/projectile/projectile.luau` uses `WAIT = 1` with `left = me.timer - 1; if left >= 0 then return`, pausing an extra tick on every panel (**1 panel every 2 ticks**, EXE5/6 speed).
- **P1 — Action frame timing offset** (`@exelib/cannon.luau:31-32,93-138`):
  - ROM `0x080B1250` sets `[player+0x22] = 0x0F`, decrements starting the next tick (`0x080B1262`), firing on **tick 15**, then waits for `00-00` anim `0x20` (27t total) to set flag `0x80` (`0x080B12AA`), then plays `0x21` (5t) -> **32 ticks total**.
  - `@exelib/cannon.luau` returns early on tick 0, fires at `s.timer == 0xF` (**tick 16**), lowers at `t == 0x1D` (**tick 30**), and exits after `LOWER_TICKS = 3` (4 ticks) -> **34 ticks total**.
- **P1 (minor) — `hit_mod` value vs non-player targets** (`chips/cannon/init.luau:18`):
  - ROM `0x080D5CE4[0]` specifies `hit_mod = 2`. In BN3's lookup tables (`0x080AF3B0` enemy `[0,1,0,3,5,7]`, `0x080AF3B6` player `[0,1,3,3,5,7]`), `hit_mod = 2` causes flinch+flash vs players (`3`) but no flinch/flash vs viruses (`0`). Luau hardcodes `hit_mod = 3` (accurate in PvP, diverges vs viruses).
- **P2 — Wrong firing SFX ID** (`lib/cannon.luau:36`, `compat/assets.toml:124,131`):
  - ROM `0x080B1298` calls `PlaySoundEffect(0x74)`. `lib/cannon.luau:36` plays `asset.sound("cannon")` = `0x7D` (which is Spreader's fire SFX at `0x080B1554`).
- **P2 — Missing cannon-raise flash effect** (`lib/cannon.luau:35-44`):
  - ROM `0x080B1242..0x080B124C` spawns SimpleEffect `0x08` (`0c-01` anim 1, 28t) at MegaMan's coordinates with `side << 15` flip when raising the cannon. Not spawned in Luau.

---

##### Module 2: AirShot (`chips/airshot/init.luau`, `lib/airshot.luau`)

**ROM Anchors**: PlayerAction family 25 `0x080B2558`..`0x080B25F1`; AttackObj #1 `0x080D5D28` variant `0x0C` (`0x080D5D14`: `[0x00, 0x0B, 0x04, 0x07]`); `00-00` anim `0x38` (10t: `f0:1t, f1:1t(0x01), f2:2t, f3:3t, f4:2t, f5:1t(0x80)`).

- **P1 — Instant hitscan instead of traveling projectile** (`chips/airshot/init.luau:52-99`):
  - ROM `0x080B25BE..0x080B25D6` spawns AttackObj #1 (`0x080D5E28`, `r4 = 0x0C`, `z = 0x18_0000`) at `(me.panel_x + dir, me.panel_y)`, traveling **1 panel/tick** until hitting a valid target or leaving the field.
  - Current Luau scans `field.all_objects()` and `battle.alive_actors()` instantaneously on tick 5. Moving targets or targets stepping into/out of the row behave differently; scanning `field.all_objects()` also matches non-hittable objects (attachments, visual markers) blocking the shot!
- **P1 — Wrong element (`wind` secondary added)** (`chips/airshot/init.luau:15,84`):
  - ROM `0x080B25D4` passes `r0 = 0` (**Null**, no Wind element; AirShot does not strip Auras/Barriers in BN3). Luau sets `WIND_ELEM = element.byte("null", { "wind" })`.
- **P1 — Wrong `hit_mod` and knockback implementation** (`chips/airshot/init.luau:19-34,91,95-97`):
  - ROM `0x080D5D14` sets `hit_mod = 4` (flinch + heavy stagger ignoring SuperArmor, no mercy flash; `table[1][4] = 5`) and `col_self = 0x07` (`0x80000100`/`0x40000200` engine-native 1-panel knockback, which also interacts with obstacles/RockCubes and respects invincibility/guard).
  - Luau uses `hit_mod = 1`, manually teleports `opp.panel_x` even if the hitbox didn't connect (e.g., against Invis/flashing/guarded targets), and on miss spawns a hitbox + wind puff on `me.panel_x + me.facing`.
- **P1 — Action timing doubled** (`chips/airshot/init.luau:52,101,121,135,149`):
  - ROM fires at **tick 2** (`00-00` anim `0x38` frame 1 flag `0x01`) and ends at **tick 10** (`0x080B25E2`, no extra recovery). Luau fires at **tick 5** and ends at **tick 20** (`10 + recovery=10`).
- **P2 — Presentation mismatches** (`chips/airshot/init.luau:13,46,85,87,94`):
  - Spark: ROM `0x080D5D15 = 0x0B` -> `14-01` anim 0 (`sparks.impact`); Luau uses `sparks.plain` (`14-05` anim 0) and spawns an invented `small-puff` (`14-02` anim 1) effect.
  - Z-height: ROM `0x18_0000` (`0x080B25C8`); Luau `0x14_0000`.
  - SFX: ROM plays `0x74` at fire time (tick 2, `0x080B25B8`); Luau plays it on init (tick 0).
- **Note**: `content/exe3/lib/airshot.luau` is unused dead code.

---

##### Module 3: Spreader Family (`chips/spreader/init.luau`, `chips/spreader/spreader.luau`)

**ROM Anchors**: PlayerAction family 6 `0x080B14D2`..`0x080B15B9`; AttackObj #2 `0x080D5E50` (`0x080D5FEA` spawner, `0x080D5FC0` burst collision reset, regions `0x080D5E4C` = `[0x02, 0x0D, 0x04, 0x0F]`, burst effects `0x080D5FAC` = `[0x0E, 0x0F, 0x10]`, burst SFX `0x080D5FAF` = `[0x00, 0xAE, 0xA9]`).

- **P1 — `Bubbler`/`Bub-V`/`BublSide` & `HeatShot`/`Heat-V`/`HeatSide` have wrong `hit_mod`** (`chips/spreader/init.luau:26-32`):
  - ROM `0x080D5FC0..0x080D5FE4` sets `mov r3, #0x0` (**`hit_mod = 0` — no flinch, no mercy invincibility**) for **ALL 10** chips in Family 6 (`r7 = 0` at `0x080B1558`).
  - Luau sets `hit_mod = 3` for all 6 Aqua/Heat variants, granting flinch + 60t mercy invincibility and breaking authentic BN3 PvP combo behavior.
- **P1 — Recovery duration too long + missing endlag** (`@exelib/spreadr/action.luau:28,64-76`):
  - ROM `0x080B159A` sets `[player+0x22] = 0x0F` (**15 ticks** recovery), then calls `SetPlayerEndlag(0x0F)` (`0x080B15AE`).
  - `@exelib/spreadr/action.luau` uses `RECOVER_TICKS = 0x14` (**21 ticks** — 6 ticks slower).
- **P2 — Wrong burst SFX for all 10 chips** (`chips/spreader/init.luau:19-32`):
  - ROM `0x080D5F78..0x080D5F82` reads burst SFX from `0x080D5FAF[sub >> 2]`:
    - `ShotGun`/`V-Gun`/`SideGun`/`Spreader` (`sub 0..3`): **`0x00` (NO burst SFX)** (Luau plays `0x74`).
    - `Bubbler`/`Bub-V`/`BublSide` (`sub 4..6`): **`0xAE` (`freeze`)** (Luau plays `0x74`).
    - `HeatShot`/`Heat-V`/`HeatSide` (`sub 8..10`): **`0xA9`** (missing from `assets.toml`; Luau plays `0x74`).
  - *(Firing SFX `0x7D` at `spreader.luau:16`, muzzle flash `0c-1c` anim 0, burst effects `14-04`/`0c-16`/`0c-15`, and regions `long`/`fork`/`wide`/`square` all match ROM!)*

---

##### Module 4: Bombs (`chips/bomb/init.luau`, `lib/bombs/init.luau`)

**ROM Anchors**: PlayerAction family 7 `0x080B15C0`..`0x080B16CD` (throw count table `0x080B1604` = `[1, 1, 2, 3, 1, 1, 1]`); AttackObj #3 `0x080D600C` (`0x080D61D0` spawner, blast regions `0x080D61C0` = `[0x01, 0x04, 0x04, 0x04]`); AttackObj #5 `0x080D6394` (`0x080D65B2` spawner, CannBall landing `0x080D6550` -> `0x0800B64C`); Effect #5 `0x080EE1EC` (`0x080EE26E` hole splash).

- **P0 — `DublBomb` (0x016) & `TrplBomb` (0x017): Missing multi-throw loop & wrong blast region** (`chips/bomb/init.luau:23-24,55-81`):
  - ROM `0x080B15F0..0x080B160A` + `0x080B1684..0x080B1698` loops the throw animation **`2` times for `DublBomb`** (2×50 = 100 dmg) and **`3` times for `TrplBomb`** (3×50 = 150 dmg), releasing a separate Bomb #3 each throw.
  - ROM `0x080D61C0` sets blast region **`0x04` (`regions.wide`, 3 vertical panels)** for `SnglBomb`, `DublBomb`, AND `TrplBomb` (only `MiniBomb` uses `0x01` `single`).
  - Current Luau throws only **1 bomb** for `DublBomb` (`regions.long`) and **1 bomb** for `TrplBomb` (`regions.line`).
- **P0 — `CannBall` (0x018): Wrong attack object, missing panel break/crack, wrong sprite/FX/SFX** (`chips/bomb/init.luau:25,83-95`):
  - ROM `0x080B1624..0x080B1662` skips Bomb #3 completely (no held `0c-02` bomb in hand) and spawns **AttackObj #5** (`0x080D65B2` -> `0x080D6394`), sprite **`10-2D` anim 0 pal 0**, flying along a ballistic arc (`horiz_speed = 0x0002C000`, `grav = -0x2800` via `0x08001008`) from `(me.x - 4*dir, me.y, me.z + 0x30)` to `(me.panel_x + 3*dir, me.panel_y)` with `col_self = 0x0B` (`thrown_break`), `hit_mod = 3`.
  - On landing on a solid panel (`0x080D6550`), CannBall calls **`0x0800B64C`** (breaks panel if unoccupied, cracks if occupied), plays SFX **`0x7B` (`panel-crack`)**, and spawns SimpleEffect **`0x15` (`14-02` anim 1 `small-puff`)** on the next tick.
  - Current Luau reuses Bomb #3 (`0c-02` bomb sprite, held attachment, `explosion` `14-00`, `hit-bomb` `0x76` SFX) and **never cracks or breaks the target panel**.
- **P1 — `SnglBomb` (0x015) wrong blast region** (`chips/bomb/init.luau:22`):
  - Uses `regions.single` instead of ROM `0x04` (`regions.wide`, 3 vertical panels at `0x080D61C0[1]`).
- **P1 — `MiniBomb`/`SnglBomb`/`DublBomb`/`TrplBomb` wrong `hit_mod`, `col_self`, and timing** (`chips/bomb/init.luau:11,14`, `@exelib/bombs/throw.luau:29-30,81-96`):
  - ROM `0x080D6078` sets `col_self = 0x01` (`attack`) and **`hit_mod = 0`** (no flinch / no mercy flash — essential so DublBomb/TrplBomb multi-hit!). Luau sets `collision.thrown` and `hit_mod = 3`.
  - ROM `0x080B16C4` plays throw SFX `0x77` at tick 7 (`0x07`), each throw lasts 22t (`00-00` anim `0x22`), and exits with `SetPlayerEndlag(0x3C)` (60t). `@exelib/bombs/throw.luau` plays SFX at tick 0 and lasts 27t (`21 + 6`).
- **P2 — Bomb palette & hole splash anim** (`chips/bomb/init.luau:21-24`, `lib/bombs/init.luau:23`):
  - ROM AttackObj #3 never changes palette (`0` for all 4 bombs); Luau assigns palettes `0, 1, 2, 3`.
  - ROM hole splash Effect #5 (`0x080EE20C`) uses `14-02` anim **`1`** rising at `z_vel = +2 px/t`; Luau uses `small-puff` anim `0` stationary.

---

##### Module 5: Punch, Hammer, DashAtk & AirSwrd (`chips/punch/init.luau`)

**ROM Anchors**:
- `AirSwrd`: Family 8 `0x080B16CE` (sub 8); AttackObj #6 `0x080D6664` (`0x080D65CC[8]`); AttackObj #0x1E `0x080D9078` (`0x080D926C`).
- `GutPunch`/`GutStrgt`/`GutImpct`: Family 26 `0x080B25F2`..`0x080B2AD3`; AttackObj #0x1A `0x080D89D8` (`0x080D8ADE`, table `0x080D89C8`); AttackObj #7 `0x080D684C`.
- `DashAtk`: Family 33 `0x080B2F62`..`0x080B31B9`; AttackObj #7 `0x080D684C`.
- `Hammer`: Family 34 `0x080B31BA`..`0x080B3299`; AttackObj #7 `0x080D684C`.

- **P0 — `AirSwrd` (0x02A): Missing 3 full-row Wind gust projectiles (`AttackObj #0x1E`)** (`chips/punch/init.luau:199-243`):
  - ROM `0x080D65CC[8]` fires a wide 3-panel slash (`0c-0c` anim 0, `regions.wide`, 100 dmg, Null+Sword, **`hit_mod = 0`**, spark `0x02` = `14-05` anim 12) at tick 7 of MegaMan anim `0x1D` (22t -> `0x1F` 7t = 29t total, SFX `0x75` only).
  - Simultaneously (`0x080D6724..0x080D6750`), it spawns **3 invisible `AttackObj #0x1E` wind gusts** (`r4 = 0x80`, `damage = 0`, `element = 5` Wind, `col_self = 0x4A` = full-row push to back column, speed `10 px/t`) on `(x+dir, y+1)`, `(x+dir, y)`, `(x+dir, y-1)` that travel across the entire row, push any enemies to the back column, strip Barriers/Auras, and clear Sand panels (`0x080D9206`).
  - Luau gives the slash `hit_mod = 3` and `WIND_SWORD_ELEM`, only teleports enemies 1 panel at `x+dir` (ignoring columns further back), fires at tick 5 / exits tick 18, uses `sparks.plain`, and plays an extra `WIND_SOUND` (`0x74`).
- **P0 — `GutPunch` (0x02F), `GutStrgt` (0x030), `GutImpct` (0x031): Missing command codes + wrong base attacks** (`chips/punch/init.luau:65-133,245-285`):
  - **Missing Hold-A 48-tick Command Inputs (`0x080B2654..0x080B2762`, `0x080B269C..0x080B26E6`)**:
    - `GutPunch` (`Down, Down+Fwd, Fwd`): chimes `0x117`, plays `0x2C` + `0c-12`, fires **1** full-row breaking rocket fist (`AttackObj #0x1A` var 2: `speed=16 px/t`, 8 panels, `col_self=0x49` knockback+break, `hit_mod=4`, SFX `0xBD`).
    - `GutStrgt` (`B, rel B, B, Down, Back, Fwd, B`): chimes `0x117`, fires **3** full-row breaking rocket fists (`AttackObj #0x1A` var 2) 15 ticks apart (`0x080B2774[1] = 3`).
    - `GutImpct` (`Back, B, Down, B, Down, B`): chimes `0x117`, executes a **5-hit breaking combo** (`0x080B28EE..0x080B2AD2`): 4× 100-dmg breaking/stun hits (`0x1B020901`, `r7=0x1000`, cycling punch `0x2C`, dash `0x39`, sword `0x1E`+`0c-00` anim 1, punch `0x2C`) + 160-dmg breaking+knockback finisher (`0x08020901`, `hit_mod=4`, 32t recovery) = **560 total damage**.
  - **Wrong Base Attacks (no command)**:
    - `GutPunch` & `GutImpct` base (`0x080B27FE..0x080B2880`): MegaMan anim **`0x2B`** (30t, strike tick 4), SFX **`0x7F`**, `AttackObj #7` with `r4 = 0x07020901` (`col_self = 0x07` = **knockback 1 panel, NOT breaking**!), **`hit_mod = 4`** (heavy stagger, no flash), spark `0x09` (`sparks.heavy`), `z = 0x18`. `GutImpct` does **NOT** push 2 panels and does **NOT** crack panels!
    - `GutStrgt` base (`0x080B2892..0x080B28EC`): MegaMan anim `0x2C` (14t, strike tick 4), SFX `0x7F` + `0xBD`, spawns **flying fist `AttackObj #0x1A` var 0** (`0c-12` anim 1, `speed=8 px/t`, 2 panels, `col_self=0x07` knockback, `hit_mod=4`), NOT a stationary `regions.long` breaking hitbox!
- **P0 — `DashAtk` (0x035): Missing 37t windup, MegaMan hittable mid-dash, wrong movement/collision/visuals** (`chips/punch/init.luau:140-192`):
  - ROM `0x080B2F8C..0x080B31B8`: winds up 30t (`0x28` + `0c-1e` anim 0, SFX `0xB4`) + 7t (`0x29` + `0c-1e` anim 1) = **37t before launch**. On launch (`0x080B3000`): anim `0x2A` + `0c-1e` anim 2, SFX `0x94` (`burst`), clears own collision (`0x080B3046`, **invulnerable/intangible** during dash), flies smoothly at `10 px/t` (`4t/panel`, `2t` first step) over holes, aborts early if `GetPanelFlags(x+dir, y) & 0x08000000` (obstacle stop), spawns `AttackObj #7` (`r4=0x01020901` normal `attack`, `hit_mod=3`, `sparks.heavy`) per panel, then returns to `(orig_x, orig_y)` with anim **`0x0A`** (`MOVE_ARRIVE`), restores collision, and sets `SetPlayerEndlag(8)`.
  - Current Luau dashes on tick 3 (no windup), leaves MegaMan's own collision active on enemy panels while dashing, teleports panel-by-panel every 3t, uses `collision.piercing`, completely omits `0c-1e` attachment, uses wrong MegaMan anim `0x39`, wrong SFX `0x74`, extra `DASH_FX`, and `sparks.impact`.
- **P1 — `Hammer` (0x040): Invented panel cracking, forced obstacle deletion, wrong timing & SFX** (`chips/punch/init.luau:87-130,287-299`):
  - ROM `0x080B31BA..0x080B3298`: anim `0x24` (32t, strike tick 16) + `0c-13` anim 0 -> anim `0x25` (5t) + `0c-13` anim 1 = **37t total** (Luau: strike tick 6 / 20t total, never switches `0c-13` to anim 1).
  - At strike (`0x080B3210`): checks `GetPanelFlags(x+dir, y) & 0x3E000000` -> plays SFX **`0x7E`** if occupied else **`0xCD`** (empty thud); spawns `AttackObj #7` (`r4=0x1B020901` breaking, `hit_mod=3`, `sparks.heavy`, `z=0`). Does **NOT crack panels** and does **NOT forcibly delete obstacles** (`obstacle.remove`) regardless of HP. Luau also spawns extra `IMPACT_FX` and shakes camera.

---

##### Module 6: Waves (`chips/wave/init.luau`)

**ROM Anchors**:
- `ShockWav`/`SonicWav`/`DynaWave`/`BigWave`: Family 10 `0x080B1968`..`0x080B1A4B` (`0x080B1A04` = `[0, 0, 0, 1]`); AttackObj #8 `0x080D68E4` (`0x080D6A7A`, table `0x080D68A0`: `[0x000F, 0x000A, 0x0006, 0x0004]`).
- `Wave`/`RedWave`/`MudWave`: Family 51 `0x080B3F14`..`0x080B3FAB` (`0x080B3F90` = `[3, 2, 4]`); AttackObj #0x6B `0x080E35DA` (`0x080E3782`, table `0x080E35D8` = `spr=10-3B, step=0x14`, palettes `0x080E36C4`).

- **P0 — `BigWave` (0x071): Completely wrong geometry & missing panel cracking** (`chips/wave/init.luau:223-235`):
  - ROM `0x080B19C0..0x080B1A06` spawns **1 single-row wave** on `me.panel_y` with `r4 = 3` (`step_ticks = 4`, `10-00` anim 3 = 8t) and `r7 = 1`, which in `AttackObj #8` (`0x080D6932..0x080D693C`, `0x080D699C..0x080D69A6`) **cracks/breaks every panel it traverses** (`0x0800B568`) and uses `col_self = 0x09` / `col_target = 0x0A` (`hit_mod = 3`).
  - Current Luau sets `all_rows = true` (spawns **3 rows**!), `step_ticks = 6`, `collision.attack`, and **never cracks panels**.
- **P1 — `ShockWav`/`SonicWav`/`DynaWave`/`BigWave`: 1-tick hitbox per panel & wrong speeds** (`chips/wave/init.luau:58-107,181-235`):
  - ROM spawns a new `AttackObj #8` per panel whose hitbox remains **persistently active** for its entire animation (`28t / 21t / 14t / 8t`) until it hits something (`0x080D69D8`), while spawning the next panel after `step_ticks` (**15t / 10t / 6t / 4t**). Luau moves one crawler emitting a **1-tick** `battle.hitbox` on `s.tick == 0` (leaving panels completely safe on ticks `1..step_ticks-1`!) and uses speeds `12t / 9t / 6t / 6t`. Missing `SetPlayerEndlag(0x3C)` (`0x080B1A40`).
  - **P2**: Wrong sprite `0c-32` anim 1 instead of **`10-00` anim `0/1/2/3`** (missing from `assets.toml`), wrong flip (`alliance` vs `alliance ^ 1`), wrong SFX `0x7B` instead of **`0x7A`**, extra `IMPACT_FX`, wrong spark `sparks.plain` vs `14-05` anim 12 (`0x02`).
- **P1 — `Wave`/`RedWave`/`MudWave`: Invented panel effects, wrong pierce/speed/hitbox/`hit_mod`/timing** (`chips/wave/init.luau:58-107,134-179,237-277`):
  - **Invented panel changes**: ROM `0x080E35DA..0x080E37AE` has **NO lava conversion** on `RedWave` (Luau lines 85-86) and **NO panel breaking** on `MudWave` (Luau lines 88-89)!
  - **Stops on hit (no pierce)**: ROM `0x080E36DA` clears `[col+1]` on hit and `0x080E373C` skips spawning the next panel if `[col+1] == 0`. Luau uses `collision.piercing` and always traverses the entire row.
  - **`MudWave` `hit_mod`**: ROM `0x080E368E..0x080E3694` sets **`hit_mod = 1`** (flinch, no mercy flash) for Wood element (`MudWave`), vs `3` for `Wave`/`RedWave`. Luau sets `hit_mod = 3` for all three.
  - **Speed & hitbox persistence**: ROM steps every **20t** (`0x14`, `0x080E35D8`) with a **25t** persistent hitbox per panel (`10-3B` anim 0); Luau steps every **7t** with a **1t** hitbox.
  - **Cast timing**: ROM `00-00` anim `0x23` (34t) fires on **tick 11** (`0x080B3F50`) and ends at **tick 34** + `SetPlayerEndlag(0x50)`; Luau fires on tick 6 and ends on tick 20.
  - **P2**: Wrong sprite `0c-32` anim 1 instead of **`10-3B` anim 0** with palettes `0` (Wave), `1` (RedWave), `3` (MudWave) (`0x080E36C4`, missing from `assets.toml`); extra `AQUA_FX`/`FIRE_FX`/`WOOD_FX`; wrong SFX (`0x74`/`0x76`/`0x7B`) instead of **`0xB0` cast** (`0x080B3F80`) + **`0xB3` per-panel** (`0x080E3664`).

---

##### Module 7: PanlOut (`chips/panlout/init.luau`, `lib/crack.luau`)

**ROM Anchors**: PlayerAction family 22 `0x080B22DC`..`0x080B23FB`; AttackObj #7 `0x080D684C`; Panel break `0x0800B610`.

- **P1 — `PanlOut3` missing center-panel solid precondition** (`lib/crack.luau:61-83`):
  - ROM `0x080B2344..0x080B234E` checks `GetPanelFlags(me.panel_x + dir, me.panel_y) & 0x10` (center panel directly ahead is solid) BEFORE entering the loop. If the center panel ahead is already missing/broken, `PanlOut3` skips ALL 3 panels (does not break `(x+dir, y-1)` or `(x+dir, y+1)`). Current `lib/crack.luau` checks each panel independently.
- **P1 — Wrong `hit_mod` & missing endlag** (`lib/crack.luau:75,110-112`):
  - ROM `0x080B2374` passes `r7 = 1` (**`hit_mod = 1`** — flinch without mercy flash) to `AttackObj #7` and calls `SetPlayerEndlag(0x0A)` (10t) at `0x080B23F0`. Current `lib/crack.luau` sets `hit_mod = 0` (no flinch).
- **P2 — Missing hit spark** (`lib/crack.luau:70`):
  - ROM `0x080B236C` passes `r4 = 0x090A0901` (spark `0x09` = `14-05` anim 8 = **`sparks.heavy`**). Current `lib/crack.luau` sets `hit_effect = nil`.
  - *(Animation timing `0x24` 32t strike at tick 16 -> `0x25` 5t, `0c-07` anim 0->1, `small-puff` `14-02` anim 1 on break, and `panel-crack` `0x7B` SFX when `broke_any` all match ROM accurately.)*


---

### 5.2 Group 2：剑系、电击/火焰/龙卷、悠悠/回旋镖、传感器与麦格农（50 张）

- **Target ROM**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US — `A6BE`)
- **Codebase Audited**: `/home/harp/projects/saito` (read-only audit)
- **Modules Covered (50 chips total)**:
  1. `content/exe3/chips/sword/init.luau` + `varswrd.luau` + `content/exe3/lib/swords.luau` (14 chips)
  2. `content/exe3/chips/zapring/init.luau` (8 chips)
  3. `content/exe3/chips/yoyo/init.luau` (7 chips)
  4. `content/exe3/chips/elec/init.luau` (6 chips)
  5. `content/exe3/chips/magnum/init.luau` (15 chips)

> **Scope note**: `burning`, `fireratn`, `condor`, `geyser`, and `shake1..3` are not defined in these 5 modules; the 50 chips below represent 100% of all chips defined in the assigned files.

---

#### Executive Summary

| Severity | Count | Chips |
| :--- | :---: | :--- |
| **P0** | **20** | `varswrd`, `rope1..3`, `burner`, `sensor1..3`, `magnum1..3`, `nobeam1..3`, `icewave1..3`, `arrow1..3` |
| **P1** | **22** | `bambswrd`, `stepswrd`, `stepcros`, `slasher`, `muramasa`, `zapring1..3`, `tornado`, `yo-yo1..3`, `boomer1..3`, `lance`, `plasma1..3`, `ratton1..3` |
| **P2** | **8** | `sword`, `wideswrd`, `longswrd`, `fireswrd`, `aquaswrd`, `elecswrd`, `custswrd`, `heroswrd` |
| **OK** | **0** | *(Note: `sword`, `wideswrd`, `longswrd`, `custswrd`, `heroswrd` are behaviorally OK; flagged P2 solely for minor presentation/SFX/z-offset diffs)* |
| **Total** | **50** | |

##### Key Shared ROM Tables Verified (`bn3.gba` A6BE)
- **PlayerActionJumpTable**: `0x080B0DA4` (`family * 4`)
- **DimmingEffectJumpTable**: `0x08008D00` (`effect_id * 4`)
- **ActorObjJumpTable (Type 1)**: `0x080036A8`
- **AttackObjJumpTable (Type 3)**: `0x080038C4`
- **EffectObjJumpTable (Type 4)**: `0x08003C18` (SimpleEffect table at `0x080EDB88`, stride 4: `[cat, idx, anim, pal]`)
- **Region Table**: `0x080AF478` (`0x01`=1x1, `0x02`=2x1, `0x04`=1x3, `0x06`=3x1, `0x09`=Plus, `0x0B`=X, `0x10`=8-ring, `0x11`=2x3, `0x12`=rising diag `[(0,0),(1,-1),(-1,1)]`, `0x13`=falling diag `[(0,0),(-1,-1),(1,1)]`)
- **Collision Table**: `0x080AF51C` (`0x01/0x02`=attack, `0x12/0x13`=break, `0x18/0x19`=pierce+break, `0x1B`=wind, `0x26/0x27`=sword/slash, `0x2F/0x36/0x38`=destructible field object, `0x4B`=knockback/push)
- **EXE3 Elements in ROM**: `0=Null, 1=Elec, 2=Fire, 3=Aqua, 4=Wood` *(Note: Saito uses `0=null, 1=fire, 2=aqua, 3=elec, 4=wood` in `content/exe3/rules/elements.luau`; chip element semantics match Saito's internal table unless explicitly noted below.)*
- **Panel Helpers**: `0x0800B610` breaks unoccupied panels directly to broken (`kind=1`, skips occupied panels `0x3E0A0080`); `0x0800BF4A` checks solid panel (`flags & 0x10`).
- **Asset Mislabelling Discovered (`content/exe3/compat/assets.toml`)**:
  - `nobeam = "0c-35"` is actually **StepCros's X-slash** (`0x080D65CC[16]`); actual **NoBeam** sprite is **`0c-36`** (`0x080E72FC`, trail SimpleEffect `0x44` = `0c-36` anim 1).

---

#### Complete Per-Chip Audit Table (50 Chips)

##### 1. Swords (`content/exe3/chips/sword/init.luau`, `varswrd.luau`, `lib/swords.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `sword` | `0x01E` | `sword/init.luau` | **P2** | Behavior/dmg(80)/hitbox(1x1)/`hit_mod=3`/SFX match. Visual z-offset is `0x10_0000` (ROM `0x08_0000`) and misses `0x1F` sword recovery pose. | Fam 8 (`0x080B16CE`), AttackObj `0x06` (`0x080D6664`), Row 0 (`0x080D65CC`): `0c-0c` anim 2 pal 0, blade `0c-00`, SFX `0x75`, `hit_mod=3`. |
| `wideswrd` | `0x01F` | `sword/init.luau` | **P2** | Behavior/dmg(80)/hitbox(1x3)/`hit_mod=3`/SFX match. Visual z-offset `0x10_0000` vs ROM `0x08_0000`; misses `0x1F` recovery pose. | Fam 8 (`0x080B16CE`), Row 1 (`0x080D65D4`): `0c-0c` anim 0 pal 0, blade `0c-00`, reg `0x04`, SFX `0x75`, `hit_mod=3`. |
| `longswrd` | `0x020` | `sword/init.luau` | **P2** | Behavior/dmg(80)/hitbox(2x1)/`hit_mod=3`/SFX match. Visual z-offset `0x10_0000` vs ROM `0x08_0000`; misses `0x1F` recovery pose. | Fam 8 (`0x080B16CE`), Row 2 (`0x080D65DC`): `0c-0c` anim 1 pal 0, blade `0c-00`, reg `0x02`, SFX `0x75`, `hit_mod=3`. |
| `fireswrd` | `0x021` | `sword/init.luau` | **P2** | Behavior/dmg(130)/elem(fire)/hitbox(1x3)/`hit_mod=3`/blade/SFX match. Slash palette uses `5` (ROM uses `0`); z-offset `0x10` vs `0x08`; no `0x1F` pose. | Fam 8 (`0x080B16CE`), Row 3 (`0x080D65E4`): `0c-0c` anim 0 **pal 0**, blade `0c-08`, reg `0x04`, SFX `0x75`, `hit_mod=3`. |
| `aquaswrd` | `0x022` | `sword/init.luau` | **P2** | Behavior/dmg(150)/elem(aqua)/hitbox(1x3)/`hit_mod=3`/blade/SFX match. Slash palette uses `6` (ROM uses `0`); z-offset & `0x1F` pose diff. | Fam 8 (`0x080B16CE`), Row 4 (`0x080D65EC`): `0c-0c` anim 0 **pal 0**, blade `0c-09`, reg `0x04`, SFX `0x75`, `hit_mod=3`. |
| `elecswrd` | `0x023` | `sword/init.luau` | **P2** | Behavior/dmg(130)/elem(elec)/hitbox(1x3)/`hit_mod=3`/blade/SFX match. Slash palette uses `7` (ROM uses `0`); z-offset & `0x1F` pose diff. | Fam 8 (`0x080B16CE`), Row 5 (`0x080D65F4`): `0c-0c` anim 0 **pal 0**, blade `0c-0a`, reg `0x04`, SFX `0x75`, `hit_mod=3`. |
| `bambswrd` | `0x024` | `sword/init.luau` | **P1** | **`hit_mod = 3` (causes mercy invuln flash)** vs ROM **`hit_mod = 1` (no mercy flash)**! Also P2: slash palette `8` vs ROM `0`; z-offset & `0x1F` pose. | Fam 8 (`0x080B16CE`), Row 6 (`0x080D65FC`): `0c-0c` anim 0 pal 0, blade `0c-0b`, reg `0x04`, SFX `0x75`, **`hit_mod=1`**. |
| `custswrd` | `0x025` | `sword/init.luau` | **P2** | Gauge damage formula (`0x08010610`, `full=0`), 2x3 hitbox, `0c-0d` anim 0 pal 1, `hit_mod=3` all match! Wrong SFX `0xD6` (`big-sword-swing`) vs ROM `0x75` (`sword-swing`); z-offset & `0x1F`. | Fam 8 (`0x080B16CE`), Row 7 (`0x080D6604`), SFX table `0x080B17AC[7] = 0x75`, formula `0x08010610`. |
| `varswrd` | `0x026` | `sword/varswrd.luau` | **P0** | Missing 6th input (`ElemSonic`: `A+B,A,A+B,Back,Down,Up`); `seq[4]` (`SonicBoom`) hijacked to `ElemSonic` (`sonicbom_pick` dead code); PVP Side-1 input reversal (`me.flip` vs `me.alliance`); SonicBoom wrongly pierces (`pierces=true` vs ROM destroys on hit); ElemSonic order `[1,2,3,4]` vs ROM `[Aqua,Elec,Fire,Wood]`; wave swing/palettes/chime SFX wrong. | Fam 32 (`0x080B2D24`), seqs `0x080B2D8C..080B2DF4`, AttackObj `0x31` (`0x080DBD9C`), elem table `0x080B2F04`, pal table `0x080DBE58`, chime SFX `0x117` (`0x080B2E6A`). |
| `stepswrd` | `0x027` | `sword/init.luau` | **P1** | Warp check omits obstacle bit (`forbid=0x0F800000` vs ROM `0x38000000` incl. obstacles); instant 1-frame warp vs ROM 3-tick hidden/intangible transit + blocked `0x0B` pose. P2: uses EXE5/6 afterimages instead of SimpleEffects `0x04`/`0x05` (`14-09` anim 2/3). | Fam 41 sub 0 (`0x080B373C`), check `0x080B3790`, SE `0x04/0x05` (`14-09` anim 2/3), AttackObj `0x06` row 1 (`0c-0c` anim 0), SFX `0x75`. |
| `stepcros` | `0x028` | `sword/init.luau` | **P1** | Single unified X hitbox vs ROM two diagonal AttackObjs (`0x06` sub `0x10` reg `0x12` + `0x07` reg `0x13`); same warp check/transit diffs as StepSwrd. P2: wrong sprite `0c-0d` (`big-slash`) instead of **`0c-35`** anim 0 pal 0; wrong SFX `0xD6` instead of `0x75`. | Fam 41 sub 1 (`0x080B373C`), AttackObj `0x06` row `0x10` (`0x080D664C`, `0c-35`, reg `0x12`) + AttackObj `0x07` (`0x080D684C`, reg `0x13`), SFX `0x75`. |
| `slasher` | `0x02B` | `sword/init.luau` | **P1** | Triggers immediately on frame 0 without checking A held (`SLASHER_START`), lacks AttackObj `0x43` active sensor/interruption lifecycle. P2: wrong pose `0x1D` (ROM `0x1E -> 0x1F`), blade anim `0` (ROM `1`), SFX `0xD6` (ROM `0x75`), flip passes `me.alliance`. | Fam 40 (`0x080B365C`), intruder search `0x080B5870`, sensor AttackObj `0x43` (`0x080DE368`), AttackObj `0x06` row 1, pose `0x1E`/`0x1F`, blade `0c-00` anim 1, SFX `0x75`. |
| `muramasa` | `0x0CA` | `sword/init.luau` | **P1** | **Wrong hitbox range**: uses `regions.single` (**1x1**) instead of ROM `region 0x02` (**LongSwrd 2x1**)! Damage formula (`hp_lost`, cap 999) matches `0x080105F4`. P2: SFX `0xD6` vs ROM `0x75`. | Fam 8 (`0x080B16CE`), Row 9 (`0x080D6614`): `0c-0c` anim 3 pal 3, **reg `0x02` (2x1)**, SFX `0x75`, `hit_mod=3`, formula `0x080105F4`. |
| `heroswrd` | `0x0CF` | `sword/init.luau` | **P2** | Behavior/dmg(180)/hitbox(3x1)/`hit_mod=3`/`0c-0c` anim 4 pal 2 all match! Wrong SFX `0xD6` (`big-sword-swing`) vs ROM `0x75` (`sword-swing`); z-offset `0x10` vs `0x08` & `0x1F` pose. | Fam 8 (`0x080B16CE`), Row 10 (`0x080D661C`): `0c-0c` anim 4 pal 2, reg `0x06`, SFX `0x75` (`0x080B17AC[10]`), `hit_mod=3`. |

##### 2. ZapRing / Rope / Tornado / Burner (`content/exe3/chips/zapring/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `zapring1` | `0x042` | `zapring/init.luau` | **P1** | `hit_mod=3` vs ROM `hit_mod=0` (no mercy flash); fixed paralyze vs status `0x10`; discrete 5t/panel teleport vs smooth `10 px/t`; bypasses obstacles/shields. P2: wrong sprite `14-05` (ROM `10-03`), pose `buster_shoot` (ROM `0x36` + `0c-1b`), SFX `0x6E` (ROM `0xB0`), `z=0x10` vs `0x18`. | Fam 14 (`0x080B1C90`), AttackObj `0x0D` (`0x080D71A8`), speed `0x0A_0000` (`0x080D7190`), spr `10-03` anim 0, arm `0c-1b`, pose `0x36`, SFX `0xB0`, `hit_mod=0`, status `0x10`. |
| `zapring2` | `0x043` | `zapring/init.luau` | **P1** | Same as `zapring1`: `hit_mod=3` vs `0`, fixed paralyze vs status `0x11`, discrete 4t/panel vs smooth `14 px/t`, bypasses obstacles. Same P2 sprite/pose/arm/SFX/z diffs. | Fam 14 sub 1 (`0x080B1C90`), AttackObj `0x0D` (`0x080D71A8`), speed `0x0E_0000`, spr `10-03`, arm `0c-1b`, pose `0x36`, SFX `0xB0`, `hit_mod=0`, status `0x11`. |
| `zapring3` | `0x044` | `zapring/init.luau` | **P1** | Same as `zapring1`: `hit_mod=3` vs `0`, fixed paralyze vs status `0x12`, discrete 3t/panel vs smooth `18 px/t`, bypasses obstacles. Same P2 sprite/pose/arm/SFX/z diffs. | Fam 14 sub 2 (`0x080B1C90`), AttackObj `0x0D` (`0x080D71A8`), speed `0x12_0000`, spr `10-03`, arm `0c-1b`, pose `0x36`, SFX `0xB0`, `hit_mod=0`, status `0x12`. |
| `rope1` | `0x04E` | `zapring/init.luau` | **P0** | **Completely wrong chip**: fires instant forward 2x1 lash (`hit_mod=3`) instead of scanning **8 surrounding solid panels** for enemies and spawning **60-tick destructible Wood vine pillars** (`hit_mod=0`, status `0x30` bind/paralyze). P2: sprite `14-05` vs `10-2e`, pose `buster_shoot` vs `0x3A..0x3C`, SFX `0x75` vs `0xFF`. | Fam 50 sub 0 (`0x080B3E10`), offsets `0x080B3ED4` (8-ring), AttackObj `0x62` (`0x080E2334`), spr `10-2e` anim 0..2, pose `0x3A/0x3B/0x3C`, SFX `0xFF`, `hit_mod=0`, status `0x30`. |
| `rope2` | `0x04F` | `zapring/init.luau` | **P0** | Same P0 mechanic mismatch as `rope1` (instant 2x1 forward lash vs 8-surrounding enemy-targeted 60t destructible vine pillars, dmg 70, status `0x31`). Same P2 diffs. | Fam 50 sub 1 (`0x080B3E10`), AttackObj `0x62` (`0x080E2334`), spr `10-2e`, pose `0x3A..0x3C`, SFX `0xFF`, `hit_mod=0`, status `0x31`. |
| `rope3` | `0x050` | `zapring/init.luau` | **P0** | Same P0 mechanic mismatch as `rope1` (instant 3x1 forward lash vs 8-surrounding enemy-targeted 60t destructible vine pillars, dmg 90, status `0x32`). Same P2 diffs. | Fam 50 sub 2 (`0x080B3E10`), AttackObj `0x62` (`0x080E2334`), spr `10-2e`, pose `0x3A..0x3C`, SFX `0xFF`, `hit_mod=0`, status `0x32`. |
| `tornado` | `0x041` | `zapring/init.luau` | **P1** | Gives mercy flash (`hit_mod=3`) on hit 8 vs ROM `hit_mod=1` on ALL hits; missing **Sand Panel combo** (`flags & 0x200` -> 16 hits, Plus `+` region `0x09`, pal 2); chip uses normal `coll 0x01/0x02` (not piercing/wind!). P2: pal `0` vs `1` on `0c-1a`, pose `buster_shoot` vs `0x2D..0x30` + `0c-19`, SFX `0x74` vs `0xC9`. | Fam 15 (`0x080B1D6E`), AttackObj `0x0E` (`0x080D72E4`), coll `0x080D7430`, sand check `0x080D7468`, spr `0c-1a` **pal 1** (sand: pal 2), arm `0c-19`, pose `0x2D..0x30`, SFX `0xC9`. |
| `burner` | `0x036` | `zapring/init.luau` | **P0** | **Completely wrong hitbox & mechanics**: instant 1-tick piercing **8-panel ring** (hits diagonals, misses user's own panel, ignores holes, no user protection) vs ROM **2-stage 5-panel PLUS (`+`)**: center `(x,y)` for 15t while protecting user, then +10t later 4 cardinal neighbors for 15t each (solid-only, non-piercing `0x01/0x02`). P2: spr `0c-10` vs `10-22`, pose `buster_shoot` vs `0x23`, SFX `0x76` vs `0x80` x2. | Fam 36 (`0x080B3316`), cardinal offsets `0x080B3420`, user protect `0x080B33A0` (`0x080AF348`), AttackObj `0x3B` (`0x080DD2BC`), spr `10-22` anim `0->1->2`, pose `0x23`, SFX `0x80`. |

##### 3. Yo-Yo / Boomerang / Lance (`content/exe3/chips/yoyo/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `yo-yo1` | `0x045` | `yoyo/init.luau` | **P1** | Deals **4 hits** at tip (arrival + 3 spin hits) vs ROM **3 hits** (arrival + 2 spin hits); `hit_mod=3` on final spin hit vs ROM `hit_mod=1` on all hits; indestructible piercing vs ROM clashable/destructible `0x4F/0x50`; exits player action at 18t vs ROM locks until Yo-Yo returns; discrete 4t/panel vs smooth `5 px/t` (8t/panel). P2: spr `0c-21` vs `10-12` anim 4 pal 0, pose `0x23` vs `0x36`, SFX `0x77/0x6E` vs `0xBD`. | Fam 35 sub 0 (`0x080B329A`), player wait `0x080B3308`, AttackObj `0x3A` (`0x080DCFEC`), spr `10-12` anim 4 pal `0`, pose `0x36`, SFX `0xBD`, `hit_mod=1`. |
| `yo-yo2` | `0x046` | `yoyo/init.luau` | **P1** | Same P1/P2 diffs as `yo-yo1` (4 tip hits vs 3, `hit_mod=3` vs `1`, indestructible vs `0x4F/0x50`, early player unlock, discrete speed, spr `10-12` pal `1`, pose `0x36`, SFX `0xBD`). | Fam 35 sub 1 (`0x080B329A`), AttackObj `0x3A` (`0x080DCFEC`), spr `10-12` anim 4 **pal 1**, pose `0x36`, SFX `0xBD`, `hit_mod=1`. |
| `yo-yo3` | `0x047` | `yoyo/init.luau` | **P1** | Same P1/P2 diffs as `yo-yo1` (4 tip hits vs 3, `hit_mod=3` vs `1`, indestructible vs `0x4F/0x50`, early player unlock, discrete speed, spr `10-12` pal `2`, pose `0x36`, SFX `0xBD`). | Fam 35 sub 2 (`0x080B329A`), AttackObj `0x3A` (`0x080DCFEC`), spr `10-12` anim 4 **pal 2**, pose `0x36`, SFX `0xBD`, `hit_mod=1`. |
| `boomer1` | `0x054` | `yoyo/init.luau` | **P1** | `hit_mod=3` + `piercing` vs ROM **`hit_mod=1`** + non-piercing `0x01/0x02`; extra panel `(1,2)`/`(6,2)` on path (ROM starts `(0,3)`/`(7,3)` and exits `(0,1)`/`(7,1)`); discrete 5t/panel vs smooth `8 px/t`; 16t endlag vs ROM 50t. P2: spr `0c-1e` anim 1 vs `10-1d` anim 0, pose `0x23` vs idle, SFX `0x77/0x75` vs `0xB7`. | Fam 30 sub 0 (`0x080B2CB8`), speed `8 px/t` (`0x080B2D1E`), AttackObj `0x2A` (`0x080DB0C4`), spr `10-1d` anim 0, idle pose + 50t endlag, SFX `0xB7`, `hit_mod=1`. |
| `boomer2` | `0x055` | `yoyo/init.luau` | **P1** | Same P1/P2 diffs as `boomer1` (`hit_mod=3` + piercing vs `hit_mod=1` non-piercing, extra `(1,2)`/`(6,2)` panel, discrete 4t/panel vs smooth `9 px/t`, 16t vs 50t endlag, spr/pose/SFX). | Fam 30 sub 1 (`0x080B2CB8`), speed `9 px/t` (`0x080B2D1F`), AttackObj `0x2A` (`0x080DB0C4`), spr `10-1d` anim 0, SFX `0xB7`, `hit_mod=1`. |
| `boomer3` | `0x056` | `yoyo/init.luau` | **P1** | Same P1/P2 diffs as `boomer1` (`hit_mod=3` + piercing vs `hit_mod=1` non-piercing, extra `(1,2)`/`(6,2)` panel, discrete 3t/panel vs smooth `10 px/t`, 16t vs 50t endlag, spr/pose/SFX). | Fam 30 sub 2 (`0x080B2CB8`), speed `10 px/t` (`0x080B2D20`), AttackObj `0x2A` (`0x080DB0C4`), spr `10-1d` anim 0, SFX `0xB7`, `hit_mod=1`. |
| `lance` | `0x04B` | `yoyo/init.luau` | **P1** | Instant 1-tick `piercing` `hit_mod=3` + unconditional manual `opp.panel_x` teleport (pushes even invuln/guarded/unhit targets!) vs ROM 3 AttackObj `0x45` thrusting 8t + persisting 20t with `hit_mod=0` & non-piercing engine knockback `coll 0x4B/0x02`; uses `buster_shoot` + counter window vs ROM idle + 20t endlag. P2: static `z=0` effect vs smooth thrust at `z=0x10_0000`. | Fam 42 (`0x080B3934`), AttackObj `0x45` (`0x080DE54C`), `coll 0x4B/0x02` (`0x080DE5DC`), spr `0c-27` anim 0 (`z=0x10_0000`), idle + 20t endlag, SFX `0x75`, `hit_mod=0`. |

##### 4. Elec — Sensor & Plasma (`content/exe3/chips/elec/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `sensor1` | `0x065` | `elec/init.luau` | **P0** | Indestructible (ROM has **HP 40** + `coll 0x2F/0x38`); spawns on holes/occupied panels (ROM checks `req=0x10, forb=0x3E000000`); lifetime 480t vs ROM 600t; instant 1-tick beam (`hit_mod=3`, no paralyze) vs ROM 64t propagating beam AttackObj `0x7E/0x7D` (`10-46`, `hit_mod=0`, **Paralyze `0x10`**, SFX `0x8D`). P2: missing visible scanner line EffectObj `0x22`, SFX `0x76` vs `0x8D`. | Dim 62 (`0x080FCF60`), check `0x080FD026`, AttackObj `0x83` (`0x080E7664`, HP 40 `0x080E773E`), Beam `0x7E/0x7D` (`0x080E6C34`, `10-46`), Scanner FX `0x22` (`0x080F042A`), spr `10-47` pal 0, SFX `0x8D`. |
| `sensor2` | `0x066` | `elec/init.luau` | **P0** | All `sensor1` P0 diffs (HP **60**, placement check, 600t life, 64t paralyzing `hit_mod=0` beam) PLUS **wrong scan geometry**: scans 3-row fan instead of ROM **diagonal aim** (`y=1 -> dy=+1`, `y=2 -> dy=0`, `y=3 -> dy=-1`)! P2: ROM palette is **`3`** (`sub*3`), not `1`. | Dim 62 param 1 (`0x080FCF60`), AttackObj `0x83` (`0x080E7664`, HP 60, diag table `0x080E7738`), Beam `0x7E/0x7D` (`10-46`), spr `10-47` **pal 3**, SFX `0x8D`. |
| `sensor3` | `0x067` | `elec/init.luau` | **P0** | All `sensor2` P0 diffs (HP **80**, placement check, diagonal scan, 600t life, 64t paralyzing `hit_mod=0` beam) PLUS **despawns after 1 shot** instead of ROM **re-arming after 60t cooldown and firing repeatedly**! P2: ROM palette is **`6`** (`sub*3`), not `2`. | Dim 62 param 2 (`0x080FCF60`), AttackObj `0x83` (`0x080E7664`, HP 80, re-arm `0x080E789C`), Beam `0x7E/0x7D` (`10-46`), spr `10-47` **pal 6**, SFX `0x8D`. |
| `plasma1` | `0x05D` | `elec/init.luau` | **P1** | Mostly accurate (HP 10, 40t startup, 12x/15t, `04-23` pal 0, spark `10-07`, SFX `0xBC`, `hit_mod=1`). Diffs: allows multiple simultaneous Plasmas (ROM `0x080B5712` replaces prior one); spark only hits on frame 1 (ROM `0x080D9336` persists across all 15t until hit); requires solid panel vs ROM `0x10000` (panel exists). P2: spark `z=0x10` vs `0`, 18t vs 32t post-anim endlag, death SFX `0x76`. | Fam 27 sub 0 (`0x080B2AD4`), replace `0x080B2B0E`, AttackObj `0x20` (`0x080D938E`, check `0x080D9482`), Spark `0x1F` (`0x080D92A8`), spr `04-23` pal 0 + `10-07` anim 1, SFX `0xBC`. |
| `plasma2` | `0x05E` | `elec/init.luau` | **P1** | Same P1/P2 diffs as `plasma1` (HP 30, 15x/12t, pal 1 match; missing single-instance replace, 1-tick vs 15-tick spark hitbox persistence, solid vs panel-exists check, spark `z`/endlag/death SFX). | Fam 27 sub 1 (`0x080B2AD4`), AttackObj `0x20` (`0x080D938E`), Spark `0x1F` (`0x080D92A8`), spr `04-23` pal 1 + `10-07` anim 1, SFX `0xBC`. |
| `plasma3` | `0x05F` | `elec/init.luau` | **P1** | Same P1/P2 diffs as `plasma1` (HP 50, 22x/8t, pal 2 match; missing single-instance replace, 1-tick vs 15-tick spark hitbox persistence, solid vs panel-exists check, spark `z`/endlag/death SFX). | Fam 27 sub 2 (`0x080B2AD4`), AttackObj `0x20` (`0x080D938E`), Spark `0x1F` (`0x080D92A8`), spr `04-23` pal 2 + `10-07` anim 1, SFX `0xBC`. |

##### 5. Magnum / Ratton / NoBeam / IceWave / Arrow (`content/exe3/chips/magnum/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `magnum1` | `0x051` | `magnum/init.luau` | **P0** | Sweeps 1x1 panel across all 9 enemy panels & **aimbot-locks on enemy** vs ROM sweeps full **3-panel vertical column** L->R (`7t/col`, 180t auto-fire); breaks panels unconditionally vs ROM `0x0800B610` breaks **unoccupied panels only**. P2: cursor `14-05`/SFX `0x6C`/blast `0x76` vs ROM SE `0x3B` (`10-41` anim 1)/SFX `0xEC`/pose `0x35`/fire `0x74`/SE `0x3C` (`14-10`)/30t shake. | Dim 49 param 0 (`0x080FC138`), ActorObj `0x40` (`0x080C7E6C`), step `7t` (`0x080C7F38`), col blast `0x080C8206`, break `0x0800B610`, SE `0x3B/0x3C`, SFX `0xEC`/`0x74`/`0x7B`. |
| `magnum2` | `0x052` | `magnum/init.luau` | **P0** | Same P0 cursor/aimbot/break diffs as `magnum1`: ROM sweeps full **3-panel horizontal row** bottom->top (`y=3->2->1`, `7t/row`, 180t auto-fire) and breaks only unoccupied panels. Same P2 sprite/pose/SFX diffs. | Dim 49 param 1 (`0x080FC138`), ActorObj `0x40` (`0x080C7E6C`), step `7t`, row blast `0x080C8266`, break `0x0800B610`, SE `0x3B/0x3C`, SFX `0xEC`/`0x74`/`0x7B`. |
| `magnum3` | `0x053` | `magnum/init.luau` | **P0** | Same P0 cursor/aimbot/break diffs PLUS **wrong blast shape**: blasts a **5-panel Plus (`+`)** instead of ROM's **fast R->L 3-panel vertical column** (`5t/col`, shares `0x080C8206` column blast with Magnum1!). Same P2 diffs. | Dim 49 param 2 (`0x080FC138`), ActorObj `0x40` (`0x080C7E6C`), step **`5t`** (`0x080C7F38[2]`), col blast `0x080C8206`, SE `0x3B/0x3C`, SFX `0xEC`/`0x74`/`0x7B`. |
| `ratton1` | `0x03A` | `magnum/init.luau` | **P1** | Indestructible vs ROM **HP=1** (`0x080B2C4A`); flies over holes vs ROM requires solid panel (`0x0800BF4A`); wrong turn check (ROM `0x080D9E98` checks current col & behind); discrete 7t/panel vs smooth `2.5 px/t` (`16t` horiz / `9t` vert); `hit_mod=3` vs `2`; 18t vs 66t endlag. P2: spr `0c-20` anim 1 pal 0 vs `10-0b` anim 0 pal 0, SFX `0x6E` vs `0xB0`. | Fam 29 sub 0 (`0x080B2BE4`), HP=1 (`0x080B2C4A`), AttackObj `0x23` (`0x080D9C4C`), speed table `0x080D9C2C`, spr `10-0b` anim 0 pal 0, SFX `0xB0`/`0x76`, `hit_mod=2`. |
| `ratton2` | `0x03B` | `magnum/init.luau` | **P1** | Same P1/P2 diffs as `ratton1` (indestructible vs HP=1, flies over holes, turn scan, 5t vs `3.2 px/t` = `12t`/`7t`, `hit_mod=3` vs `2`, 18t vs 50t endlag, spr `10-0b` pal **1**, SFX `0xB0`). | Fam 29 sub 1 (`0x080B2BE4`), AttackObj `0x23` (`0x080D9C4C`), spr `10-0b` anim 0 **pal 1**, SFX `0xB0`/`0x76`, `hit_mod=2`. |
| `ratton3` | `0x03C` | `magnum/init.luau` | **P1** | Same P1/P2 diffs as `ratton1` (indestructible vs HP=1, flies over holes, turn scan, 4t vs `4.0 px/t` = `10t`/`6t`, `hit_mod=3` vs `2`, 18t vs 42t endlag, spr `10-0b` pal **2**, SFX `0xB0`). | Fam 29 sub 2 (`0x080B2BE4`), AttackObj `0x23` (`0x080D9C4C`), spr `10-0b` anim 0 **pal 2**, SFX `0xB0`/`0x76`, `hit_mod=2`. |
| `nobeam1` | `0x094` | `magnum/init.luau` | **P0** | Scans ALL panels behind user and **still fires at 50% dmg** if none found vs ROM checks ONLY `(x - dir, y)` for `0x0E000000` and **duds completely (0 dmg, no beam)** if empty! Also P1: instant piercing `hit_mod=3` no-status vs ROM traveling beam (`16px/4t`), non-piercing `0x01/0x02`, **`hit_mod=0`**, **Paralyze `0x10`**. P2: spr `0c-35` vs **`0c-36`** (+ trail SE `0x44`), pose `buster_shoot` vs `0x3E`, SFX `0x7D` vs `0x97`. | Fam 58 sub 0 (`0x080B4324`), behind check `0x080B436E`, AttackObj `0x81` (`0x080E7254`), spr `0c-36` + SE `0x44` (`0c-36` anim 1), pose `0x3E`, SFX `0x97`, `hit_mod=0`, status `0x10`. |
| `nobeam2` | `0x095` | `magnum/init.luau` | **P0** | Same P0/P1/P2 diffs as `nobeam1` (behind-check + 50% fallback vs strict 1-panel-behind dud; instant piercing `hit_mod=3` vs traveling `hit_mod=0` paralyzing beam; spr `0c-36`, pose `0x3E`, SFX `0x97`). | Fam 58 sub 1 (`0x080B4324`), AttackObj `0x81` (`0x080E7254`), spr `0c-36` + SE `0x44`, pose `0x3E`, SFX `0x97`, `hit_mod=0`, status `0x10`. |
| `nobeam3` | `0x096` | `magnum/init.luau` | **P0** | Same P0/P1/P2 diffs as `nobeam1` (behind-check + 50% fallback vs strict 1-panel-behind dud; instant piercing `hit_mod=3` vs traveling `hit_mod=0` paralyzing beam; spr `0c-36`, pose `0x3E`, SFX `0x97`). | Fam 58 sub 2 (`0x080B4324`), AttackObj `0x81` (`0x080E7254`), spr `0c-36` + SE `0x44`, pose `0x3E`, SFX `0x97`, `hit_mod=0`, status `0x10`. |
| `icewave1` | `0x05A` | `magnum/init.luau` | **P0** | Travels in a straight line and **freezes panels to Ice** (piercing, indestructible) vs ROM **single sine-wave projectile oscillating across 2 adjacent rows**, **DOES NOT freeze panels**, destructible/clashable (`coll 0x2F/0x36`), `20t/panel`. P2: spr `0c-2a` vs `10-25` anim 0, pose `0x23` vs `0x36` (60t+60t), SFX `0xAE` vs `0x10C`. | Fam 38 sub 0 (`0x080B3510`), speed `20t` (`0x080B3584`), AttackObj `0x40` (`0x080DDC58`), sine `0x080DDCAC`, spr `10-25` anim 0, pose `0x36`, SFX `0x10C`. |
| `icewave2` | `0x05B` | `magnum/init.luau` | **P0** | Spawns **2 straight rows** & freezes panels vs ROM **1 single sine-wave projectile** (`15t/panel`, oscillating across 2 rows, no ice panels, destructible `0x2F/0x36`). Same P2 diffs. | Fam 38 sub 1 (`0x080B3510`), speed `15t` (`0x080B3585`), AttackObj `0x40` (`0x080DDC58`), spr `10-25` anim 0, pose `0x36`, SFX `0x10C`. |
| `icewave3` | `0x05C` | `magnum/init.luau` | **P0** | Spawns **3 straight rows** & freezes panels vs ROM **1 single sine-wave projectile** (`10t/panel`, oscillating across 2 rows, no ice panels, destructible `0x2F/0x36`). Same P2 diffs. | Fam 38 sub 2 (`0x080B3510`), speed `10t` (`0x080B3586`), AttackObj `0x40` (`0x080DDC58`), spr `10-25` anim 0, pose `0x36`, SFX `0x10C`. |
| `arrow1` | `0x060` | `magnum/init.luau` | **P0** | Spawns from back col `(1/6, y)` and flies straight piercing vs ROM spawns **1 arrow** at `(x+dir, y)`, flies straight across own area, pauses `15t` on enemy area, then **flies DIAGONALLY** (`vx=±8, vy=±4.8`), non-piercing `0x01/0x02`, stops on obstacles (`0x0E000000`). P2: spr `14-05` vs `04-1d` anims `0/1/3` pal `0`, SFX `0x6E` vs `0x7D`. | Fam 55 sub 0 (`0x080B41D8`), AttackObj `0x88` (`0x080E80B4`), diag turn `0x080E8234`, spr `04-1d` anim `0/1/3` pal `0`, SFX `0x7D`. |
| `arrow2` | `0x061` | `magnum/init.luau` | **P0** | Spawns **2 straight rows** from back col vs ROM **1 single diagonal-turning arrow** from `(x+dir, y)` (stops on obstacles, non-piercing). P2: spr `04-1d` pal **`2`** (`tier*2`), SFX `0x7D`. | Fam 55 sub 1 (`0x080B41D8`), AttackObj `0x88` (`0x080E80B4`), spr `04-1d` anim `0/1/3` **pal 2**, SFX `0x7D`. |
| `arrow3` | `0x062` | `magnum/init.luau` | **P0** | Spawns **3 straight rows** from back col vs ROM **1 single diagonal-turning arrow** from `(x+dir, y)` (stops on obstacles, non-piercing). P2: spr `04-1d` pal **`4`** (`tier*2`), SFX `0x7D`. | Fam 55 sub 2 (`0x080B41D8`), AttackObj `0x88` (`0x080E80B4`), spr `04-1d` anim `0/1/3` **pal 4**, SFX `0x7D`. |

---

#### Detailed Findings by Module & Chip Family

##### Module 1: Swords (`content/exe3/chips/sword/init.luau`, `varswrd.luau`, `lib/swords.luau`)

###### 1.1 Shared Sword Family 8 (`0x080B16CE`, AttackObj `0x06` `0x080D6664`)
- **Table `0x080D65CC`** (8 bytes/row: `[anim, pal, spr_ii, region, status, b5, b6, hit_mod]`):
  - `bambswrd` (`0x024`, row 6 at `0x080D65FC`): **`hit_mod = 1`** (no mercy invulnerability flash). Saito `sword/init.luau:38` hardcodes `hit_mod = 3` in the shared `slash()` helper used at line 129. **(P1)**
  - `muramasa` (`0x0CA`, row 9 at `0x080D6614`): **`region = 0x02` (2x1 LongSwrd)**. Saito `sword/init.luau:330` passes `regions.single` (1x1). Damage formula (`hp_lost`, cap 999) matches ROM `0x080105F4`. **(P1)**
  - `fireswrd` / `aquaswrd` / `elecswrd` / `bambswrd` (rows 3–6 at `0x080D65E4..FC`): ROM passes `pal = 0` (`0x080D66BA: ldrb r0, [r7, #1]; bl 0x08002BDC`), whereas Saito `sword/init.luau:27-30` sets palettes `5 / 6 / 7 / 8`. **(P2)**
  - `custswrd` (`0x025`, row 7), `muramasa` (`0x0CA`, row 9), `heroswrd` (`0x0CF`, row 10): ROM SFX table `0x080B17AC` has `0x75` (`sword-swing`) for all three (only LifeSwrd row 13 & row 14 use `0xD6`), whereas Saito `sword/init.luau:145, 330, 344` uses `swords.big_swing` (`0xD6`). **(P2)**
  - All Family 8 swords: ROM spawns slash at `z = +0x08_0000` (`0x080D66D6`) and transitions MegaMan into recovery pose `0x1F` (`0x080B17EE`); Saito (`exelib/swords/slash.luau:243`) spawns at `z = +0x10_0000` and holds `0x1D` through recovery. **(P2)**

###### 1.2 `varswrd` (`0x026`, Fam 32 `0x080B2D24`, AttackObj `0x31` `0x080DBD9C`) — **P0**
- **Missing 6th Command Input & Hijacked SonicBoom (`sword/varswrd.luau:125-152`)**:
  - ROM `0x080B2D8C` defines **6 sequences** (`0..5`):
    - `[0]` (`0x080B2DBC`): Down, Down+Fwd, Fwd -> LongSwrd
    - `[1]` (`0x080B2DC4`): Back, Down+Back, Down, Down+Fwd, Fwd -> FighterSwd (`0x0F`, 3x1)
    - `[2]` (`0x080B2DD0`): Up, Fwd, Down -> WideSwrd
    - `[3]` (`0x080B2DD8`): Down, Back, Up, Fwd, Down -> LifeSwrd (`0x0D`, 2x3, SFX `0xD6`)
    - `[4]` (`0x080B2DE4`): Back, A+B, Fwd, A+B -> **SonicBoom** (3 null waves, `hit_mod = 3` on EVERY wave at `0x080B2ED8`)
    - `[5]` (`0x080B2DEE`): **A+B, A, A+B, Back, Down, Up** (`0x03, 0x01, 0x03, 0x21, 0x81, 0x41, 0`) -> **Elemental Sonic** (4 waves: countdown `4->3->2->1` indexes `0x080B2F04 = [4, 1, 3, 2]` with `count-1` = **Aqua(3->2), Elec(1), Fire(2), Wood(4)** — in EXE3 elements: `2=Fire, 3=Aqua, 1=Elec, 4=Wood`, wait: `[4, 1, 3, 2]` at `r0=3,2,1,0` gives `2(Fire), 3(Aqua), 1(Elec), 4(Wood)`! Specifically: wave 1 = `2 (Fire)`, wave 2 = `3 (Aqua)`, wave 3 = `1 (Elec)`, wave 4 = `4 (Wood)` — i.e., **Fire -> Aqua -> Elec -> Wood** — wait: `0x080B2F04` bytes are `[4, 1, 3, 2]`: index 3 = `2` (Fire), index 2 = `3` (Aqua), index 1 = `1` (Elec), index 0 = `4` (Wood)! So the element order Fire -> Aqua -> Elec -> Wood actually matches Saito's `{1, 2, 3, 4}`, while `hit_mod` is `1` for waves 1–3 and `3` for wave 4!).
  - Because `@exelib/varswrd/action.luau` allocates `progress = "u8[5]"`, Saito dropped sequence `[5]` completely and bound sequence `[4]` (`Back, A+B, Fwd, A+B`) to `elemsonic_pick`, making `sonicbom_pick` unreachable dead code.
- **PVP Side-1 Input Reversal (`content/exelib/varswrd/action.luau:128`)**:
  - Saito selects `sequences[me.flip]`, whereas ROM `0x080B2D5A` reads `[r5, #0x16]` (`me.alliance`). In PVP, Side 1 has `alliance = 1, flip = 0` when unflipped, causing Side 1 to use Side 0's (reversed Left/Right) command table.
- **SonicBoom Piercing / Swing / Visuals (`sword/varswrd.luau:21-64`)**:
  - ROM AttackObj `0x31` (`0x080DBE7C..80`) checks `tst r0, #0x3E000000` and **destroys itself on first hit** (does NOT pierce). Saito `varswrd.luau:54` sets `pierces = true`, which also triggers `sonic_boom.luau:90-91` to force `piercing_palette = 0` on every Elemental Sonic wave (wiping out ROM's elemental palette table `0x080DBE58 = [0, 2, 5, 4, 8]`).
  - ROM swings MegaMan's full sword animation (`0x1D` + blade attachment + SFX `0x75`) for each wave rather than firing every 6 ticks from a single pose.
  - ROM success chime (`0x080B2E6A`) is SFX `0x117` (`0x18 + 0xFF`), vs Saito `damage-bonus` (`0xED`) at `varswrd.luau:112`.

###### 1.3 `stepswrd` (`0x027`) & `stepcros` (`0x028`) (Fam 41 `0x080B373C`) — **P1**
- **Step Check & Transit (`sword/init.luau:173-201`, `exelib/swords/slash.luau:44,171`)**:
  - ROM `0x080B3790` checks destination 2 panels ahead with `require = 0x00010000` (panel exists — allows holes), `forbid = 0x38000000` (actors + obstacles). Saito checks `forbid = 0x0F800000` (bodies only — misses `0x00080000` obstacles).
  - ROM hides user & disables collision during 3-tick warp transit (`0x080B37E4`), spawns SimpleEffect `0x04` (`14-09` anim 2) at origin and `0x05` (`14-09` anim 3) at destination, and plays `0x0B` if blocked. Saito warps instantaneously in 1 tick with EXE5/6 yellow afterimage shaders.
- **`stepcros` Double-Diagonal & Sprite/SFX (`sword/init.luau:194-200`)**:
  - ROM `0x080B384E..80` spawns AttackObj `0x06` sub `0x10` (sprite **`0c-35`** anim 0 pal 0, **region `0x12` rising diagonal**) followed by AttackObj `0x07` (`0x080D684C`, **region `0x13` falling diagonal**, `hit_mod = 3`, `coll 0x26/0x27`).
  - Saito uses a single unified `regions.x` hitbox, wrong sprite `0c-0d` (`big-slash`, pal 0) instead of **`0c-35`** (mislabeled `nobeam` in `compat/assets.toml`), and wrong SFX `0xD6` (`big-sword-swing`) instead of ROM `0x75` (`0x080B37F8`).

###### 1.4 `slasher` (`0x02B`, Fam 40 `0x080B365C`) — **P1**
- `sword/init.luau:240-258`: `SLASHER_START` triggers `SLASHER_HIT` on frame 0 without checking `me:held("a")` (ROM `0x080B36A6` checks A held first) and does not spawn AttackObj `0x43` (`0x080DE368`) as the active sensor/interruption handle.
- `sword/init.luau:261-263, 288`: Uses pose `0x1D` (`SWORD_SWING`), blade anim `0`, SFX `0xD6` (`big-sword-swing`), and `me.alliance` for flip; ROM uses pose `0x1E` -> `0x1F` (`0x080B36B2`, `0x080B3720`), blade `0c-00` anim **`1`** (`0x0C000100`), and SFX **`0x75`** (`sword-swing`). *(Note: `SLASHER_SEARCH` bitmasks `0x00200000`/`0x00400020` at lines 217–218 match Saito's `rules/collision.luau:7` navi flags.)*

---

##### Module 2: ZapRing / Rope / Tornado / Burner (`content/exe3/chips/zapring/init.luau`)

###### 2.1 `zapring1..3` (`0x042..0x044`, Fam 14 `0x080B1C90`, AttackObj `0x0D` `0x080D71A8`) — **P1**
- **P1 Mechanics (`zapring/init.luau:62-97, 328-368`)**:
  - `hit_mod = 3` at line 80 causes mercy invincibility frames; ROM `0x080D7230` sets **`hit_mod = 0`** (no mercy flash).
  - Fixed `statuses.paralyze` (120t) vs ROM escalating status byte `[col+4] = 0x10 / 0x11 / 0x12`.
  - Discrete panel teleport every `5 / 4 / 3` ticks vs ROM continuous pixel velocity `0x0A_0000 / 0x0E_0000 / 0x12_0000` (`10 / 14 / 18 px/t` at `0x080D7190`).
  - Manual `battle.alive_actors` scan at line 66 ignores obstacles/cubes/shields, whereas ROM `0x080D726C` destroys on any collision (`[col+0x2C] != 0`).
- **P2 Presentation (`zapring/init.luau:16, 117-123`)**:
  - Uses `hit-sparks` (`14-05` anim 4) at `z = 0x10_0000` instead of **`10-03`** (`zapring`, already in `assets.toml:74`!) anim 0 pal 0 at `z = 0x18_0000`.
  - Uses `anims.buster_shoot` and `buster-shot` (`0x6E`) instead of pose **`0x36`** + weapon attachment **`0c-1b`** (`zapring-arm`, already in `assets.toml:53`) + SFX **`0xB0`** (`zapring`, already in `assets.toml:144`).

###### 2.2 `rope1..3` (`0x04E..0x050`, Fam 50 `0x080B3E10`, AttackObj `0x62` `0x080E2334`) — **P0**
- **P0 Complete Mechanic Mismatch (`zapring/init.luau:151-191, 370-411`)**:
  - Saito implements Rope as an instant forward line lash (`regions.long` for Rope1/2, `regions.line` for Rope3) with `hit_mod = 3`.
  - ROM `0x080B3E6A..080B3EE4` checks all **8 surrounding panels** (`0x080B3ED4`: `(-1,-1)..(1,1)`) for solid panels (`0x10`) occupied by an enemy (`0x1A000000`/`0x2C000000`) and spawns a **60-tick destructible Wood vine pillar** (`AttackObj 0x62`, `coll 0x35/0x39`, **`hit_mod = 0`**, status **`0x30 / 0x31 / 0x32`** bind/paralyze) on every matching panel. All 3 tiers share the same 8-panel surrounding range.
- **P2 Presentation**:
  - ROM sprite **`10-2e`** anims `0->1->2`, player pose **`0x3A -> 0x3B (60t) -> 0x3C`**, SFX **`0xFF`** (`0x080E23C8`) — vs Saito `hit-sparks` (`14-05` anim 5), `anims.buster_shoot`, and `sword-swing` (`0x75`).

###### 2.3 `tornado` (`0x041`, Fam 15 `0x080B1D6E`, AttackObj `0x0E` `0x080D72E4`) — **P1**
- **P1 Mechanics (`zapring/init.luau:23, 219-289`)**:
  - Sets `hit_mod = 3` on hit 8 (line 239); ROM `0x080D7450` uses **`hit_mod = 1`** on ALL hits (no mercy frames ever).
  - Missing **Sand Panel combo (`0x080D7468`)**: when `GetPanelFlags & 0x200` (Sand), Tornado doubles tick speed (hits every 2 ticks -> **16 hits** total), expands hitbox from single panel to **Plus `+` (`region 0x09`)**, switches to palette `2`, and sets null element!
  - Uses `WIND_ELEM` + `collision.piercing` (lines 23, 236); ROM `0x080D7430..48` for Tornado chip (`sub = 0`, `r4 = 0`) uses normal `coll 0x01/0x02` (neither piercing nor wind — bit 6 `0x40` for wind `0x1B` is only set by WoodStyle Twister!).
- **P2 Presentation (`zapring/init.luau:17, 224, 244, 267`)**:
  - `0c-1a` (`tornado`) uses palette `0` instead of ROM **palette `1`** (`0x080D7394[0] = 1`).
  - Uses `anims.buster_shoot` and `spreader` (`0x74`) + `small-puff` spam instead of poses **`0x2D -> 0x2E -> 0x2F -> 0x30`** + attachment **`0c-19`** (`twister-arm`, anims `0..3`) + SFX **`0xC9`** (`twister`, already in `assets.toml:155`).

###### 2.4 `burner` (`0x036`, Fam 36 `0x080B3316`, AttackObj `0x3B` `0x080DD2BC`) — **P0**
- **P0 Complete Hitbox & Mechanics Mismatch (`zapring/init.luau:292-325`)**:
  - Saito fires an instant single-tick piercing **8-panel ring** around the user (hits all 4 diagonals, misses user's own panel, ignores holes, no user protection).
  - ROM `0x080B336A..080B3426` is a **2-stage 5-panel PLUS (`+`) flame**:
    1. Checks `0x0800BF4A` (user is on a solid panel), spawns AttackObj `0x3B` on user's **own panel `(x, y)`** (15t active), and sets user collision protection (`0x080B33A0` -> `0x080AF348` + damage 0).
    2. After **10 ticks** (`0x080B33B0`), spawns 4 flames at the **4 cardinal neighbors** (`0x080B3420`: `(0,-1), (0,1), (1,0), (-1,0)` — NO diagonals!) for 15t each.
    3. Each flame requires a solid panel (`GetPanelFlags & 0x10`) and uses **non-piercing** `coll 0x01/0x02`, `hit_mod = 3`.
- **P2 Presentation**:
  - ROM sprite **`10-22`** anims `0->1->2` pal 0 (`0x080DD2B4`), player pose **`0x23`** (`THROW_LOW`), SFX **`0x80`** played twice (`0x080B33AA`, `0x080B340A`) — vs Saito `0c-10` (`flame`), `anims.buster_shoot`, and `hit-bomb` (`0x76`).

---

##### Module 3: Yo-Yo / Boomerang / Lance (`content/exe3/chips/yoyo/init.luau`)

###### 3.1 `yo-yo1..3` (`0x045..0x047`, Fam 35 `0x080B329A`, AttackObj `0x3A` `0x080DCFEC`) — **P1**
- **P1 Mechanics (`yoyo/init.luau:60-185`)**:
  - Deals **4 hits** at the tip panel (1 arrival hit in phase 0 + 3 hits in phase 1) vs ROM **3 hits** (1 arrival + 2 spin hits at 8t intervals, `0x080DD190`).
  - Sets `hit_mod = 3` on final tip hit (line 106), giving mercy frames that block return hits; ROM uses **`hit_mod = 1`** on ALL hits.
  - Uses indestructible `collision.piercing`; ROM AttackObj `0x3A` uses clashable/destructible `coll 0x4F/0x50` and breaks on obstacle/clash (`0x3E000010`).
  - Player exits action after 18 ticks (line 188); ROM `0x080B3308` keeps player locked in pose `0x36` until the Yo-Yo returns or is destroyed.
  - Discrete `4t/panel` teleport vs ROM smooth `5 px/t` (`8t/panel`).
- **P2 Presentation (`yoyo/init.luau:14, 168-174`)**:
  - Uses `0c-21` (`yoyo`) anim 0 pal 0, pose `0x23` (`THROW_LOW`), and SFX `0x77`/`0x6E`; ROM uses **`10-12`** anim `4` palette **`0 / 1 / 2`** (tier), pose **`0x36`**, and SFX **`0xBD`** (`0x080B32FC`).

###### 3.2 `boomer1..3` (`0x054..0x056`, Fam 30 `0x080B2CB8`, AttackObj `0x2A` `0x080DB0C4`) — **P1**
- **P1 Mechanics (`yoyo/init.luau:197-305`)**:
  - Uses `hit_mod = 3` and `collision.piercing` (lines 249, 252); ROM `0x080DB156` uses **`hit_mod = 1`** (no mercy flash) and **non-piercing** `coll 0x01/0x02` (disables collision on hit until entering next panel at `0x080DB182`).
  - `PERIMETER_SIDE0`/`PERIMETER_SIDE1` (lines 203, 211) visit `(1,2)`/`(6,2)` on return; ROM starts off-screen at `(0,3)`/`(7,3)` and exits off-screen at `(0,1)`/`(7,1)` without visiting `(1,2)`/`(6,2)`.
  - Uniform discrete `5 / 4 / 3 t/panel` vs ROM smooth `8 / 9 / 10 px/t` (`5 / 4.44 / 4t` horiz, `3 / 2.67 / 2.4t` vert).
  - Player endlag `16t` vs ROM `50t` (`0x32` at `0x080B2D08`).
- **P2 Presentation (`yoyo/init.luau:15, 286-291`)**:
  - Uses `0c-1e` (`boomer`) anim 1, pose `0x23` (`THROW_LOW`), SFX `0x77`/`0x75`; ROM uses **`10-1d`** anim `0`, **idle pose** (`0x080B2CE0`), and SFX **`0xB7`** (`obstacle-lift`, already in `assets.toml:147`).

###### 3.3 `lance` (`0x04B`, Fam 42 `0x080B3934`, AttackObj `0x45` `0x080DE54C`) — **P1**
- **P1 Mechanics (`yoyo/init.luau:313-358`)**:
  - Fires a 1-tick `collision.piercing`, `hit_mod = 3` hitbox and then **unconditionally teleports `opp.panel_x`** (lines 343–355) — ignoring invulnerability, i-frames, barriers, guards, superarmor, and even pushing enemies who were not hit!
  - ROM spawns **3 separate AttackObj `0x45`** instances (`y = 3, 2, 1`) starting `+64px` behind `back_col`, thrusting in at `8 px/t` for 8t and persisting 20t (`0x080DE5CA`, `0x080DE666`) with **`hit_mod = 0`**, **non-piercing**, and built-in push collision **`coll 0x4B / 0x02`** (`0x080DE5DC`).
  - Uses `anims.buster_shoot` and opens a counter window; ROM stays in idle pose with `20t` endlag (`0x080B395E`).
- **P2 Presentation**: Static 16-tick effect at `z = 0` vs ROM animated thrust at `z = 0x10_0000` (`0c-27` anim 0, flipped `alliance ^ 1`).

---

##### Module 4: Elec — Sensor & Plasma (`content/exe3/chips/elec/init.luau`)

###### 4.1 `sensor1..3` (`0x065..0x067`, Dim 62 `0x080FCF60`, AttackObj `0x83` `0x080E7664`, Beam `0x7E/0x7D` `0x080E6C34`) — **P0**
- **P0 Missing HP & Destructibility (`elec/init.luau:30-149`)**:
  - ROM `0x080E76F8..080E7716` gives Sensor **HP `40 / 60 / 80`** (`0x080E773E`) and full object collision (`coll 0x2F / 0x38`) so attacks can destroy it. Saito Sensor has no HP and no collision (indestructible).
- **P0 Missing Placement Check (`elec/init.luau:125-149`)**:
  - ROM `0x080FD026` checks target panel 1 ahead with `require = 0x10` (solid), `forbid = 0x3E000000` (unoccupied), failing with SimpleEffect `0x02` if blocked or a hole. Saito spawns on holes and occupied panels.
- **P0 Wrong Scan Geometry & Sensor3 Re-Arm (`elec/init.luau:75-103`)**:
  - ROM `0x080E76A6..BC` + `0x080E7738`: Sensor1 scans straight (`dy = 0`), while **Sensor2 & Sensor3 aim DIAGONALLY** (`y=1 -> dy=+1`, `y=2 -> dy=0`, `y=3 -> dy=-1`), NOT a 3-row fan (`dy <= 1`).
  - **Sensor3 (`sub = 2`)** at `0x080E789C` loops back to scanning after its 64t beam + 60t cooldown, firing repeatedly until lifetime expires or destroyed! Saito despawns Sensor3 after 1 shot (line 103).
- **P0/P1 Beam & Lifetime (`elec/init.luau:52, 85-102`)**:
  - Lifetime is `480t` vs ROM `600t` (`0x258` at `0x080E76F4`).
  - Beam is an instant 1-tick hitbox (`hit_mod = 3`, no status) vs ROM 64-tick propagating AttackObj `0x7E/0x7D` (`10-46`) with **`hit_mod = 0`** and **Paralyze status (`0x10`)** (`0x080E786C`).
- **P2 Presentation**:
  - ROM palette is `sub * 3` (`0 / 3 / 6` at `0x080E769E`) vs Saito `tier - 1` (`0 / 1 / 2`); missing visible scanner laser EffectObj `0x22` (`0x080F042A`, spawned every 24t); beam SFX is **`0x8D`** every 16t (`0x080E6E4C`), not `hit-bomb` (`0x76`).

###### 4.2 `plasma1..3` (`0x05D..0x05F`, Fam 27 `0x080B2AD4`, AttackObj `0x20` `0x080D938E`, Spark `0x1F` `0x080D92A8`) — **P1**
- Already well-researched (sprite `04-23` pal `0/1/2`, HP `10/30/50`, 40t startup, `12x/15t`, `15x/12t`, `22x/8t`, offsets `0x080BA398`, spark `10-07` anim 1, SFX `0xBC`, `hit_mod = 1`, no paralyze on chip variant).
- **Remaining P1 Diffs (`elec/init.luau:198-212, 260-367`)**:
  - Missing single-instance enforcement (`0x080B2B0E` calls `0x080B5712` to replace any existing Plasma owned by the same player).
  - Spark `battle.hitbox` fires on spawn frame 1 only (line 198), whereas ROM AttackObj `0x1F` (`0x080D9336`) keeps collision live across all 15 ticks until hitting once.
  - Placement check requires `field.solid` (`0x10`, line 277) vs ROM `0x080D9482` `require = 0x00010000` (panel exists — allows holes).
- **P2 Diffs**: Spark `z = 0x10_0000` vs ROM `z = 0` (`0x080D92D4`); player exits at fixed 18t vs ROM 32t after anim `0x36` (`0x080B2B54`); destruction plays `hit-bomb` (`0x76`) instead of ROM blink/fade (`0x080D9574`).

---

##### Module 5: Magnum / Ratton / NoBeam / IceWave / Arrow (`content/exe3/chips/magnum/init.luau`)

###### 5.1 `magnum1..3` (`0x051..0x053`, Dim 49 `0x080FC138`, ActorObj `0x40` `0x080C7E6C`) — **P0**
- **P0 Cursor Sweep, Aimbot & Blast Shape (`magnum/init.luau:68-133`)**:
  - Saito sweeps a single 1x1 panel across all 9 enemy panels and auto-locks onto the enemy's exact panel (lines 68–79), and makes Magnum3 blast a 5-panel Plus (`regions.plus`, line 91).
  - ROM ActorObj `0x40` (`0x080C7F20..080C82DC`):
    - **Magnum1 (`param = 0`)**: Sweeps a **3-panel vertical column** left-to-right at **`7t/col`** (`0x080C7F38[0]`), 180t auto-fire (`0x080C7F1E`), blasts that column (`0x080C8206`).
    - **Magnum2 (`param = 1`)**: Sweeps a **3-panel horizontal row** (enemy side) bottom-to-top (`y = 3 -> 2 -> 1`) at **`7t/row`**, blasts that row (`0x080C8266`).
    - **Magnum3 (`param = 2`)**: Sweeps a **3-panel vertical column** right-to-left at **`5t/col`** (`0x080C7F38[2]`), blasts that **vertical column** (`0x080C8074 -> 0x080C8207` — NOT a Plus!).
- **P0/P1 Panel Break Rule (`magnum/init.luau:114`)**:
  - ROM `0x0800B610` breaks **only unoccupied panels** (blocked by `0x3E0A0080`; occupied panels are NOT cracked/broken). Saito breaks every panel unconditionally.
- **P2 Presentation**:
  - ROM displays SimpleEffect `0x3B` (`10-41` anim 1 pal 0) on all 3 target panels + tick SFX **`0xEC`** (`0x080C7F56`), MegaMan pose **`0x35`** (`0x080C8034`) + fire SFX **`0x74`** (`spreader`, `0x080C8058`), explosion SimpleEffect `0x3C` (`14-10` anim 0), crack SFX `0x7B` only when a panel breaks, and 30-tick camera shake (`0x080C825A`) — vs Saito `hit-sparks` anim 0, `sfx-cursor` (`0x6C`), `hit-bomb` (`0x76`).

###### 5.2 `ratton1..3` (`0x03A..0x03C`, Fam 29 `0x080B2BE4`, AttackObj `0x23` `0x080D9C4C`) — **P1**
- **P1 Mechanics (`magnum/init.luau:173-273`)**:
  - Missing **HP = 1** destructibility (`0x080B2C4A`, takes damage at `0x080D9EE0`).
  - Missing **solid-panel check** (`0x0800BF4A` at `0x080D9C7C`, `0x080D9D38`, `0x080D9DD4`, `0x080D9E3E`) — Saito Ratton flies over holes instead of vanishing.
  - Turn check (`magnum/init.luau:210-226`) checks enemy in same column only, whereas ROM `0x080D9E98` scans from current column **backward (`-forward_dir`)** across all rows.
  - Discrete `7 / 5 / 4 t/panel` vs ROM smooth `2.5 / 3.2 / 4.0 px/t` (`16 / 12 / 10t` horiz, `9 / 7 / 6t` vert at `0x080D9C2C`).
  - `hit_mod = 3` vs ROM **`hit_mod = 2`** (`0x080D9CF8`), and 18t vs ROM `66 / 50 / 42t` endlag (`0x080B2CB4`).
- **P2 Presentation**: Uses `0c-20` (`ratton`) anim 1 pal 0 and SFX `0x6E`; ROM uses **`10-0b`** anim `0` palette **`0 / 1 / 2`**, launch SFX **`0xB0`** (`0x080B2C6E`) + hit SFX `0x76`.

###### 5.3 `nobeam1..3` (`0x094..0x096`, Fam 58 `0x080B4324`, AttackObj `0x81` `0x080E7254`) — **P0**
- **P0 Activation & Dud Failure (`magnum/init.luau:295-311`)**:
  - Saito scans ALL panels behind the user and still fires at **50% damage** when nothing is behind.
  - ROM `0x080B436E..80` checks ONLY the single panel immediately behind the user `(panel_x - forward_dir, panel_y)` for `0x0E000000`; if empty, it **duds completely** (0 damage, no beam, no SFX).
- **P1 Projectile, `hit_mod`, Paralyze (`magnum/init.luau:315-327`)**:
  - Saito fires an instant 1-tick piercing row (`hit_mod = 3`, no status).
  - ROM AttackObj `0x81` (`0x080E7254`) travels forward `16 px` per `4 ticks` *(ROM quirk at `0x080B4394`: indexes `0x080B43B8` with `r4 = 1` for all 3 tiers)*, non-piercing `coll 0x01/0x02` (re-enables per panel), **`hit_mod = 0`**, and inflicts **Paralyze (`[col+4] = 0x10`)**.
- **P2 Presentation**: Uses `0c-35` (actually StepCros's X-slash!), `anims.buster_shoot`, and SFX `0x7D` (`cannon`); ROM uses **`0c-36`** (`0x080E72FC`) + trail SimpleEffect `0x44` (`0c-36` anim 1), pose **`0x3E`**, and SFX **`0x97`** (`0x080B43A0`).

###### 5.4 `icewave1..3` (`0x05A..0x05C`, Fam 38 `0x080B3510`, AttackObj `0x40` `0x080DDC58`) — **P0**
- **P0 Trajectory, Row Count & Panel Effect (`magnum/init.luau:343-444, 681-720`)**:
  - Saito spawns **1 / 2 / 3 straight-line piercing waves** that **turn every panel into Ice**.
  - ROM `0x080B354A..88` + `0x080DDC58..DDE10` spawns **1 single sine-wave projectile** for ALL 3 tiers (tiers only change speed: `20 / 15 / 10 t/panel` at `0x080B3584`), oscillating vertically across 2 adjacent rows (`y_center + sin(angle) * 12px`, `0x080DDCAC..DDD7E`).
  - ROM **never changes panels to Ice**!
  - ROM AttackObj `0x40` is destructible/clashable (`coll 0x2F/0x36`, dies on `0x38000010`), non-piercing, `hit_mod = 3`.
- **P2 Presentation**: Uses `0c-2a` (`ice`), pose `0x23` (`THROW_LOW`), SFX `0xAE` (`freeze`); ROM uses **`10-25`** anim `0`, pose **`0x36`** (60t + 60t endlag), and SFX **`0x10C`** (`0x0D + 0xFF` at `0x080B356E`).

###### 5.5 `arrow1..3` (`0x060..0x062`, Fam 55 `0x080B41D8`, AttackObj `0x88` `0x080E80B4`) — **P0**
- **P0 Complete Mechanic Mismatch (`magnum/init.luau:452-550, 723-762`)**:
  - Saito spawns **1 / 2 / 3 straight-line piercing bolts** starting from the user's back column (`x = 1` or `6`).
  - ROM `0x080B4218..4A` + `0x080E80B4..E839C` spawns **1 single arrow** at `(panel_x + forward_dir, panel_y)` for ALL 3 tiers, travels straight (`8 px/t`) across user's own area, **pauses `15t`** upon entering enemy territory (`0x080E8278`), selects a diagonal direction (`y=1 -> down-fwd`, `y=3 -> up-fwd`, `y=2 -> scans diagonally for enemy via 0x080E8354, else up-fwd`), and **flies DIAGONALLY** (`vx = ±8 px/t`, `vy = ±4.8 px/t = 0x4CCCC`)!
  - ROM stops on obstacles (`0x0E000000`, `0x080E8166`) and uses non-piercing `coll 0x01/0x02`, `hit_mod = 3`.
- **P2 Presentation**: Uses `hit-sparks` (`14-05` anim 4) and SFX `0x6E`; ROM uses **`04-1d`** (anims `0` straight, `1` diag-down, `3` diag-up) palette **`0 / 2 / 4`** (`tier * 2` at `0x080E80FE`), flipped `alliance ^ 1`, and SFX **`0x7D`** (`cannon` at `0x080B4224`).


---

### 5.3 Group 3：陷阱、反制、屏障、潜行、毒、风、地形、回复、速度与加攻（69 张 + 2 内部占位）

> *注：原属 `content/exe3/chips/utility/init.luau` 的 7 张芯片（`CrsShld1..3`、`GrabBack`、`GrabRvng`、`Team1`、`Team2`）已归并至 5.4 节，本节明细表共涵盖 69 张可用芯片 + 2 张内部占位（`invalid`/`zeroed`）。*


**ROM Reference**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US, `A6BE`)  
**Scope**: 78 chips across 17 files (`trap`, `utility` assigned subset, `anti`, `barrier`, `stealth`, `invis`, `poison`, `wind`, `rockcube`, `metagel`, `grab`, `stage`, `geddon`, `recov`, `gauge`, `plus`, `copydmg`, `invalid`, `zeroed`) + shared helpers (`lib/guard.luau`, `lib/barriers.luau`, `lib/grab.luau`, `lib/gauge_speed.luau`, `lib/plus.luau`, `rules/init.luau`).  
**Mode**: Read-only audit. No workspace files modified.

#### Summary Counts

| Severity | Count | Description |
| :--- | ---: | :--- |
| **P0** | **31** | Severe mechanics mismatch, missing field object/summon, wrong dimming vs non-dimming execution, fabricated effect, or broken immunity/counter |
| **P1** | **21** | Hitbox/range, HP, duration/lockout, activation condition, or modifier element-filtering mismatch |
| **P2** | **14** | Presentation only (sprite `cc-ii`, anim/palette, flicker ticks, or SFX ID mismatch) |
| **OK** | **12** | Matches ROM behavior & presentation |
| **Total** | **78** | 76 ROM chips + `invalid` + `zeroed` |

---

#### Complete Per-Chip Audit Table

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `blkbomb1` | `0x01B` | `chips/trap/init.luau` | **P0** | Saito spawns a timed 150-tick bomb that **auto-detonates without fire**, skips the parabolic throw arc, and uses wrong SFX (`0x76`). ROM throws parabolic bomb 3 panels ahead (`speed=0x2C000`, `grav=-0x2800`), lands as **100 HP** obstacle (`6000t` life), and **detonates ONLY when hit by Fire-element damage** (`[coll+0x1E] != 0`) into a 3×3 Fire blast. | Act `0x080B359E` (`fam=39`) → AtkObj `0x42` `0x080DDFCC`, fire check `0x080DE214` (`[r7+0x1E]`), detonate `0x080DE23E` (`r4=0x01020282`/`0x01020281`, 60t shake). Spr `0c-02`; SFX `0xED` (detonate), `0x76` (destroyed). |
| `blkbomb2` | `0x01C` | `chips/trap/init.luau` | **P0** | Same as `blkbomb1` (170 dmg): auto-detonates after 150t without Fire trigger, no throw arc, wrong detonation SFX. | Act `0x080B359E` → AtkObj `0x42` `0x080DDFCC` / `0x080DE214`. Spr `0c-02`; SFX `0xED` (detonate), `0x76` (destroyed). |
| `blkbomb3` | `0x01D` | `chips/trap/init.luau` | **P0** | Same as `blkbomb1` (220 dmg): auto-detonates after 150t without Fire trigger, no throw arc, wrong detonation SFX. | Act `0x080B359E` → AtkObj `0x42` `0x080DDFCC` / `0x080DE214`. Spr `0c-02`; SFX `0xED` (detonate), `0x76` (destroyed). |
| `timebomb` | `0x063` | `chips/trap/init.luau` | **P1** | Saito gives **100 HP** (ROM: **50 HP**), uses 180t countdown with `trap-mark` (`0x127`), explodes in **3×3** around bomb with `0x76` (`hit-bomb`), and uses sprite `0c-02` (`bomb`). ROM places **50 HP** obstacle on random enemy panel, counts 3→2→1 (60t+60t+60t+30t=210t), explodes across **entire enemy area** with `0xED` + 60t shake. | Dim `18` `0x080F9A90` (HP table `0x080F9BEC`: `0x32`=50) → AtkObj `0x29` `0x080DADF8`, countdown `0x080DAF06..0x080DAFB6`, explode `0x080DAFBC` (`0x080DB04C`: `0x01020282`/`0x01020281`). Spr **`0c-23`** (anim 0/1); SFX `0x8A` (3/2), `0x8B` (1), `0xED` (boom), `0x76` (destroyed). |
| `mine` | `0x064` | `chips/trap/init.luau` | **P1** | Saito hides banner (`hidden = true` `????`), lasts **1200t**, places immediately with `0c-02` (`bomb`). ROM shows normal banner `0x64`, cycles visibly across random enemy panels for 120t (`60*2`, SFX `0xB1`/hop) before arming invisibly, lasts **6000t** (`0x1770`), uses sprite **`0c-24`**, and explodes with SimpleEffect `0x24` (`14-10`) + `0x76`. | Dim `19` `0x080F9C20` → AtkObj `0x35` `0x080DC5B4`, cycle `0x080DC706`, trigger `0x080DC722..0x080DC76A`, panel filter `0x080DC7A4`. Spr **`0c-24`**, hit FX `14-10` anim 0 (SimpleEff `0x24`); SFX `0xB1` (hop), `0x76` (explode). |
| `guard` | `0x076` | `chips/trap/init.luau` + `lib/guard.luau` | **P1** | Saito guards for **36t** (ROM: **26t** `0x1A`), shows `HEAL_FX` instead of attached shield `0c-03` anim 0, and retaliates with a floating `reflector-shield` that ignores holes and uses `GUARD_SOUND`/`IMPACT_FX`. ROM spawns a ground-bound **ShockWav** (AtkObj `0x08` row 4, stops at holes, 10t/panel, SFX `0x7A`). | Act `0x080B12D4` (`fam=5 sub=0`, timer `0x1A`=26t) → AtkObj `0x08` `0x080D68E4` (table `0x080D68A0` row 4: `10-00` anim 2, 10t). Shield spr **`0c-03`** anim 0; wave spr **`10-00`** anim 2; SFX `0x7A` (wave). |
| `snake` | `0x083` | `chips/trap/init.luau` | **P0** | Saito forces `math.max(1, holes)` (deals damage even with **0 holes**) and fires instant dimming hitboxes on the opponent with `WOOD_FX`. ROM spawns **0 snakes if 0 holes** on user's side; each hole spawns an actual flying **Snake projectile** (AtkObj `0x3E`, up to 3 active concurrently, homing on opponent row). | Dim `23` `0x080FA094` → AtkObj `0x3F` `0x080DD978` → AtkObj `0x3E` `0x080DD75C` (spawner `0x080DD954`, target `0x080DDB48`). Spr **`10-24`** anim 0; SFX **`0xD2`**. |
| `prism` | `0x08E` | `chips/trap/init.luau` | **P0** | Saito places an instant stationary **300 HP** `ice` wall (`600t` life) with **NO parabolic throw** and **NO damage reflection/splash mechanic**. ROM throws Prism (`0c-2f`, `speed=0x40000`, `grav=-0x3C00`) 3 panels ahead, landing as an **indestructible (`0xFFFF` HP)** obstacle (`0c-2a`, **`360t`** life) that absorbs any damage hit and **splashes that damage to all 8 surrounding panels**! | Act `0x080B3C2E` (`fam=48`) → AtkObj `0x4F` `0x080DFD84` (throw) → AtkObj `0x4E` `0x080DFAF4` (stationary, `0x0168`=360t, `0xFFFF` HP), reflect `0x080DFC3E` → `0x080DFCEC`. Throw spr **`0c-2f`**, obstacle spr **`0c-2a`**; SFX `0x7E` (land + 15t shake), `0x76` (expire). |
| `pawn` | `0x097` | `chips/trap/init.luau` | **P0** | Saito spawns an **invincible non-obstacle** that **auto-slashes every 45t** without player input. ROM places a **10 HP destructible obstacle** (`1200t` life) 1 panel ahead that **slashes ONLY when user presses A** (`[player_extra+0x22] & 1`), firing LongSwrd (`0x080D678A`) with 90t cooldown. | Dim `53` `0x080FC4E8` → AtkObj `0x73` `0x080E51D4`, table `0x080E51BC` row 0 (`wait=1 anim=0 hp=10 life=1200`), A-press `0x080E536A`. Spr **`10-3d`** anim 0; SFX `0x92` (spawn), `0x76` (destroyed). |
| `knight` | `0x098` | `chips/trap/init.luau` | **P0** | Saito omits the chess obstacle entirely and fires an **instant dimming hitbox directly on the opponent's panel**. ROM places a **300 HP chess obstacle** (`6000t` life) 1 panel ahead that **hops forward in 50t parabolic arcs** tracking opponent's row (`0x080E5526`), slamming for 150 dmg and cracking panels (`0x0800B568`). | Dim `53` `0x080FC4E8` → AtkObj `0x73` `0x080E51D4`, table `0x080E51BC` row 1 (`wait=40 anim=6 hp=300 life=6000`), hop/slam `0x080E53C2..0x080E54C2` (`r4=0x090A0901`, 20t shake). Spr **`10-3d`** anim 6; SFX `0x92` (spawn), `0x7B` + `0x134` (slam), `0x76` (destroyed). |
| `rook` | `0x099` | `chips/trap/init.luau` | **P2** | HP (500), lifetime (1800t), stationary obstacle behavior, and sprite `10-3d` anim 4 match ROM. Minor presentation diff: spawn SFX uses `0x93`/`0x127` instead of ROM `0x92` + SimpleEffect `0x03` (`14-09` anim 1). | Dim `53` `0x080FC4E8` → AtkObj `0x73` `0x080E51D4`, table `0x080E51BC` row 2 (`wait=1 anim=4 hp=500 life=1800`), update `0x080E54CC` (no-op). Spr **`10-3d`** anim 4; SFX **`0x92`** (spawn), `0x76` (destroyed). |
| `hole` | `0x0A1` | `chips/trap/init.luau` | **P0** | Saito completely fabricated a panel-breaking attack that breaks 3 panels around the opponent and deletes enemy obstacles. ROM spawns a **1 HP DarkHole obstacle** (`10-42`) 1 panel ahead that waits 300t, then opens for 100t with SFX `0x8F`, setting battle flag bit `0x16+side` (`0x080012B4`, checked at `0x080B595C` for Dark/Hole-dependent effects). | Dim `44` `0x080FB72C` → AtkObj `0x6C` `0x080E37B0`, open `0x080E3904..0x080E3950` (`0x080012B4(0x16+side)`), check `0x080B595C`. Spr **`10-42`** (anim 0/1/2); SFX **`0x8F`** (open). |
| `repair` | `0x09F` | `chips/trap/init.luau` | **P1** | Saito only repairs `broken` and `cracked` panels instantly without flicker. ROM runs the **64-tick** stage flicker effect (`EffectObj 0x07`, row 0) resetting **ALL non-missing panels on user's side** (including Lava/Poison/Ice/Grass/Sand/Metal) to Normal (`panel=2`), with SFX `0x10E` (every 4t) and `0x10F` (finish). | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 0 (`panel=2 mode=0`, req `0x0`/`0x20`, forbid `0x8020`/`0x8000`). 64t flicker; SFX **`0x10E`** (tick), **`0x10F`** (finish). |
| `antinavi` | `0x0BE` | `chips/anti/init.luau` + `rules/init.luau` | **P0** | Saito only **cancels** the opponent's Navi chip with a puff (`rules/init.luau:159-163`). ROM **steals and reverses the Navi chip against the opponent** (`eor side, 1`, swaps owner to defender, displays `AntiNavi` banner `0xBE`, executes stolen Navi!). Arming SFX in Saito is `0x127` (ROM: **`0x91`** `invisible`). | Dim `32` `0x080FAA0C` (arm SFX `0x91`, trap `0x0C`) → trigger `0x08014D10` + `0x08008F84..0x0800904C` (`0x080FE2C2`). Trigger FX `14-05` anim 7 pal 2 (SimpleEff `0x2011`) + `00-00` anim 9 (`0x1B`); SFX `0x91` (arm), **`0x127`** (trigger). |
| `antidmg` | `0x0BF` | `chips/anti/init.luau` + `rules/init.luau` | **P1** | Saito grants **90t of `flash` + `invisible`** after counter, plays `0x91` + `small-puff`, and arms with `0x127`. ROM cancels hit, enters `PlayerAction 0x13` (MegaMan disappears with explosion puffs `0x080EE26E` and SFX **`0xB6`**, rises to z=48, throws shuriken for stored damage — **NO post-counter 90t invisibility**), and arms with **`0x91`**. | Dim `32` `0x080FAA0C` (arm SFX `0x91`, trap `0x04`) → check `0x080B4FFA` (`[coll+0x2C] & 0xFE000000`) → Act `0x13` `0x080B2088..0x080B2168`. SFX **`0x91`** (arm), **`0xB6`** (vanish). |
| `antiswrd` | `0x0C0` | `chips/anti/init.luau` + `rules/init.luau` | **P0** | Saito spawns **3 simultaneous `regions.wide` hitboxes directly on the opponent's panel on 1 tick** (instant unavoidable 300 dmg anywhere on field!). ROM cancels sword hit (`[coll+0x2C] & 0x08`), plays **`0x109`** + **`0x75`**, spawns attached sword `0c-00`, and fires **3 sequential SonicBoom waves** (`0x080DBED8`, 8 px/f) from MegaMan! Arming SFX also wrong (`0x127` vs `0x91`). | Dim `32` `0x080FAA0C` (arm SFX `0x91`, trap `0x08`) → check `0x080B50C8` → Act `0x2E` `0x080B3A40` (3x `0x080DBED8`, speed `0x80000`). SFX **`0x91`** (arm), **`0x109`** + **`0x75`** (counter). |
| `antirecv` | `0x0C1` | `chips/anti/init.luau` + `rules/init.luau` | **P0** | Saito hardcodes a **flat 150-dmg hitbox** and only triggers on `chip.family == "recovery"` (misses Roll/Team2/etc. heals), with `hit-bomb` (`0x76`). ROM hooks **ANY heal** (`0x080ADB04`) and deals **2× the heal amount** (`lsl r0, #1`) directly via `0x080ADA00` (e.g., Recov300 → **600 dmg**!), with SimpleEffect `0x2011` + 2 explosions (`0x080EE26E`) + SFX **`0x127`**. Arming SFX also wrong (`0x127` vs `0x91`). | Dim `32` `0x080FAA0C` (arm SFX `0x91`, trap `0x10`) → trigger `0x080ADB16..0x080ADB7E` (`lsl r0, r0, #1`, `0x080ADA00`). FX `14-05` anim 7 pal 2 (`0x2011`) + 2× `0x080EE26E`; SFX **`0x91`** (arm), **`0x127`** (trigger). |
| `barrier` | `0x0AD` | `chips/barrier/init.luau` + `lib/barriers.luau` | **P0** | Saito marks `barrier` as **`"dimming"`** and runs the EXE5/6 dimming controller (screen dim, time freeze, chip banner). In BN3 ROM, Barrier is a **NON-DIMMING instant action** (`fam=3 sub=1`, `b11=0x02`)! (10 HP, sprite `0c-04` anim 4 pal 0, SFX `0xCE` match.) | Act `0x080B117C` (`fam=3 sub=1`, `b11=0x02`), visual table `0x080EDFF4` row 1. Spr `0c-04` anim 4 pal 0; SFX `0xCE`. |
| `barr100` | `0x0AE` | `chips/barrier/init.luau` + `lib/barriers.luau` | **P0** | Same as `barrier`: wrongly marked **`"dimming"`** in Saito; ROM (`fam=3 sub=2`, `b11=0x02`) is **NON-DIMMING**! (100 HP, `0c-04` anim 4 pal 1, SFX `0xCE` match.) | Act `0x080B117C` (`fam=3 sub=2`, `b11=0x02`), visual table `0x080EDFF4` row 2. Spr `0c-04` anim 4 pal 1; SFX `0xCE`. |
| `barr200` | `0x0AF` | `chips/barrier/init.luau` + `lib/barriers.luau` | **P0** | Same as `barrier`: wrongly marked **`"dimming"`** in Saito; ROM (`fam=3 sub=3`, `b11=0x02`) is **NON-DIMMING**! (200 HP, `0c-04` anim 4 pal 2, SFX `0xCE` match.) | Act `0x080B117C` (`fam=3 sub=3`, `b11=0x02`), visual table `0x080EDFF4` row 3. Spr `0c-04` anim 4 pal 2; SFX `0xCE`. |
| `aura` | `0x0B0` | `chips/barrier/init.luau` + `lib/barriers.luau` | **P0** | Same as `barrier`: wrongly marked **`"dimming"`** in Saito; ROM (`fam=3 sub=6`, `b11=0x02`) is **NON-DIMMING**! (Threshold 100, 3000t timer, `0c-04` anim 0 pal 0, SFX `0xCE` match.) | Act `0x080B117C` (`fam=3 sub=6`, `b11=0x02`), visual table `0x080EDFF4` row 6 (`z=-4`, `0c-04` anim 0 pal 0). SFX `0xCE`. |
| `lifeaura` | `0x0C9` | `chips/barrier/init.luau` + `lib/barriers.luau` | **P0** | Same as `barrier`: wrongly marked **`"dimming"`** in Saito; ROM (`fam=3 sub=7`, `b11=0x02`) is **NON-DIMMING**! (Threshold 200, 3000t timer, `0c-04` anim 0 pal 1, SFX `0xCE` match.) | Act `0x080B117C` (`fam=3 sub=7`, `b11=0x02`), visual table `0x080EDFF4` row 7 (`z=-4`, `0c-04` anim 0 pal 1). SFX `0xCE`. |
| `mole1` | `0x0A2` | `chips/stealth/init.luau` | **P1** | Saito duration is **240t** (ROM: **480t** `0x01E0`), wrongly sets `invisible` + `flash` in addition to `submerged`, and uses `small-puff` instead of SimpleEffect `0x25` (`0c-14` anim 0). | Dim `3` `0x080F8D80`, durations `0x080F8E10` (`480/720/960`), status `0x080B580C` (bit `0x80` submerged only). FX **`0c-14`** anim 0 (`0x25`); SFX `0x91`. |
| `mole2` | `0x0A3` | `chips/stealth/init.luau` | **P1** | Saito duration is **360t** (ROM: **720t** `0x02D0`), wrongly sets `invisible` + `flash`, and uses `small-puff`. | Dim `3` `0x080F8D80`, `0x080F8E10` (`720t`), `0x080B580C`. FX **`0c-14`** anim 0; SFX `0x91`. |
| `mole3` | `0x0A4` | `chips/stealth/init.luau` | **P1** | Saito duration is **480t** (ROM: **960t** `0x03C0`), wrongly sets `invisible` + `flash`, and uses `small-puff`. | Dim `3` `0x080F8D80`, `0x080F8E10` (`960t`), `0x080B580C`. FX **`0c-14`** anim 0; SFX `0x91`. |
| `shadow` | `0x0A5` | `chips/stealth/init.luau` + `rules/init.luau` | **P0** | Saito sets **`user:set_status_timer("flash", 360)`** (`stealth/init.luau:31`) alongside `shadow_ticks = 360`. Because `rules/status.luau:47` uses `flash_timer_first`, setting `flash` makes the user **invulnerable even to non-piercing Sword attacks** (breaking Shadow's sole weakness to Swords)! Also uses `small-puff` instead of `0c-14` (`0x25`). | Dim `27` `0x080FA4F8`, duration `0x080FA5D0` (`360t`), status `0x080B57F4` (bit `0x20` shadow ONLY — NOT hit-flash!). FX **`0c-14`** anim 0 (`0x25`); SFX `0x91`. |
| `invis` | `0x0A0` | `chips/invis/init.luau` + `controller.luau` | **P0** | Saito marks `invis` as **`"dimming"`** and runs the EXE5/6 dimming controller. In BN3 ROM, Invis is a **NON-DIMMING instant action** (`fam=17`, `b11=0x02`, 360t invis, SimpleEffect `0x13` = `0c-06`, SFX `0x91`, 10t wait)! | Act `0x080B1FB6` (`fam=17`, `b11=0x02`), duration `0x080B2020` (`0x0168`=360t), `0x080B57DA`. FX `0c-06` anim 0 (`0x13`); SFX `0x91`. |
| `poismask` | `0x06C` | `chips/poison/init.luau` | **P0** | Saito fabricated an instant `buster_shoot` 100-dmg hitbox with **no mask obstacle, no A-hold channeling, and no poison**. ROM spawns a **channeling mask obstacle** (`10-10` pal 0, hold A up to `1200t`, min `56t`) emitting poison clouds (`0x080DC518`, pattern 8, SFX `0xF5`) and dealing 1-dmg poison ticks every 5t (`0x080DC49A`). | Act `0x080B398A` (`fam=44 sub=0`) → AtkObj `0x34` `0x080DC320`, table `0x080DC31C` (`period=5 pattern=8`), clouds `0x080DC518`. Spr **`10-10`** pal 0; SFX **`0xF5`**. |
| `poisface` | `0x06D` | `chips/poison/init.luau` | **P0** | Same as `poismask`: fabricated instant 100-dmg hitbox. ROM spawns channeling mask obstacle (`10-10` pal 1, wider poison pattern 17, hold A up to `1200t`). | Act `0x080B398A` (`fam=44 sub=1`) → AtkObj `0x34` `0x080DC320`, table `0x080DC31C` (`period=5 pattern=17`). Spr **`10-10`** pal 1; SFX **`0xF5`**. |
| `anubis` | `0x0CC` | `chips/poison/init.luau` | **P0** | Saito spawns an **indestructible non-obstacle** (NO HP, NO collision) lasting only **480t**, draining 1 HP every 3t, with no sky-drop/shake. ROM drops a **300 HP destructible statue obstacle** (`0c-29` pal 1 anim 1, **`6000t`** life `0x1770`, falls from z=160 at 8 px/f, SFX `0x7E` + 30t shake) draining **1 HP/tick** (`0x080DF926`, `period=1` from pal byte) + poison clouds every 16t (`0x080EE26E`). | Dim `33` `0x080FAB04` → AtkObj `0x4C` `0x080DF75C`, table `0x080DF74C` row 1 (`0c-29` pal 1 anim 1, HP=300), drain `0x080DF8FA` (`0x080DF960`). Spr **`0c-29`** pal 1 anim 1; SFX **`0x7E`** (land), `0x76` (destroyed). |
| `wind` | `0x08F` | `chips/wind/init.luau` | **P0** | Saito spawns an **invincible non-obstacle** (NO HP, NO collision) with wrong sprite `0c-23` (`timebomb`!), only **480t** life, and manually moves enemy without wind AtkObj or blowing away barriers. ROM places an **80 HP destructible obstacle** (`04-0b` pal 0, **`6000t`** life, SFX `0xEF`) spawning wind lines (`0x080EEAD2`) + Wind push AtkObj `0x1E` (`10-07`, pushes enemies & blows away barriers/auras!). | Dim `5` `0x080F8F3C` → AtkObj `0x2D` `0x080DB4F4`, table `0x080DB4EC` row 0 (`hp=80`), wind EffObj `0x0B` `0x080EEAD2`, push AtkObj `0x1E` `0x080D926C`. Spr **`04-0b`** pal 0; SFX **`0xEF`** (blow), `0x76` (destroyed). |
| `fan` | `0x090` | `chips/wind/init.luau` | **P0** | Same as `wind`: invincible non-obstacle, wrong sprite `0c-39` instead of **`04-0b` pal 1**, only **480t** life (ROM: **6000t**, **80 HP**), no wind AtkObj `0x1E` / barrier strip. | Dim `5` `0x080F8F3C` → AtkObj `0x2D` `0x080DB4F4`, table `0x080DB4EC` row 1 (`hp=80`), AtkObj `0x1E` `0x080D926C`. Spr **`04-0b`** pal 1; SFX **`0xEF`**, `0x76`. |
| `rockcube` | `0x08D` | `chips/rockcube/init.luau` | **P2** | Core mechanics match ROM (dimming, 1 panel ahead, 200 HP obstacle, 200 push dmg). Presentation diffs: Saito uses sprite `0c-2d` (`rockcube`) anim 9 + `grab-shot-land` (`0x79`) place SFX + `panel-crack` (`0x7B`) break SFX; ROM uses **`10-08` anim 2** + place SFX **`0x93`** (`fade`) + break SFX `0x76`. | Dim `2` `0x080F8CB8` (place SFX `0x93`) → AtkObj `0x12` `0x080D7AA6`, table `0x080D7A88` row 1 (`anim=2 dmg=200 hp=200 life=6000`). Spr **`10-08`** anim 2; SFX **`0x93`** (place), `0x76` (break). |
| `metagel1` | `0x085` | `chips/metagel/init.luau` | **P2** | Core mechanics match ROM (dimming, panel search/steal, 90 Aqua dmg). Presentation diffs: Saito uses sprite `10-0c` (`bubble`, no palette swap), drop SFX `0x78`, land SFX `0x79`. ROM uses **`04-1c` pal 0** anim 2→3→0, drop SFX **`0xDA`**, land SFX **`0xAC`**. | Dim `22` `0x080F9F78` → AtkObj `0x3D` `0x080DD580` (`0x080DD6D0` checks). Spr **`04-1c`** pal 0 anim 2/3/0; SFX **`0xDA`** (drop), **`0xAC`** (land). |
| `metagel2` | `0x086` | `chips/metagel/init.luau` | **P2** | Same as `metagel1` (130 Aqua dmg): wrong sprite `10-0c` (no palette), wrong drop/land SFX (`0x78`/`0x79` vs `0xDA`/`0xAC`). | Dim `22` `0x080F9F78` → AtkObj `0x3D` `0x080DD580`. Spr **`04-1c`** pal 1 anim 2/3/0; SFX **`0xDA`**, **`0xAC`**. |
| `metagel3` | `0x087` | `chips/metagel/init.luau` | **P2** | Same as `metagel1` (170 Aqua dmg): wrong sprite `10-0c` (no palette), wrong drop/land SFX (`0x78`/`0x79` vs `0xDA`/`0xAC`). | Dim `22` `0x080F9F78` → AtkObj `0x3D` `0x080DD580`. Spr **`04-1c`** pal 2 anim 2/3/0; SFX **`0xDA`**, **`0xAC`**. |
| `panlgrab` | `0x081` | `chips/grab/init.luau` + `lib/grab.luau` | **OK** | Matches ROM: dimming, drops `0c-0e` anim 0 onto frontmost enemy panel in user's row, steals panel for 1800t (30s) or deals 10 dmg if occupied, SFX `0x78` / `0x79`. | Dim `0` `0x080F8A20` → AtkObj `0x0B`. Spr `0c-0e` anim 0; SFX `0x78` (drop), `0x79` (land). |
| `areagrab` | `0x082` | `chips/grab/init.luau` + `lib/grab.luau` | **OK** | Matches ROM: dimming, drops `0c-0e` anim 0 across frontmost enemy column, steals column for 1800t or deals 10 dmg per occupied panel, SFX `0x78` / `0x79`. | Dim `0` `0x080F8A20` → AtkObj `0x0B`. Spr `0c-0e` anim 0; SFX `0x78` (drop), `0x79` (land). |
| `holypanl` | `0x0B2` | `chips/stage/init.luau` | **P2** | Functional panel change matches ROM (`mode=8` front panel → Holy). Presentation diffs: Saito flickers **30t** with `0x7B`/`0x92`; ROM flickers **64t** (`0x40`, every 8t) with SFX **`0x10E`** (every 4t) and **`0x10F`** (finish). | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 6 (`panel=9 mode=8`), selector `0x080EE556`. 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `lavastge` | `0x0B3` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (all non-missing panels → Lava). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 1 (`panel=8 mode=0`). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `icestage` | `0x0B4` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (all non-missing panels → Ice). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 2 (`panel=7 mode=0`). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `grassstg` | `0x0B5` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (all non-missing panels → Grass). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 3 (`panel=6 mode=0`). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `sandstge` | `0x0B6` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (all non-missing panels → Sand). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 4 (`panel=10 mode=0`). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `metlstge` | `0x0B7` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (all non-missing panels → Metal). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 5 (`panel=5 mode=0`). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `snctuary` | `0x0B8` | `chips/stage/init.luau` | **P2** | Functional stage change matches ROM (user's side non-missing panels → Holy). Presentation diffs: 30t vs **64t**, SFX `0x7B`/`0x92` vs **`0x10E`/`0x10F`**. | Dim `4` `0x080F8E48` → EffObj `0x07` `0x080EE3EC`, table `0x080EE34C` row 7 (`panel=9 mode=0`, own side). 64t flicker; SFX **`0x10E`**, **`0x10F`**. |
| `geddon1` | `0x08A` | `chips/geddon/init.luau` + `controller.luau` | **OK** | Matches ROM: dimming, 8t period, cracks solid non-cracked panels & breaks unoccupied cracked panels, SFX `0x7B`. | Dim `25` `0x080FA2E0` → EffObj `0x12` `0x080EF0F8`, table `0x080EF0B0` row 0 (`0x0800B5C0`). SFX `0x7B`. |
| `geddon2` | `0x08B` | `chips/geddon/init.luau` + `controller.luau` | **OK** | Matches ROM: dimming, 8t period, breaks unoccupied solid panels to holes & cracks occupied solid panels, SFX `0x7B`. | Dim `25` `0x080FA2E0` → EffObj `0x12` `0x080EF0F8`, table `0x080EF0B0` row 1 (`0x0800B6F0`). SFX `0x7B`. |
| `geddon3` | `0x08C` | `chips/geddon/init.luau` + `controller.luau` | **P2** | Panel conversion matches ROM (all non-hole panels → Swamp/Poison, 8t period). Presentation diff: Saito plays `panel-poison` (`0x10F`); ROM plays **`0xB6`** (`arrive`). | Dim `25` `0x080FA2E0` → EffObj `0x12` `0x080EF0F8`, table `0x080EF0B0` row 2 (`0x0800B73C`). SFX **`0xB6`**. |
| `recov10` | `0x079` | `chips/recov/init.luau` + `recov.luau` | **P1** | Heal amount (10), FX `0c-06` anim 0, and SFX `0x72` match ROM, but Saito sets **`lockout = 30`** (2× ROM's **15t** `0x0F` action wait with no extra lockout). | Act `0x080B1A4C` (`fam=11 sub=5`, heals `sub*2`, wait `0x0F`=15t), heal `0x080ADB04`. FX `0c-06` anim 0 (`0x0D`); SFX `0x72`. |
| `recov30` | `0x07A` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (30 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=15`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov50` | `0x07B` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (50 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=25`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov80` | `0x07C` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (80 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=40`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov120` | `0x07D` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (120 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=60`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov150` | `0x07E` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (150 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=75`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov200` | `0x07F` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (200 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=100`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `recov300` | `0x080` | `chips/recov/init.luau` + `recov.luau` | **P1** | Same as `recov10` (300 HP): `lockout = 30` vs ROM 15t action wait. | Act `0x080B1A4C` (`fam=11 sub=150`, wait `0x0F`=15t). FX `0c-06` anim 0; SFX `0x72`. |
| `slogauge` | `0x09D` | `chips/gauge/init.luau` + `lib/gauge_speed.luau` | **OK** | Matches ROM: dimming, sets custom gauge rate to `0x10` (half), 70t wait, warning SFX `0xE3` every 16t. | Dim `28` `0x080FA608` (rate table `0x080FA6B0`: `0x10`/`0x40`, wait `0x46`=70t), warning `0x0800A0F8` (SFX `0xE3` / 16t). |
| `fstgauge` | `0x09E` | `chips/gauge/init.luau` + `lib/gauge_speed.luau` | **OK** | Matches ROM: dimming, sets custom gauge rate to `0x40` (double), 70t wait, warning SFX `0xE3` every 16t. | Dim `28` `0x080FA608` (rate `0x40`, wait `0x46`=70t), warning `0x0800A0F8` (SFX `0xE3` / 16t). |
| `airshoes` | `0x0A8` | `chips/plus/init.luau` | **P2** | Permanent `air_shoes` dimming effect matches ROM. Presentation diffs: Saito plays `HEAL_FX` (`0c-06`) + `damage-bonus` (`0xED`) with 20t wait; ROM spawns SimpleEffect `0x25` (**`0c-14`** anim 0) + SFX **`0x91`** (`invisible`) with 10t+30t=40t wait. | Dim `13` `0x080F95BC` (sets `[player+0x2E] \|= 0x4000`, 10t+30t). FX **`0c-14`** anim 0 (`0x25`); SFX **`0x91`**. |
| `atk-10` | `0x0C3` | `chips/plus/init.luau` + `lib/plus.luau` | **OK** | Matches ROM: custom-screen modifier adding +10 to any attacking chip (`[chip+0x0B] & 0x05`). | Custom modifier check `0x0800F9CC` (`cmp #0xC3`, `+10`), fallback act `0x080B226E` (`fam=21`). |
| `fire-30` | `0x0C4` | `chips/plus/init.luau` + `lib/plus.luau` | **P1** | Saito uses generic `modifier = "attack_plus"` (`builder.rs` applies to **ANY** `HAS_DAMAGE` chip!). ROM strictly requires target chip element `[chip+0x06] == 2` (**Fire only**)! | Custom modifier check `0x0800F9E0` (`cmp #0xC4`, checks `[r7+0x06] == 0x02` Fire → `+30`). |
| `aqua-30` | `0x0C5` | `chips/plus/init.luau` + `lib/plus.luau` | **P1** | Same as `fire-30`: Saito boosts ANY damaging chip; ROM requires `[chip+0x06] == 3` (**Aqua only**)! | Custom modifier check `0x0800F9FA` (`cmp #0xC5`, checks `[r7+0x06] == 0x03` Aqua → `+30`). |
| `elec-30` | `0x0C6` | `chips/plus/init.luau` + `lib/plus.luau` | **P1** | Same as `fire-30`: Saito boosts ANY damaging chip; ROM requires `[chip+0x06] == 1` (**Elec only**)! | Custom modifier check `0x0800FA2E` (`cmp #0xC6`, checks `[r7+0x06] == 0x01` Elec → `+30`). |
| `wood-30` | `0x0C7` | `chips/plus/init.luau` + `lib/plus.luau` | **P1** | Same as `fire-30`: Saito boosts ANY damaging chip; ROM requires `[chip+0x06] == 4` (**Wood only**)! | Custom modifier check `0x0800FA14` (`cmp #0xC7`, checks `[r7+0x06] == 0x04` Wood → `+30`). |
| `navi-20` | `0x0C8` | `chips/plus/init.luau` + `lib/plus.luau` | **OK** | Matches ROM: custom-screen modifier adding +20 to Navi chips (`[chip+0x0B] & 0x04`). | Custom modifier check `0x0800F9B8` (`cmp #0xC8`, `[r7+0x0B] & 0x04` → `+20`). |
| `atk-30` | `0x0CD` | `chips/plus/init.luau` + `lib/plus.luau` | **OK** | Matches ROM: Mega-class modifier adding +30 to any attacking chip. | Custom modifier check `0x0800FA46+` (`0xCD` → `+30`), fallback act `0x080B226E`. |
| `navi-40` | `0x0CE` | `chips/plus/init.luau` + `lib/plus.luau` | **OK** | Matches ROM: Mega-class modifier adding +40 to Navi chips. | Custom modifier check `0x0800FA46+` (`0xCE` → `+40`), fallback act `0x080B226E`. |
| `fullcust` | `0x0D3` | `chips/plus/init.luau` | **P0** | Saito marks `fullcust` as **`"dimming"`** and plays `gauge-full` SFX in a dimming controller. In BN3 ROM, FullCust is a **NON-DIMMING instant action** (`fam=45`, `b11=0x02`) that immediately writes `0x4000` to the custom gauge and ends the action! | Act `0x080B3A10` (`fam=45`, `b11=0x02`) → `0x08010B20` (writes `0x4000` to `[r2+0x2C]`) + `EndPlayerChipAction`. |
| `copydmg` | `0x0C2` | `chips/copydmg/init.luau` | **P1** | Saito marks target for **120 ticks** with `14-05` **anim 7** and `trap-mark` (**`0x127`**). ROM marks target for **90 ticks** (`0x5A`) with `14-05` **anim 11 (`0x0B`)** and SFX **`0x98`**! | Act `0x080B18A0` (`fam=9`, `b11=0x20`) → AtkObj `0x04` `0x080D61EC` (`timer=0x5A`=90t, anim `0x06`→`0x0B`). Spr `14-05` anim **`11`** (`0x0B`); SFX **`0x98`**. |
| `invalid` | `0x000` | `chips/invalid/init.luau` | **OK** | Non-library sentinel role `chips.invalid`; no-op `plus.spawner(0)` matches BN3 dummy chip `0x000` entry. | `ChipDataTable[0]` @ `0x08011510` (`fam=0`, `sub=0`, `pow=0`). |
| `zeroed` | `—` | `chips/zeroed/init.luau` | **OK** | Internal engine fallback role `chips.zeroed` (40-dmg Cannon-like placeholder for zeroed/corrupted slots). | Engine sentinel (no standalone ROM chip ID). |

---

#### Detailed Findings Notes

##### 1. Non-Dimming Chips Wrongly Made Dimming (7 P0 Chips)
A pervasive port artifact from EXE5/6 `@exelib` controllers is flagging BN3 **non-dimming** support chips (`rom_b11 == 0x02`) as `"dimming"`:
- **`barrier`, `barr100`, `barr200`, `aura`, `lifeaura`** (`0x0AD`–`0x0B0`, `0x0C9`): All 5 route to `PlayerAction[3]` (`0x080B117C`), which immediately writes barrier type (`0x04`..`0x1C`) and power/timer (`10/100/200/3000/3000`) to collision data and calls `EndPlayerChipAction` with **no dimming, no time freeze, and no chip banner**.
- **`invis`** (`0x0A0`): Routes to `PlayerAction[17]` (`0x080B1FB6`), sets 360t invisibility (`0x080B57DA`), spawns SimpleEffect `0x13` (`0c-06`), plays SFX `0x91`, waits 10 ticks, and ends action without dimming.
- **`fullcust`** (`0x0D3`): Routes to `PlayerAction[45]` (`0x080B3A10`), calls `0x08010B20` (writes `0x4000` to custom gauge) and immediately ends action without dimming.

##### 2. Missing / Fabricated Field Objects & Obstacles (15 P0 Chips)
- **`blkbomb1..3`** (`0x01B`–`0x01D`): ROM `AttackObj 0x42` (`0x080DDFCC`) is a parabolic throw (`0x08001008`) that lands as a 100-HP, 6000-tick obstacle and checks `[coll+0x1E]` (fire element damage) at `0x080DE21E`. It **never** self-detonates on a 150-tick fuse.
- **`prism`** (`0x08E`): ROM `AttackObj 0x4F` (`0x080DFD84`, `0c-2f`) throws 3 panels ahead and spawns `AttackObj 0x4E` (`0x080DFAF4`, `0c-2a`, `HP=0xFFFF`, `360t`), which accumulates any damage taken into `[r5+0x64]` and splashes it across all 8 adjacent panels via `0x080DFCEC`. Saito's Prism is a plain 300-HP destructible wall with no throw and no reflection.
- **`pawn` & `knight`** (`0x097`, `0x098`): ROM `AttackObj 0x73` (`0x080E51D4`, table `0x080E51BC`) implements all three chess pieces as destructible obstacles (`10-3d`):
  - **Pawn**: 10 HP, 1200t life; attacks ONLY on player A-press (`0x080E536A`: `[player_extra+0x22] & 1`), 90t cooldown. Saito's Pawn is invincible and auto-slashes every 45t.
  - **Knight**: 300 HP, 6000t life; hops forward in 50t sine-arc jumps tracking opponent's row (`0x080E53C2` / `0x080E5526`), slamming for 150 dmg and cracking panels (`0x0800B568`). Saito has no Knight obstacle at all — just an instant dimming hitbox on the enemy.
- **`snake`** (`0x083`): ROM `AttackObj 0x3F` (`0x080DD978`) scans user-side holes and spawns flying `AttackObj 0x3E` (`0x080DD75C`, `10-24`, SFX `0xD2`) projectiles (0 holes = 0 snakes). Saito guarantees at least 1 hit and fires instant dimming hitboxes.
- **`crsshld1..3`** (`0x068`–`0x06A`): ROM guards for 26t (`0x080B12D4`) and on block launches flying projectile `AttackObj 0x44` (`0x080DE380`, `0c-03` anim 9→10, 12 px/f, SFX `0x111`). Saito guards 42t and drops an instant `regions.plus` hitbox on the enemy's panel.
- **`team1` & `team2`** (`0x084`, `0x0A9`): ROM `AttackObj 0x8D` (`0x080E8E14`, `04-26`, 40 HP) spawns an ally obstacle 1 panel ahead (`300t` / `600t` life). Team1 fires `AttackObj 0x49` (`10-29`, 10 dmg) when player takes hits; Team2 heals **500 HP** (`0x080E8FE8`) whenever player HP ≤ MaxHP/4 (60t cooldown). Saito fabricated an instant 10-dmg hit for Team1 and an instant 50 HP + 120t invisibility buff for Team2.
- **`hole`** (`0x0A1`): ROM `AttackObj 0x6C` (`0x080E37B0`, `10-42`, 1 HP) places a DarkHole obstacle 1 panel ahead that opens after 300t for 100t (SFX `0x8F`) to set battle flag `0x16+side` (`0x080012B4`, checked at `0x080B595C`). Saito fabricated a 3-panel breaking/obstacle-deleting attack.
- **`poismask` & `poisface`** (`0x06C`, `0x06D`): ROM `AttackObj 0x34` (`0x080DC320`, `10-10` pal 0/1) is an A-held channeling obstacle (56t–1200t) that spreads poison clouds (`0x080DC518`, patterns 8 / 17, SFX `0xF5`) and deals 1-dmg poison ticks every 5t. Saito fabricated an instant 100-dmg `buster_shoot` hit.
- **`anubis`** (`0x0CC`): ROM `AttackObj 0x4C` (`0x080DF75C`, table `0x080DF74C` row 1) is a **300 HP destructible statue obstacle** (`0c-29` pal 1 anim 1, 6000t life) falling from z=160 (SFX `0x7E` + 30t shake) and draining 1 HP/tick. Saito's Anubis is invincible (no HP/collision), lasts 480t, and drains 1 HP/3t.
- **`wind` & `fan`** (`0x08F`, `0x090`): ROM `AttackObj 0x2D` (`0x080DB4F4`, table `0x080DB4EC`) places an **80 HP destructible obstacle** (`04-0b` pal 0/1, 6000t life, SFX `0xEF`) that spawns wind lines (`0x080EEAD2`) and Wind push/pull `AttackObj 0x1E` (`0x080D926C`, `10-07`) which pushes/pulls enemies AND blows away barriers/auras. Saito spawns invincible non-obstacles (`0c-23`/`0c-39`) lasting 480t without spawning wind hitboxes or stripping barriers.
- **`grabback` & `grabrvng`** (`0x088`, `0x089`): ROM `0x080F9DD0` counts stolen panels (spawning `0c-0e` anim 1 markers, **NEVER reclaiming panels**) and drops `stolen` sequential `AttackObj 0x3C` (`0x080DD450`, `0c-0e`, 32t fall, 30t spacing, 40/80 dmg each) on the opponent. Saito reclaims all panels and deals one lump-sum hit.

##### 3. Anti-Trap & Stealth Mechanics Mismatches (4 P0, 4 P1)
- **`antinavi`** (`0x0BE`, P0): ROM (`0x08008F84..0x0800904C`) steals and reverses the opponent's Navi chip back at them; Saito only cancels it with a puff.
- **`antiswrd`** (`0x0C0`, P0): ROM (`0x080B50C8` → `PlayerAction 0x2E` `0x080B3A40`) fires 3 sequential SonicBoom waves (`0x080DBED8`, 8 px/f) from MegaMan; Saito spawns 3 simultaneous `regions.wide` hitboxes directly on the enemy's panel on a single tick.
- **`antirecv`** (`0x0C1`, P0): ROM (`0x080ADB16`) triggers on ANY heal and deals **2× heal amount** directly via `0x080ADA00`; Saito hardcodes 150 dmg and only checks `chip.family == "recovery"`.
- **`antidmg`** (`0x0BF`, P1): ROM (`0x080B4FFA` → `PlayerAction 0x13` `0x080B2088`) vanishes during the jump/shuriken action (`0xB6` SFX, smoke puffs) without granting 90t of post-counter `flash` + `invisible`. All 4 Anti chips also use `0x127` (`trap-mark`) on arm instead of ROM **`0x91`** (`invisible`).
- **`shadow`** (`0x0A5`, P0): Saito sets `user:set_status_timer("flash", 360)` (`stealth/init.luau:31`). Under `rules/status.luau:47` (`flash_timer_first`), this confers full hit-flash invulnerability against non-piercing **Sword** attacks as well, breaking Shadow's core vulnerability.
- **`mole1..3`** (`0x0A2`–`0x0A4`, P1): Durations are halved (`240/360/480t` vs ROM `480/720/960t` at `0x080F8E10`), and Saito wrongly sets `invisible` + `flash` alongside `submerged` (`0x080B580C` sets bit `0x80` only).

##### 4. Elemental Plus Modifiers, Recovery Lockout, TimeBomb/Mine/Guard/Repair/CopyDmg (P1)
- **`fire-30`, `aqua-30`, `elec-30`, `wood-30`** (`0x0C4`–`0x0C7`, P1): ROM `0x0800F9E0..0x0800FA46` strictly checks target chip element `[chip+0x06]` (`2=Fire, 3=Aqua, 4=Wood, 1=Elec`); Saito assigns generic `modifier = "attack_plus"`, which in `crates/saito-battle/src/custom/builder.rs` boosts ANY `HAS_DAMAGE` chip regardless of element.
- **`recov10..300`** (`0x079`–`0x080`, P1): ROM `0x080B1A4C` waits 15t (`0x0F`) with no extra lockout; Saito sets `lockout = 30`.
- **`timebomb`** (`0x063`, P1): 100 HP vs ROM 50 HP (`0x080F9BEC`); 3×3 blast vs ROM full enemy area (`0x080DB04C`); 180t vs 210t; wrong sprite `0c-02` vs `0c-23` and SFX `0x127`/`0x76` vs `0x8A`/`0x8B`/`0xED`.
- **`mine`** (`0x064`, P1): 1200t vs ROM 6000t life; hidden banner `????` vs ROM visible `0x64` banner + 120t random panel cycling animation; wrong sprite `0c-02` vs `0c-24`.
- **`guard`** (`0x076`, P1): 36t vs ROM 26t window; floating hole-ignoring `reflector-shield` vs ROM ground-bound ShockWav (`10-00` anim 2, SFX `0x7A`); `HEAL_FX` vs attached shield `0c-03` anim 0.
- **`repair`** (`0x09F`, P1): Only fixes `cracked`/`broken` panels instantly; ROM (`0x080EE34C` row 0) resets ALL non-missing own-side panels (including Lava/Poison/Ice/Grass/Sand/Metal) to Normal via 64t flicker (`0x10E`/`0x10F`).
- **`copydmg`** (`0x0C2`, P1): 120t mark / `14-05` anim 7 / SFX `0x127` vs ROM **90t** (`0x5A`) / `14-05` **anim 11 (`0x0B`)** / SFX **`0x98`** (`0x080D61EC`).

##### 5. Presentation-Only Mismatches (14 P2 Chips)
- **7 Stage chips** (`holypanl`, `lavastge`, `icestage`, `grassstg`, `sandstge`, `metlstge`, `snctuary`): Saito flickers **30t** with `0x7B`/`0x92`; ROM (`EffectObj 0x07` `0x080EE3EC`) flickers **64t** with SFX **`0x10E`** (every 4t) and **`0x10F`** (finish).
- **`metagel1..3`**: Sprite `10-0c` (no palette swap) & SFX `0x78`/`0x79` vs ROM **`04-1c` pal 0/1/2** & SFX **`0xDA`/`0xAC`**.
- **`rockcube`**: Sprite `0c-2d` anim 9 & SFX `0x79`/`0x7B` vs ROM **`10-08` anim 2** & SFX **`0x93`/`0x76`**.
- **`geddon3`**: SFX `0x10F` (`panel-poison`) vs ROM **`0xB6`** (`arrive`).
- **`airshoes`**: FX `0c-06` + SFX `0xED` vs ROM SimpleEffect `0x25` (**`0c-14`** anim 0) + SFX **`0x91`**.
- **`rook`**: Spawn SFX `0x93`/`0x127` vs ROM **`0x92`** + SimpleEffect `0x03` (`14-09` anim 1).

##### 6. Unsigned Timer Check
- `me.timer` (`OverlayField::Timer`) is `u16` (`crates/saito-content-api/src/api.rs:194`, `bind.rs:311` casts via `as u16`). All Batch 3 dimming controllers (`anti`, `stealth`, `poison`, `wind`, `plus`, `trap`, `utility`) store `local t = me.timer - 1; me.timer = t; if t < 0 then dimming_chips.done(me) end`, checking the signed Lua local `t` rather than reading back the wrapped `me.timer` (`65535`). Thus none of these softlock on timer underflow, though `me.timer` momentarily stores `0xFFFF` on the final tick.


---

### 5.4 Group 4：`utility/init.luau` 综合置物、条件攻击与 9 张病毒芯片（52 张）

- **Target Module**: `/home/harp/projects/saito/content/exe3/chips/utility/init.luau` (52 chips, lines 1–1979)
- **Reference ROM**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US, `A6BE`)
- **Audit Scope**: All 52 chips defined in `utility/init.luau` (note: `nobeam1..3`, `repair`, and `hole` are not defined in this file).

#### Executive Summary

| Severity | Count | Chips |
| :--- | ---: | :--- |
| **P0** | **49** | `grabback`, `grabrvng`, `fanfare`, `discord`, `timpani`, `panic`, `rndmmetr`, `holemetr`, `shotmetr`, `lavacan1`, `lavacan2`, `lavacan3`, `volcano`, `airstrm1`, `airstrm2`, `airstrm3`, `totem1`, `totem2`, `totem3`, `spice1`, `spice2`, `spice3`, `crsshld1`, `crsshld2`, `crsshld3`, `geyser`, `shake1`, `shake2`, `shake3`, `condor`, `burning`, `fireratn`, `rockarm1`, `rockarm2`, `rockarm3`, `needler1`, `needler2`, `needler3`, `team1`, `team2`, `mettaur`, `bunny`, `spikey`, `swordy`, `jelly`, `mushy`, `momogra`, `killreye`, `scuttlst` |
| **P1** | **3** | `nrthwind`, `iceball`, `lavaball` |
| **P2** | **0** | — |
| **OK** | **0** | — |
| **Total** | **52** | 100% audited against `bn3.gba` |

##### Systemic Findings
1. **Missing Field Objects / Summons (21 chips)**: `fanfare`, `discord`, `timpani`, `rndmmetr`, `holemetr`, `shotmetr`, `totem1..3`, `shake1..3`, `fireratn`, `rockarm1..3`, `needler1..3`, `team1`, `team2` ALL place real field objects/obstacles/turrets on the panel in front of the user in `bn3.gba` (replacing any prior summon via `0x080B5712` and checking panel validity via `CheckPanelFlags`). Saito replaces nearly all of them with instant dimmed/non-dimmed effects or invincible stubs without HP, placement checks, or real AI loops.
2. **Missing Virus Summon Actors & Virus Breeder System (9 chips)**: All 9 Virus chips (`mettaur`, `bunny`, `spikey`, `swordy`, `jelly`, `mushy`, `momogra`, `killreye`, `scuttlst`) share a fabricated single-frame auto-hit stub (`VIRUS_CTRL`) with static hardcoded damage and zero virus sprites/actors. In ROM, Family `0x1F` (`0x0804752A`) validates Virus Breeder captures (`0x080076EA`), selects tier via weighted RNG (`0x08007794`), scales damage via BugFrag feed count (`0x0800781C`), and spawns dedicated `Type1` Virus Summon actors (`0x0800369C` jump table).
3. **Dimming Mismatch (3 chips)**: `spice1..3` are marked `"dimming"` in saito, whereas ROM `fam=0x2B` (`b11=0x01`) is **non-dimming** and spawns an async column-sweeping `atk 0x46` object.
4. **Inverted / Fabricated Mechanics (16 chips)**:
   - `grabback` / `grabrvng` erroneously reclaim stolen panels and deal 1 combined hit instead of `N` multi-hit falling strikes across all enemies without reclaiming panels.
   - `holemetr` breaks panels into holes (likely confused by the English localization name "HoleMetr"), whereas ROM `atk 0x0A` (`r7[5]=0`) never cracks or breaks panels—it sweeps enemy panels dropping 15 fire meteors.
   - `lavacan1..3` / `volcano` fail to consume own-side lava panels for `+10/panel` + pierce and instead turn the enemy's panel into lava (`volcano` invents `+` explosions instead of 3-row cannon shots).
   - `panic` ignores its mandatory self-Confused / Confusion-Bug activation condition.
   - `geyser` ignores its mandatory Hole-panel activation condition.
   - `condor` stands still and fires a wind fan instead of physically dashing across the row.
   - `burning` fires a single-frame T+row laser (`region_2a`) from user's own panel instead of a 2-stage `+` fire cross centered 1 panel ahead while intangible.
   - `airstrm1..3` fire 3 instant single-row hits instead of a 2-phase 3-row suction wind + 3 shuffled Tornadoes.
   - `crsshld1..3` teleport a homing `+` Break AoE onto the enemy instead of firing a straight single-panel Cross projectile down the row (and guard for 42f instead of 26f).

---

#### Complete 52-Chip Audit Table

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `grabback` | `0x088` | `utility/init.luau` | **P0** | **Saito**: Reclaims all stolen home panels (`alliance = me.alliance`) and deals a single combined `40 * stolen` hit to `enemy_pos` only. **ROM**: Does **NOT** reclaim panels. Counts stolen home panels (`x=1..3` / `4..6`), spawns FX `0x32` on each, plays SFX `0xE3`, then loops `stolen` times every 30f spawning `atk 0x3C` (falling fist from `z=0x01000000`) on **every** panel with an enemy, dealing `40` dmg per hit (multi-hit). | `fam=0x0C sub=0x0D`, `eff=0x15` `0x080F9DD0`, count `0x080F9EC4`, spawn `0x080F9F2A`, `atk 0x3C` `0x080DD450` (land `0x080DD504`). Sprite: `0c-0e` (`grab-shot`). SFX: `0xE3` (count), `0x78` (drop), `0x79` (land). |
| `grabrvng` | `0x089` | `utility/init.luau` | **P0** | **Saito**: Same as `grabback` (`80 * stolen` single combined hit + panel reclamation). **ROM**: Same as `grabback` with `80` dmg per falling strike (`stolen` strikes every 30f on all enemies); never reclaims panels. | `fam=0x0C sub=0x0E`, `eff=0x15` `0x080F9DD0`, `atk 0x3C` `0x080DD450`. Sprite: `0c-0e`. SFX: `0xE3`, `0x78`, `0x79`. |
| `nrthwind` | `0x0B1` | `utility/init.luau` | **P1** | **Saito**: Dimming controller directly sets `barrier = "popped"` on `battle.alive_actors` over 22f with `WIND_FX` and SFX `0x74`. **ROM**: Dimming spawns `atk 0x2C` (100f duration) which fires a field-wide Wind hitbox (`r1=0x25, r2=0x13`, `0x080AF0C0(0x80)`) stripping Barriers, Auras, and Wind-vulnerable objects/obstacles, while spawning wind gust particles (`0x080EEFC4`) every 8f. Wrong duration (22f vs 100f), missing Wind hitbox/obstacle removal, wrong SFX. | `fam=0x0C sub=0x05`, `eff=0x0F` `0x080F974C`, `atk 0x2C` `0x080DB404`, gust `0x080EEFC4`. SFX: `0xEF` (not `0x74`). |
| `fanfare` | `0x0AA` | `utility/init.luau` | **P0** | **Saito**: Dimming instant self-buff (`invuln = 240f`) with no field object. **ROM**: Dimming places destructible **40-HP** Trumpet obstacle (`atk 0x58`, 6000f lifetime) on panel in front (fails with puff `r4=2` if hole/occupied; replaces prior summon `0x080B5712`). Periodically every **80f** (`0x50`) calls `cb[0]` (`0x080C1876` -> `0x080012B4(0x1C+side)`) granting temporary invincibility to owner's side while alive. | `fam=0x0C sub=0x2D`, `eff=0x26` `0x080FB168`, `atk 0x58` `0x080E0F5C`, table `0x080AE900`, `cb[0]` `0x080C1876`, cd `0x080E1174[0]=0x50`. Sprite: `04-1f` pal `0`. SFX: `0xF6` (`0x080C1840[0]`). |
| `discord` | `0x0AB` | `utility/init.luau` | **P0** | **Saito**: Dimming instant `confuse` (180f) directly on opponent with no field object. **ROM**: Dimming places destructible **40-HP** Horn obstacle (`atk 0x58`, 6000f lifetime) on panel in front (fails if hole/occupied; replaces prior summon). Periodically every **120f** (`0x78`) calls `cb[1]` (`0x080C1882` -> `0x080D684C`, `r4=0x0C0DFE80, r7=0x2300`) emitting field-wide Confuse pulse. | `fam=0x0C sub=0x2E`, `eff=0x26` `0x080FB168`, `atk 0x58` `0x080E0F5C`, `cb[1]` `0x080C1882`, cd `0x080E1174[1]=0x78`. Sprite: `04-1f` pal `1`. SFX: `0xF7` (`0x080C1840[1]`). |
| `timpani` | `0x0AC` | `utility/init.luau` | **P0** | **Saito**: Dimming instant `paralyze` (120f) directly on opponent with no field object. **ROM**: Dimming places destructible **40-HP** Drum obstacle (`atk 0x58`, 6000f lifetime) on panel in front (fails if hole/occupied; replaces prior summon). Periodically every **120f** (`0x78`) calls `cb[2]` (`0x080C18A0` -> `0x080D684C`, `r4=0x0C0DFE83, r7=0x4100` + screen shake `0x08022B80(2,30)`) immobilizing enemies. | `fam=0x0C sub=0x2F`, `eff=0x26` `0x080FB168`, `atk 0x58` `0x080E0F5C`, `cb[2]` `0x080C18A0`, cd `0x080E1174[2]=0x78`. Sprite: `04-1f` pal `2`. SFX: `0xF8` (`0x080C1840[2]`). |
| `panic` | `0x029` | `utility/init.luau` | **P0** | **Saito**: Unconditionally fires a row wave (`regions.row`) down user's row with `buster_shoot`, `CONFUSE_FX`, SFX `0x128`. **ROM**: Checks if user has Confusion Bug (`0x0804733A(0x10,side)==0x0C`) **OR** is currently Confused (`[r5+0x2E] & 0x1000`). **ONLY if true** does it fire a field-wide confuse flash (`atk 0x07`, `r4=0x3D02FE80, r7=0x2200`); otherwise whiffs. Always flashes screen (`SpawnPanelEffect(0x0F00)`) + SFX `0xC5`, endlag 16f. | `fam=0x38 sub=0x00`, `0x080B4262`, bug check `0x0804733A`, `atk 0x07` `0x080D684C`. Sprite: screen flash `0x0F00` (no row FX). SFX: `0xC5` (`flashman-flash`, not `0x128`). |
| `rndmmetr` | `0x057` | `utility/init.luau` | **P0** | **Saito**: Drops 6 fire meteors every 8f *while dimmed*; places no field object. **ROM**: Dimming places destructible **40-HP** Meteor Rock obstacle (`atk 0x30`) in front of user (fails with puff if hole/occupied; replaces prior summon). After un-dimming, rock stays on field and drops **12 meteors** (`atk 0x0A`, 100 Fire dmg) every **30f** on random enemy panels (`0x080DBC20`, weighted toward occupied enemy panels when `rand<10`). | `fam=0x0C sub=0x34`, `eff=0x2D` `0x080FB844`, HP `0x080FB938[0]=40`, `atk 0x30` `0x080DB9FE`, cfg `0x080DB9F8=(12,30)`, target `0x080DBC20`, meteor `atk 0x0A` `0x080D6C2C`. Sprites: `0c-0f` (rock), `10-0a` anim 0 (meteor), FX `0x4F`. SFX: `0xE1` (fall), `0x76` (impact). |
| `holemetr` | `0x058` | `utility/init.luau` | **P0** | **Saito**: Drops 6 meteors every 8f *while dimmed* AND sets struck panels to `"broken"` (holes); places no field object. **ROM**: Dimming places destructible **60-HP** Meteor Rock (`atk 0x30`) in front of user. After un-dimming, drops **15 meteors** (`atk 0x0A`, `r4=0`, `r7[5]=0` — **NEVER cracks or breaks panels!**) every **20f** sweeping enemy panels column-by-column (`0x080DBC98`). | `fam=0x0C sub=0x35`, `eff=0x2D` `0x080FB844`, HP `0x080FB938[1]=60`, `atk 0x30` `0x080DB9FE`, cfg `0x080DB9F8=(15,20)`, target `0x080DBC98`, meteor `atk 0x0A` `0x080D6C2C` (`0x080D6C08[0]`). Sprites: `0c-0f`, `10-0a`. SFX: `0xE1`, `0x76`. |
| `shotmetr` | `0x059` | `utility/init.luau` | **P0** | **Saito**: Drops 6 meteors every 8f *while dimmed* targeting `enemy_pos`; places no field object. **ROM**: Dimming places destructible **80-HP** Meteor Rock (`atk 0x30`) in front of user. After un-dimming, drops **9 meteors** (`atk 0x0A`, 100 Fire dmg) every **20f** at mirrored X (`x = 2*rock.x - user.x`, cycling `y=1,2,3` via `0x080DBD22`). | `fam=0x0C sub=0x36`, `eff=0x2D` `0x080FB844`, HP `0x080FB938[2]=80`, `atk 0x30` `0x080DB9FE`, cfg `0x080DB9F8=(9,20)`, target `0x080DBD22`, meteor `atk 0x0A` `0x080D6C2C`. Sprites: `0c-0f`, `10-0a`. SFX: `0xE1`, `0x76`. |
| `lavacan1` | `0x007` | `utility/init.luau` | **P0** | **Saito**: Auto-targets enemy in row (or `x+2`), deals flat 90 Fire dmg, and turns target panel *into* Lava. **ROM**: Completely inverted! Counts Lava panels on **user's own side** (`0x0800BF5A(8,side)`), consumes/reverts them to Normal via `eff 0x20` (`0x080F0184`), adds `+10 dmg` per panel, upgrades projectile `atk 0x01` from `0x0D` to **piercing** `0x8E`, and fires a fast straight shot down user's row using attached weapon `0c-33` pal 0 (anim `0x36` 30f -> `0x3D`). | `fam=0x34 sub=0`, `0x080B3FAC`, lava count `0x0800BF5A`, `eff 0x20` `0x080F00B8`/`0x080F0184`, `atk 0x01` `0x080D5D28`, tbl `0x080D5CE4`, rows `0x080B40D8[0]=0` (1 row). Sprite: `0c-33` pal `0` (weapon). SFX: `0x115`/`0x116` (charge), `0x74`/`0xED` (fire, not `0x7D`). |
| `lavacan2` | `0x008` | `utility/init.luau` | **P0** | **Saito**: Same inverted stub (flat 120 dmg, creates Lava on target, no own-side absorption/bonus/pierce/weapon). **ROM**: Base 120 + `10 * own_lava_panels`, reverts own Lava to Normal, piercing straight shot when boosted, weapon `0c-33` pal 1. | `fam=0x34 sub=1`, `0x080B3FAC`, `eff 0x20` `0x080F00B8`, `atk 0x01` `0x080D5D28`. Sprite: `0c-33` pal `1`. SFX: `0x115`/`0x116`, `0x74`/`0xED`. |
| `lavacan3` | `0x009` | `utility/init.luau` | **P0** | **Saito**: Same inverted stub (flat 150 dmg, creates Lava on target, no own-side absorption/bonus/pierce/weapon). **ROM**: Base 150 + `10 * own_lava_panels`, reverts own Lava to Normal, piercing straight shot when boosted, weapon `0c-33` pal 2. | `fam=0x34 sub=2`, `0x080B3FAC`, `eff 0x20` `0x080F00B8`, `atk 0x01` `0x080D5D28`. Sprite: `0c-33` pal `2`. SFX: `0x115`/`0x116`, `0x74`/`0xED`. |
| `volcano` | `0x072` | `utility/init.luau` | **P0** | **Saito**: Fabricated `+` explosions on all lava panels on field (or hits enemy directly if none). **ROM**: `LavaCan` `sub=3`! Absorbs own-side Lava panels (`+10 dmg` each + piercing `0x8E`) and fires **3 straight cannon shots** (`0x080B40D8[3]=2` -> rows `y, y+1, y-1`) down all 3 rows with weapon `0c-33` pal 3! | `fam=0x34 sub=3`, `0x080B3FAC`, rows `0x080B40D8[3]=2` (3 shots: `y, y+1, y-1`), `eff 0x20` `0x080F00B8`, `atk 0x01` `0x080D5D28`. Sprite: `0c-33` pal `3`. SFX: `0x115`/`0x116`, `0x74`/`0xED`. |
| `iceball` | `0x019` | `utility/init.luau` | **P1** | **Saito**: Instant hit at frame 10 on `x+3*facing`, sets panel to Ice, **erroneously inflicts `statuses.freeze`**, no parabolic thrown ball in flight, wrong endlag (20f vs anim+60f), wrong FX/SFX (`0xAE`). **ROM**: Anim `0x22` throws parabolic `atk 0x05` (`10-2d` pal 1) at frame 7 (SFX `0x77`) to `x+3*facing`; on landing on solid panel, deals 120 Aqua dmg (no direct freeze status) and converts panel to Ice (`type 7` via `0x080D6562`) with SFX `0x124`; endlag `0x3C` (60f). | `fam=0x07 sub=5`, `0x080B15C0`, `atk 0x05` `0x080D6394`, land `0x080D652C[1]`=`0x080D6562`. Sprite: `10-2d` pal `1`. SFX: `0x77` (throw), `0x124` (`0x25+0xFF`, land). |
| `lavaball` | `0x01A` | `utility/init.luau` | **P1** | **Saito**: Instant hit at frame 10 on `x+3*facing`, sets panel to Lava, no parabolic thrown ball in flight, wrong endlag (20f vs anim+60f), wrong FX/SFX (`0x76`). **ROM**: Anim `0x22` throws parabolic `atk 0x05` (`10-2d` pal 2) at frame 7 (SFX `0x77`) to `x+3*facing`; on landing on solid panel, deals 140 Fire dmg and converts panel to Lava (`type 8` via `0x080D658A`) with SFX `0x114`; endlag `0x3C` (60f). | `fam=0x07 sub=6`, `0x080B15C0`, `atk 0x05` `0x080D6394`, land `0x080D652C[2]`=`0x080D658A`. Sprite: `10-2d` pal `2`. SFX: `0x77` (throw), `0x114` (`0x15+0xFF`, land). |
| `airstrm1` | `0x032` | `utility/init.luau` | **P0** | **Saito**: Fires 3 instant single-row hitboxes (`regions.row`) at frames 4/10/16 with `buster_shoot`, `WIND_FX`, SFX `0x74`. **ROM**: **Two-phase attack** with weapon `0c-32` pal 0: Phase 1 (**100f** `0x64`) spawns 0-dmg wind suction gusts (`atk 0x1E`, `10-0f`) every 12f cycling rows 3,2,1 (SFX `0xEF`); Phase 2 switches weapon to anim 1 and fires **3 multi-hit Tornadoes** (`atk 0x59`, `0c-1a`, 50 dmg) across all 3 rows in shuffled order (20f delay, then every 10f). | `fam=0x31 sub=0`, `0x080B3CB4`, dur `0x64`, `atk 0x1E` `0x080D9078`, `atk 0x59` `0x080E11F8`. Sprites: `0c-32` pal `0` (weapon), `10-0f` (gust), `0c-1a` (tornado). SFX: `0xEF` (wind). |
| `airstrm2` | `0x033` | `utility/init.luau` | **P0** | **Saito**: Same single-row 3-hit stub (60 dmg). **ROM**: Two-phase with Phase 1 duration **80f** (`0x50`), weapon `0c-32` pal 1, Phase 2 fires 3 shuffled-row Tornadoes (`atk 0x59`, 60 dmg). | `fam=0x31 sub=1`, `0x080B3CB4`, dur `0x50`, `atk 0x1E` `0x080D9078`, `atk 0x59` `0x080E11F8`. Sprites: `0c-32` pal `1`, `10-0f`, `0c-1a`. SFX: `0xEF`. |
| `airstrm3` | `0x034` | `utility/init.luau` | **P0** | **Saito**: Same single-row 3-hit stub (70 dmg). **ROM**: Two-phase with Phase 1 duration **60f** (`0x3C`), weapon `0c-32` pal 2, Phase 2 fires 3 shuffled-row Tornadoes (`atk 0x59`, 70 dmg). | `fam=0x31 sub=2`, `0x080B3CB4`, dur `0x3C`, `atk 0x1E` `0x080D9078`, `atk 0x59` `0x080E11F8`. Sprites: `0c-32` pal `2`, `10-0f`, `0c-1a`. SFX: `0xEF`. |
| `totem1` | `0x037` | `utility/init.luau` | **P0** | **Saito**: Invincible no-HP object (`0c-2e`), fires **3 times** (wrong!), heals **40 HP** (wrong!), auto-hits `enemy_pos` directly instead of firing a flame wave down the row. **ROM**: Places destructible **50-HP** obstacle (`atk 0x8C`, replaces prior summon `0x080B5712`) on panel in front (fails if hole/occupied). Cycles `sub+1` = **1 time**: waits 30f -> fires flame wave `atk 0x0C` (`0c-10`, 100 Fire dmg, SFX `0x82` every 15f for 30f) -> waits 10f -> **heals owner 120 HP** (`0x78`, SFX `0x72`) -> waits 100f -> vanishes. | `fam=0x39 sub=0`, `0x080B42B0`, `atk 0x8C` `0x080E8B5C`, HP `0x32` (`0x080E8BBA`), cycles `sub+1=1` (`0x080E8BC4`), heal `0x78=120` (`0x080E8D52`), flame `atk 0x0C` `0x080D6FB4`. Sprites: `04-25` pal `0` (totem), `0c-10` (flame). SFX: `0x92` (place), `0x82` (flame), `0x72` (heal). |
| `totem2` | `0x038` | `utility/init.luau` | **P0** | **Saito**: Invincible no-HP object (`0c-2e`), fires 3 times (should be **2**), heals 60 HP (should be **120 HP**), auto-hits `enemy_pos`. **ROM**: Destructible **50-HP** obstacle (`04-25` pal 1), **2 cycles** of flame wave `atk 0x0C` (140 Fire dmg) + **120 HP** heal. | `fam=0x39 sub=1`, `0x080B42B0`, `atk 0x8C` `0x080E8B5C` (cycles=2, HP=50, heal=120), `atk 0x0C` `0x080D6FB4`. Sprites: `04-25` pal `1`, `0c-10`. SFX: `0x92`, `0x82`, `0x72`. |
| `totem3` | `0x039` | `utility/init.luau` | **P0** | **Saito**: Invincible no-HP object (`0c-2e`), heals 80 HP (should be **120 HP**), auto-hits `enemy_pos`. **ROM**: Destructible **50-HP** obstacle (`04-25` pal 2), **3 cycles** of flame wave `atk 0x0C` (180 Fire dmg) + **120 HP** heal. | `fam=0x39 sub=2`, `0x080B42B0`, `atk 0x8C` `0x080E8B5C` (cycles=3, HP=50, heal=120), `atk 0x0C` `0x080D6FB4`. Sprites: `04-25` pal `2`, `0c-10`. SFX: `0x92`, `0x82`, `0x72`. |
| `spice1` | `0x048` | `utility/init.luau` | **P0** | **Saito**: Marked **`"dimming"`** (WRONG!) and detonates all grass panels simultaneously in 1 frame with `GRASS_FX` + SFX `0x76`. **ROM**: **NON-DIMMING** (`b11=0x01`)! Spawns `atk 0x46` (endlag `0x40`) which sweeps column-by-column every 8f (skipping columns without grass), spawning Wood hitboxes `atk 0x07` (`r0=4, r4=0x0102FE01, r7=0x2000`, 80 dmg) + FX `0x33` + SFX `0x10D` on each Grass panel. | `fam=0x2B sub=0`, `0x080B3972` (non-dimming `b11=0x01`), `atk 0x46` `0x080DE6BC`, `atk 0x07` `0x080D684C`. FX: `SpawnSimpleEffect(0x33)`. SFX: `0x10D` (`0x0E+0xFF`, not `0x76`). |
| `spice2` | `0x049` | `utility/init.luau` | **P0** | **Saito**: Marked `"dimming"`, simultaneous 1-frame detonation, wrong SFX. **ROM**: Non-dimming column-sweeping `atk 0x46` (110 Wood dmg per Grass panel, 8f/col). | `fam=0x2B sub=1`, `0x080B3972`, `atk 0x46` `0x080DE6BC`. FX: `0x33`. SFX: `0x10D`. |
| `spice3` | `0x04A` | `utility/init.luau` | **P0** | **Saito**: Marked `"dimming"`, simultaneous 1-frame detonation, wrong SFX. **ROM**: Non-dimming column-sweeping `atk 0x46` (140 Wood dmg per Grass panel, 8f/col). | `fam=0x2B sub=2`, `0x080B3972`, `atk 0x46` `0x080DE6BC`. FX: `0x33`. SFX: `0x10D`. |
| `crsshld1` | `0x068` | `utility/init.luau` | **P0** | **Saito**: Guards for **42f**; on hit immediately teleports a homing **`+` Break** AoE directly onto `enemy_pos` with `HEAL_FX`/`IMPACT_FX`. **ROM**: Raises shield (`0c-03` anim 6 pal 0, SFX `0x8C`) for **26f** (`0x1A`); on blocking an attack, spawns `atk 0x44` (`0c-03` anim 9 pal 0) moving straight forward along user's row at `12 px/f` (`0x0C_0000`), hitting a **single panel** (`r1=1`, Null element, 160 dmg, anim `0x0A`, SFX `0x111`) upon contact. | `fam=0x05 sub=3`, `0x080B12D4`, dur `0x1A=26f`, `atk 0x44` `0x080DE380`. Sprite: `0c-03` anim `6` (shield), anim `9`/`0x0A` (cross), pal `0`. SFX: `0x8C` (guard), `0x111` (`0x12+0xFF`, hit). |
| `crsshld2` | `0x069` | `utility/init.luau` | **P0** | **Saito**: Same 42f guard + homing `+` Break AoE stub (210 dmg). **ROM**: 26f guard + straight-line `atk 0x44` (`0c-03` pal 1, 210 Null dmg, single panel). | `fam=0x05 sub=4`, `0x080B12D4`, `atk 0x44` `0x080DE380`. Sprite: `0c-03` pal `1`. SFX: `0x8C`, `0x111`. |
| `crsshld3` | `0x06A` | `utility/init.luau` | **P0** | **Saito**: Same 42f guard + homing `+` Break AoE stub (260 dmg). **ROM**: 26f guard + straight-line `atk 0x44` (`0c-03` pal 2, 260 Null dmg, single panel). | `fam=0x05 sub=5`, `0x080B12D4`, `atk 0x44` `0x080DE380`. Sprite: `0c-03` pal `2`. SFX: `0x8C`, `0x111`. |
| `geyser` | `0x06B` | `utility/init.luau` | **P0** | **Saito**: Unconditionally erupts a 3x3 square at `x+3*facing` at frame 10 with `buster_shoot`, `ICE_FX`, SFX `0x76`. **ROM**: Anim `0x22` throws `atk 0x2F` (`0c-21`) at frame 7 (SFX `0x77`) to `x+3*facing`. **ACTIVATION CONDITION**: On landing (`0x080DB8F8`), checks `0x0800BF4A` (solid?); if solid (NOT a Hole), **FAILS** with puff `r4=1`! Only if landing in a **Hole panel** does it switch to sprite `10-23`, play SFX `0xB2`, and erupt in 2 stages (`r4=0x0102020A` 15f, then `0x0102020C`, 200 Aqua dmg). | `fam=0x25 sub=0`, `0x080B3462`, `atk 0x2F` `0x080DB78E` (hole check `0x080DB8F8`). Sprites: `0c-21` (thrown), `10-23` (eruption). SFX: `0x77` (throw), `0xB2` (erupt). |
| `shake1` | `0x06E` | `utility/init.luau` | **P0** | **Saito**: Instant single-hit `region_1d` (`{-1,0},{0,0},{1,0}`) explosion at frame 10; never places a field object. **ROM**: Anim `0x22` throws `atk 0x84` at frame 7 (SFX `0x77`) to `x+3*facing`. On valid solid unoccupied panel, places invincible (`0xFFFF` HP) **Shake / Mole field object** (`atk 0x82`, `04-19` pal 0, replaces prior summon `0x080B5712`) that periodically shakes/attacks per `0x080AEB80[0]` (90 dmg); only deals a single fallback hit if landing panel is blocked/hole. | `fam=0x36 sub=0`, `0x080B4154`, throw `atk 0x84` `0x080E7924`, mole `atk 0x82` `0x080E73C0` (update `0x080E7488`), cfg `0x080AEB80`. Sprite: `04-19` pal `0`. SFX: `0x77` (throw). |
| `shake2` | `0x06F` | `utility/init.luau` | **P0** | **Saito**: Same instant single-hit `region_1d` stub (110 dmg); no field object. **ROM**: Throws `atk 0x84` -> places `atk 0x82` Shake object (`04-19` pal 1, cfg `0x080AEB80[1]`, 110 dmg). | `fam=0x36 sub=1`, `0x080B4154`, `atk 0x84` `0x080E7924`, `atk 0x82` `0x080E73C0`. Sprite: `04-19` pal `1`. SFX: `0x77`. |
| `shake3` | `0x070` | `utility/init.luau` | **P0** | **Saito**: Same instant single-hit `region_1d` stub (140 dmg); no field object. **ROM**: Throws `atk 0x84` -> places `atk 0x82` Shake object (`04-19` pal 2, cfg `0x080AEB80[2]`, 140 dmg). | `fam=0x36 sub=2`, `0x080B4154`, `atk 0x84` `0x080E7924`, `atk 0x82` `0x080E73C0`. Sprite: `04-19` pal `2`. SFX: `0x77`. |
| `condor` | `0x073` | `utility/init.luau` | **P0** | **Saito**: Stands still with `buster_shoot` and spawns an instant `deep_fan` Wind hitbox in front. **ROM**: **Player dash/charge attack** (`fam=0x21 sub=1`)! Plays anims `0x28->0x29->0x2A` with attached weapon `0c-1e` pal 1, SFX `0xB4` + `0x94`, and **physically dashes user forward across the row** at `14 px/f` (`0x000E_0000`), spawning single-panel hitboxes (`atk 0x07`, `r4=0x01020901`, 180 Null dmg) ahead until hitting obstacle/edge, then warps back to start panel. | `fam=0x21 sub=1`, `0x080B2F62` (dash `0x080B30D4`, warp back `0x080B3152`), `atk 0x07` `0x080D684C`. Sprite: `0c-1e` anim `0/1/2` pal `1`. SFX: `0xB4`, `0x94`. |
| `burning` | `0x074` | `utility/init.luau` | **P0** | **Saito**: Single-frame `region_2a` (`(0,-1),(0,1),(0,0),(1..5,0)`) hitbox centered on user's OWN panel (`me.panel_x`), `buster_shoot`, SFX `0x76`. **ROM**: Anim `0x23`; targets **`(panel_x + facing, panel_y)`** (1 panel ahead!), spawns center fire pillar `atk 0x3B` (`10-22`, 15f, SFX `0x80`), then **10f later** spawns 4 adjacent fire pillars `(0,-1),(0,+1),(+1,0),(-1,0)` (2-stage `+` cross centered ahead, 150 Fire dmg) while user is intangible. | `fam=0x24 sub=1`, `0x080B3316`, offsets `0x080B3420`, `atk 0x3B` `0x080DD2BC`, tbl `0x080DD2B4`. Sprite: `10-22` (fire pillar). SFX: `0x80` (not `0x76`). |
| `fireratn` | `0x075` | `utility/init.luau` | **P0** | **Saito**: Instant homing `+` Fire blast directly onto `enemy_pos` at frame 6. **ROM**: Anim `0x23`, SFX `0xB0`, replaces prior summon (`0x080B5712`), places **stationary 200-HP Ratton obstacle** (`atk 0x24`, `0c-20`) on panel in front! Sits idle until **ignited by a Fire attack or Lava panel** (`0x080DA054`), ONLY then switches to anim 1 and charges forward turning once toward an enemy (230 Fire dmg). | `fam=0x1D sub=4`, `0x080B2BE4`, `atk 0x24` `0x080D9F40` (HP `0xC8=200`, fire trigger `0x080DA054`, turn `0x080DA18C`). Sprite: `0c-20` (`ratton`). SFX: `0xB0`. |
| `rockarm1` | `0x091` | `utility/init.luau` | **P0** | **Saito**: Instant `regions.long` Break hitbox at frame 6 + manually breaks `sx`; no RockArm object or shockwave. **ROM**: Anim `0x36`, SFX `0xD2`, replaces prior summon (`0x080B5712`), drops **RockArm field object** (`atk 0x22`, `04-21` pal 0, from `z=180`, `0xFFFF` HP) onto panel in front. On landing, smashes/breaks panel (`0x080D684C` `0x090A0080`), launches shockwave `0x080D6A7A` (`r4=0x0D`, 100 dmg) down the row with camera shake + SFX `0x7E`, lingers 60f. | `fam=0x1C sub=0`, `0x080B2B60`, `atk 0x22` `0x080D9980` (impact `0x080D9B0A`, wave `0x080D6A7A`). Sprite: `04-21` pal `0`. SFX: `0xD2` (summon), `0x7E` (smash). |
| `rockarm2` | `0x092` | `utility/init.luau` | **P0** | **Saito**: Same instant `regions.long` stub (150 dmg); no RockArm object or wave. **ROM**: Drops `atk 0x22` (`04-21` pal 1), smashes panel + launches shockwave (`r4=0x0E`, 150 dmg). | `fam=0x1C sub=1`, `0x080B2B60`, `atk 0x22` `0x080D9980`. Sprite: `04-21` pal `1`. SFX: `0xD2`, `0x7E`. |
| `rockarm3` | `0x093` | `utility/init.luau` | **P0** | **Saito**: Same instant `regions.long` stub (200 dmg); no RockArm object or wave. **ROM**: Drops `atk 0x22` (`04-21` pal 2), smashes panel + launches shockwave (`r4=0x0F`, 200 dmg). | `fam=0x1C sub=2`, `0x080B2B60`, `atk 0x22` `0x080D9980`. Sprite: `04-21` pal `2`. SFX: `0xD2`, `0x7E`. |
| `needler1` | `0x09A` | `utility/init.luau` | **P0** | **Saito**: 5-hit instant homing barrage on `enemy_pos` *while dimmed*; no field object placed. **ROM**: Dimming places destructible **Needler turret** (`atk 0x5C`, `04-1e` pal 0, 6000f lifetime, replaces prior summon `0x080B5712`, fails with puff if hole/occupied) on panel in front! After un-dimming, moves across panels and fires 3-needle volleys (`0x080E193C`, 30 dmg each, SFX `0x7D`, death FX `0` + SFX `0x76`). | `fam=0x0C sub=0x30`, `eff=0x28` `0x080FB35C`, `atk 0x5C` `0x080E1688` (shoot `0x080E193C`), tables `0x080E167C` & `0x080AE980`. Sprite: `04-1e` pal `0`. SFX: `0x7D` (shoot), `0x76` (explode). |
| `needler2` | `0x09B` | `utility/init.luau` | **P0** | **Saito**: Same dimmed 5-hit homing stub (40 dmg); no turret object. **ROM**: Dimming places Needler turret (`atk 0x5C`, `04-1e` pal 1, 40 dmg/needle, cfg `0x080AE990`). | `fam=0x0C sub=0x31`, `eff=0x28` `0x080FB35C`, `atk 0x5C` `0x080E1688`. Sprite: `04-1e` pal `1`. SFX: `0x7D`, `0x76`. |
| `needler3` | `0x09C` | `utility/init.luau` | **P0** | **Saito**: Same dimmed 5-hit homing stub (50 dmg); no turret object. **ROM**: Dimming places Needler turret (`atk 0x5C`, `04-1e` pal 2, 50 dmg/needle, cfg `0x080AE9A0`). | `fam=0x0C sub=0x32`, `eff=0x28` `0x080FB35C`, `atk 0x5C` `0x080E1688`. Sprite: `04-1e` pal `2`. SFX: `0x7D`, `0x76`. |
| `team1` | `0x084` | `utility/init.luau` | **P0** | **Saito**: Instant dimmed single 10-dmg hit on `enemy_pos`; no summon placed. **ROM**: Dimming places destructible **40-HP** Team1 Navi/ally summon (`atk 0x8D`, `04-26` pal 0, **300f** lifetime, replaces prior summon) on panel in front. Monitors owner taking damage (up to 4 speed-ups: cd `60->35->30->25->20f` at `0x080E8FA8`) and periodically fires `atk 0x49` (`10-28`, 10 dmg, SFX `0xB6`) at enemy panels! | `fam=0x0C sub=0x44`, `eff=0x3F` `0x080FD0BC`, `atk 0x8D` `0x080E8E14` (cd tbl `0x080E8FA8`, dmg watch `0x080E9048`), proj `atk 0x49` `0x080DF034`. Sprites: `04-26` pal `0` (summon), `10-28` (shot). SFX: `0xB6`. |
| `team2` | `0x0A9` | `utility/init.luau` | **P0** | **Saito**: Instant dimmed heal of **50 HP** + **120f invuln**; no summon placed. **ROM**: Dimming places destructible **40-HP** Team2 ally summon (`atk 0x8D`, `04-26` pal 4, **600f** lifetime, replaces prior summon) on panel in front. Watches for owner taking damage when HP `<= 25%` max HP (`lsr r1,#2` at `0x080E8FDE`), then **heals owner 500 HP** (`0xFA<<1` at `0x080E8FE8`) with heal FX + SFX `0x72`! | `fam=0x0C sub=0x45`, `eff=0x3F` `0x080FD0BC`, `atk 0x8D` `0x080E8E14` (25% check `0x080E8FDE`, 500 HP heal `0x080E8FE8`). Sprite: `04-26` pal `4`. SFX: `0x72`. |
| `mettaur` | `0x0A6` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `80` dmg; NO Mettaur sprite/actor, NO ShockWave, NO Virus Breeder tier/power. **ROM**: Dimming Family `0x1F` checks Breeder capture (`0x080076EA`), rolls tier (`0x08007794`), computes power `[40,80,120,150] + 5*(feeds/5)` (`0x0800781C`), and spawns `Type1 0x16` Mettaur summon (`04-01`) to fire ShockWave(s). | `fam=0x1F` `0x0804752A`, `eff=0x11` `0x080F994C`, spawner `0x080BCC60`, `Type1 0x16` (`0x0800369C`) `0x080BC940`, base pwr `0x0800D764[0..3]=40/80/120/150`. Sprite: `04-01`. |
| `bunny` | `0x0A7` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `80` Elec+Paralyze; NO Bunny sprite/actor, NO ZapRing, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x22` Bunny summon (`04-0a`), Breeder base power `[60,90,140,160] + 5*(feeds/5)`, fires paralyzing ZapRing down row. | `fam=0x1F` `0x0804752A`, `eff=0x1A` `0x080FA3B4`, spawner `0x080BF89C`, `Type1 0x22` `0x080BF5A0`, base pwr `0x0800D764[4..7]=60/90/140/160`. Sprite: `04-0a`. |
| `spikey` | `0x0BA` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `100` Fire; NO Spikey sprite/actor, NO HeatShot/fireball, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x23` Spikey summon (`04-09`), Breeder base power `[50,80,120,170] + 5*(feeds/5)`, fires flame shot down row. | `fam=0x1F` `0x0804752A`, `eff=0x1D` `0x080FA6EC`, spawner `0x080BFBB0`, `Type1 0x23` `0x080BF90C`, base pwr `0x0800D764[12..15]=50/80/120/170`. Sprite: `04-09`. |
| `swordy` | `0x0B9` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `120` Sword; NO Swordy sprite/actor, NO tier-based elemental slash, NO Breeder power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x25` Swordy summon (`04-04`), Breeder base power `[120,120,120,120] + 5*(feeds/5)` (tiers change element/slash pattern). | `fam=0x1F` `0x0804752A`, `eff=0x1E` `0x080FA830`, spawner `0x080C044C`, `Type1 0x25` `0x080C01B0`, base pwr `0x0800D764[8..11]=120/120/120/120`. Sprite: `04-04`. |
| `jelly` | `0x0BC` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `90` Aqua; NO Jelly sprite/actor, NO tier-based elemental wave/shot, NO Breeder power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x4A` Jelly summon (`04-07`), Breeder base power `[50,80,80,150] + 5*(feeds/5)`. | `fam=0x1F` `0x0804752A`, `eff=0x39` `0x080FC934`, spawner `0x080CBC50`, `Type1 0x4A` `0x080CB95C`, base pwr `0x0800D764[20..23]=50/80/80/150`. Sprite: `04-07`. |
| `mushy` | `0x0BB` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `100` Wood+Confuse; NO Mushy sprite/actor, NO spore toss, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x3D` Mushy summon (`04-0d`), Breeder base power `[60,80,100,120] + 5*(feeds/5)`. | `fam=0x1F` `0x0804752A`, `eff=0x2B` `0x080FB5E8`, spawner `0x080C70C0`, `Type1 0x3D` `0x080C6CF4`, base pwr `0x0800D764[16..19]=60/80/100/120`. Sprite: `04-0d`. |
| `momogra` | `0x04D` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `110` Null; NO Momogra sprite/actor, NO underground ambush pop-up, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x49` Momogra summon (`04-17`), Breeder base power `[90,120,150,200] + 5*(feeds/5)`. | `fam=0x1F` `0x0804752A`, `eff=0x38` `0x080FC7F0`, spawner `0x080CB92A`, `Type1 0x49` `0x080CB6E8`, base pwr `0x0800D764[28..31]=90/120/150/200`. Sprite: `04-17`. |
| `killreye` | `0x0BD` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `120` Elec+Paralyze; NO KillrEye sprite/actor, NO sensor beam sweep, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x4D` KillrEye summon (`04-16`), Breeder base power `[100,100,100,150] + 5*(feeds/5)`. | `fam=0x1F` `0x0804752A`, `eff=0x3C` `0x080FCD84`, spawner `0x080CC790`, `Type1 0x4D` `0x080CC3C4`, base pwr `0x0800D764[24..27]=100/100/100/150`. Sprite: `04-16`. |
| `scuttlst` | `0x04C` | `utility/init.luau` | **P0** | **Saito**: Generic `VIRUS_CTRL` auto-hits `enemy_pos` for hardcoded `140` Null; NO Scuttlst sprite/actor, NO barrier/multi-element summon attack, NO Breeder tier/power. **ROM**: Dimming Family `0x1F` spawns `Type1 0x57` Scuttlst summon (`04-12`), Breeder base power `[100,150,200,130] + 5*(feeds/5)`. | `fam=0x1F` `0x0804752A`, `eff=0x42` `0x080FD420`, spawner `0x080D049E`, `Type1 0x57` `0x080CFF78`, base pwr `0x0800D764[32..35]=100/150/200/130`. Sprite: `04-12`. |

---

#### Detailed ROM Verification Notes

##### 1. Instruments — `Fanfare` (`0x0AA`), `Discord` (`0x0AB`), `Timpani` (`0x0AC`)
- **ROM Entry**: `DimmingEffect 0x26` (`0x080FB168`, `b11=0x60`) verifies the panel directly in front via `CheckPanelFlags(r2=0x10, r3=0x3E000000)`. If hole or occupied, spawns smoke puff `r4=2` and fails; otherwise calls `0x080B5712` (clears user's previous summon/obstacle) and spawns `atk 0x58` (`0x080E0F5C`).
- **`atk 0x58` Details**:
  - Sprite: `04-1f`, palette `0` (Fanfare) / `1` (Discord) / `2` (Timpani).
  - **HP**: `40` (`0x28` at `0x080E100C`), registered as destructible obstacle (`0x080AD7D0`).
  - **Lifetime**: `6000` frames (`0x1770`).
  - **Periodic Cooldowns** (`0x080E1174`): `0x50` (80f Fanfare), `0x78` (120f Discord), `0x78` (120f Timpani).
  - **SFX Table** (`0x080C1840`): `0xF6` (Fanfare), `0xF7` (Discord), `0xF8` (Timpani).
  - **Effect Callbacks** (`0x080C1830`):
    - `cb[0]` (`0x080C1876`, Fanfare): `0x080012B4(side + 0x1C)` — sets owner-side invincibility flag bit in battle state `[r7+0x64]`.
    - `cb[1]` (`0x080C1882`, Discord): `0x080D684C(r0=5, r4=0x0C0DFE80, r6=0, r7=0x2300)` — field-wide Confuse status hitbox.
    - `cb[2]` (`0x080C18A0`, Timpani): `0x080D684C(r0=5, r4=0x0C0DFE83, r6=0, r7=0x4100)` + `0x08022B80(2, 30)` — field-wide Immobilize status hitbox + screen shake.

##### 2. Meteor Rocks — `RndmMetr` (`0x057`), `HoleMetr` (`0x058`), `ShotMetr` (`0x059`)
- **ROM Entry**: `DimmingEffect 0x2D` (`0x080FB844`) validates panel in front (`r2=0x10, r3=0x3E000000`), calls `0x080B5712`, and spawns destructible `atk 0x30` (`0x080DB9FE`, sprite `0c-0f`) with HP from `0x080FB938`: **40 / 60 / 80 HP** (`0x28 / 0x3C / 0x50`).
- **Drop Schedules** (`0x080DB9F8`):
  - `RndmMetr` (`sub=0`): **12 meteors** every **30f**, targeting `0x080DBC20` (random enemy panel; 10/16 chance weighted to panels with an enemy).
  - `HoleMetr` (`sub=1`): **15 meteors** every **20f**, targeting `0x080DBC98` (sequential column-by-column sweep across enemy alliance panels).
  - `ShotMetr` (`sub=2`): **9 meteors** every **20f**, targeting `0x080DBD22` (`x = 2*rock.x - user.x`, cycling rows `y = 1, 2, 3`).
- **Critical ROM Correction on `HoleMetr`**: At `0x080DBB64`, ALL three variants call `0x080D6DF8(r0=2, r4=0, r6=100)` spawning `atk 0x0A` (`0x080D6C2C`) with `sub=0`. Table `0x080D6C08[0]` = `[0x0A, 0x00, 0x09, 0x0A, 0x01, 0x00]`. At `0x080D6D98`, `r7[5] == 0` skips `0x0800B568` (panel crack/break) completely! **None of the three Meteor chips crack or break panels in BN3**; saito's panel-breaking in `holemetr` is a fabrication caused by the English name "HoleMetr".

##### 3. `LavaCan1..3` (`0x007..0x009`) & `Volcano` (`0x072`)
- **ROM Entry**: Family `0x34` (`0x080B3FAC`, `sub=0..3`).
- Plays anim `0x36` + attached weapon `0c-33` anim `0` pal `sub` for 30f, then anim `0x3D` + weapon anim `1`.
- Calls `0x0800BF5A(8, side)` to count Lava panels on **user's own side**:
  - `bonus = count * 10` added to base damage (`90 / 120 / 150 / 150`).
  - If `count > 0`, spawns `eff 0x20` (`0x080F00B8` -> `0x080F0184`), which absorbs and converts those own-side Lava panels back to Normal panels (`0x0800B790`/`0x0800B7F8`), plays SFX `0x116` (else `0x115`), and upgrades `atk 0x01` (`0x080D5D28`) mode from `0x0D` to piercing `0x8E` (fire SFX `0xED` instead of `0x74`).
- Shot count table `0x080B40D8`: `[0, 0, 0, 2]` (`+1` shots), so `LavaCan1..3` fire 1 straight shot on user's row while **`Volcano` (`sub=3`) fires 3 straight cannon shots** across rows `y`, `y+1`, `y-1`.

##### 4. `Totem1..3` (`0x037..0x039`)
- **ROM Entry**: Family `0x39` (`0x080B42B0`, non-dimming) checks panel in front, calls `0x080B5712`, SFX `0x92`, spawns `atk 0x8C` (`0x080E8B5C`).
- Sprite `04-25` pal `sub`, **HP = 50** (`0x32` at `0x080E8BBA`), **cycles = `sub + 1`** (`1 / 2 / 3` at `0x080E8BC4`).
- Each cycle: 30f idle -> fires flame wave `atk 0x0C` (`0x080D6FB4`, sprite `0c-10`, Fire element, `100/140/180` dmg, SFX `0x82`) -> 10f -> **heals owner 120 HP** (`0x78` at `0x080E8D52`, SFX `0x72`) -> 100f cooldown.

##### 5. `AirStrm1..3` (`0x032..0x034`)
- **ROM Entry**: Family `0x31` (`0x080B3CB4`), weapon `0c-32` pal `sub`.
- **Phase 1**: Runs `100 / 80 / 60` frames (`0x64 / 0x50 / 0x3C`), SFX `0xEF`, every 12f spawning 0-damage wind suction gusts (`atk 0x1E` `0x080D9078`, sprite `10-0f`) cycling rows `3 -> 2 -> 1`.
- **Phase 2**: Switches weapon `0c-32` to anim `1`, shuffles rows `{1, 2, 3}`, then after 20f and every 10f fires **3 multi-hit Tornadoes** (`atk 0x59` `0x080E11F8`, sprite `0c-1a`, base `50/60/70` dmg) across all 3 rows.

##### 6. `Needler1..3` (`0x09A..0x09C`) & `Team1`/`Team2` (`0x084`/`0x0A9`)
- **`Needler1..3`**: `eff 0x28` (`0x080FB35C`) places destructible turret `atk 0x5C` (`0x080E1688`, sprite `04-1e` pal `sub`, lifetime 6000f, cfg `0x080AE980`/`90`/`A0`) that patrols panels and fires 3-needle volleys (`0x080E193C`, SFX `0x7D`, `30/40/50` dmg).
- **`Team1` / `Team2`**: `eff 0x3F` (`0x080FD0BC`) places destructible **40-HP** summon `atk 0x8D` (`0x080E8E14`, sprite `04-26` pal `0`/`4`, lifetime `300f` / `600f`). `Team1` fires `atk 0x49` (`0x080DF034`, sprite `10-28`, SFX `0xB6`) at enemy panels on a cooldown (`60/35/30/25/20f` at `0x080E8FA8`) that shortens up to 4 times whenever the owner takes damage. `Team2` monitors owner damage (`0x080E9048`) and when owner HP `<= 25%` max HP (`0x080E8FDE`), heals **500 HP** (`0x080E8FE8`, SFX `0x72`).

##### 7. `GrabBack`/`GrabRvng`, `NrthWind`, `Spice1..3`, `CrsShld1..3`, `Geyser`, `Shake1..3`, `Condor`, `Burning`, `FireRatn`, `RockArm1..3`, `Panic`, `IceBall`/`LavaBall`
- **`GrabBack`/`GrabRvng`**: `eff 0x15` (`0x080F9DD0`) counts stolen home panels (`0x080F9EC4`), never reclaims them, then every 30f (`stolen` times) calls `0x080F9F2A` spawning `atk 0x3C` (`0x080DD450`, sprite `0c-0e`, SFX `0x78`/`0x79`) on every panel occupied by an enemy (`40`/`80` dmg per strike).
- **`NrthWind`**: `eff 0x0F` (`0x080F974C`) plays SFX `0xEF` and spawns 100f `atk 0x2C` (`0x080DB404`) with field-wide Wind hitbox `r1=0x25, r2=0x13` (`0x080AF0C0(0x80)`), stripping barriers, auras, and wind-vulnerable objects.
- **`Spice1..3`**: Non-dimming (`fam=0x2B`, `b11=0x01`, `0x080B3972`), spawns `atk 0x46` (`0x080DE6BC`) sweeping column-by-column every 8f across Grass panels (`CheckPanelFlags(0x400)`), dealing Wood dmg + FX `0x33` + SFX `0x10D`.
- **`CrsShld1..3`**: `fam=0x05 sub=3..5` (`0x080B12D4`), 26f (`0x1A`) guard (`0c-03` anim 6, SFX `0x8C`), spawns straight-moving `atk 0x44` (`0x080DE380`, `0c-03` anim 9, `12 px/f`) hitting single panel (`r1=1`, Null, SFX `0x111`).
- **`Geyser`**: `fam=0x25` (`0x080B3462`), throws `atk 0x2F` (`0c-21`, SFX `0x77`) to `x+3*facing`; requires Hole panel (`0x0800BF4A == 0` at `0x080DB8F8`) to erupt (`10-23`, SFX `0xB2`, 2-stage `0x0A`/`0x0C`).
- **`Shake1..3`**: `fam=0x36` (`0x080B4154`), throws `atk 0x84` (`0x080E7924`, SFX `0x77`) to `x+3*facing`; places invincible (`0xFFFF` HP) Mole/Shake object `atk 0x82` (`0x080E73C0`, `04-19` pal `sub`, cfg `0x080AEB80`) on solid unoccupied panels.
- **`Condor`**: `fam=0x21 sub=1` (`0x080B2F62`), user physically dashes across row (`14 px/f`, weapon `0c-1e` pal 1, SFX `0xB4`/`0x94`) dealing `180` Null dmg ahead, then warps back.
- **`Burning`**: `fam=0x24 sub=1` (`0x080B3316`), anim `0x23`, intangible, spawns `atk 0x3B` (`10-22`, SFX `0x80`) at `(x+facing, y)` then 10f later spawns 4 surrounding pillars forming a `+` cross centered 1 panel ahead (`0x080B3420`).
- **`FireRatn`**: `fam=0x1D sub=4` (`0x080B2BE4`), places 200-HP stationary `atk 0x24` (`0x080D9F40`, `0c-20`, SFX `0xB0`) in front; activates only when hit by Fire or standing on Lava (`0x080DA054`).
- **`RockArm1..3`**: `fam=0x1C` (`0x080B2B60`), anim `0x36`, SFX `0xD2`, drops `atk 0x22` (`0x080D9980`, `04-21` pal `sub`, from `z=180`), smashes panel + launches shockwave `0x080D6A7A` (`r4=0x0D+sub`, SFX `0x7E`), lingers 60f.
- **`Panic`**: `fam=0x38` (`0x080B4262`), requires user Confusion Bug (`0x0804733A(0x10,side)==0x0C`) or active Confusion (`[r5+0x2E] & 0x1000`); fires field-wide confuse `atk 0x07` (`r4=0x3D02FE80, r7=0x2200`) + screen flash `0x0F00` + SFX `0xC5`.
- **`IceBall`/`LavaBall`**: `fam=0x07 sub=5/6` (`0x080B15C0`), anim `0x22`, throws parabolic `atk 0x05` (`10-2d` pal `1`/`2`, SFX `0x77`) at frame 7 to `x+3*facing`, endlag 60f; landing sets Ice (`type 7`, SFX `0x124`, no direct freeze status) or Lava (`type 8`, SFX `0x114`).

##### 8. Virus Breeder & All 9 Virus Chips (`fam=0x1F`)
- **Dispatch**: `0x0804752A` looks up dimming effect ID in `VirusDimParamTable` (`0x08047494`, 9 halfwords: `0x11, 0x1A, 0x1E, 0x1D, 0x2B, 0x39, 0x3C, 0x38, 0x42`).
- **Virus Breeder Mechanics**:
  - `0x080076EA`: Checks whether at least one tier of that virus family has been captured in the Virus Breeder.
  - `0x08007750` / `0x08007794` / `0x080077D4`: Weighted tier RNG (`0x0800D793`) across captured tiers `[0..3]`.
  - `0x0800781C`: Computes `damage = base_power[virus*4 + tier]` (`0x0800D764`) `+ (feed_count / 5) * step[virus]` (`0x0800D78A`, `step = 5` for all 9 viruses).
- **Exact `Type1` Summon Jump Table (`0x0800369C`) Mapping**:
  - `0` `Mettaur` (`0x0A6`): `eff 0x11` -> `Type1 0x16` (`0x080BC940`), sprite `04-01`, base `[40, 80, 120, 150]`
  - `1` `Bunny` (`0x0A7`): `eff 0x1A` -> `Type1 0x22` (`0x080BF5A0`), sprite `04-0a`, base `[60, 90, 140, 160]`
  - `2` `Swordy` (`0x0B9`): `eff 0x1E` -> `Type1 0x25` (`0x080C01B0`), sprite `04-04`, base `[120, 120, 120, 120]`
  - `3` `Spikey` (`0x0BA`): `eff 0x1D` -> `Type1 0x23` (`0x080BF90C`), sprite `04-09`, base `[50, 80, 120, 170]`
  - `4` `Mushy` (`0x0BB`): `eff 0x2B` -> `Type1 0x3D` (`0x080C6CF4`), sprite `04-0d`, base `[60, 80, 100, 120]`
  - `5` `Jelly` (`0x0BC`): `eff 0x39` -> `Type1 0x4A` (`0x080CB95C`), sprite `04-07`, base `[50, 80, 80, 150]`
  - `6` `KillrEye` (`0x0BD`): `eff 0x3C` -> `Type1 0x4D` (`0x080CC3C4`), sprite `04-16`, base `[100, 100, 100, 150]`
  - `7` `Momogra` (`0x04D`): `eff 0x38` -> `Type1 0x49` (`0x080CB6E8`), sprite `04-17`, base `[90, 120, 150, 200]`
  - `8` `Scuttlst` (`0x04C`): `eff 0x42` -> `Type1 0x57` (`0x080CFF78`), sprite `04-12`, base `[100, 150, 200, 130]`


---

### 5.5 Group 5：Mega 与 Giga 非领航员芯片（26 张，含 White / Blue 双版本对照）

- **Scope**: 26 chips total — 15 Mega chips in `content/exe3/chips/mega/init.luau`, 10 Giga chips in `content/exe3/chips/giga/init.luau`, and 1 Giga chip (`bassgs`, `0x138`) in `content/exe3/chips/pa/init.luau`.
- **ROMs Audited**:
  - White US (`bn3.gba`, A6BE) — primary address basis unless noted
  - Blue US (`bn3b.gba`, A3XE) — verified for all Blue-exclusive Giga chips (`deltaray`, `foldrbak`, `darkaura`, `alpharmom`, `bass-137`); all 26 chip rows in `ChipDataTable` (`0x08011510`) and dimming/action handlers exist in both ROMs, with `bn3b` dimming handlers shifted by `+0xF0` from `bn3`.
- **Verified Key A6BE Jump Tables**:
  - `ChipDataTable`: `0x08011510` (0x20 B/entry)
  - `PlayerActionJumpTable`: `0x080B0DA4`
  - `DimmingEffectJumpTable`: `0x08008D00` (`DimmingChipParamTable` Fam 12: `0x080B1AF0`; `NaviDimParamTable` Fam 20: `0x080473E4`)
  - `Type1ObjectJumpTable` (`0x0800304C`): `0x0800369C` (corrected from `0x080036A8`)
  - `AttackObjJumpTable` (`0x08003084`): `0x080038C4`
  - `EffectObjJumpTable` (`0x080030D8`): `0x08003C18`
  - `CollisionMaskTable`: `0x080AF51C` (`0x080AF074`)
  - `RegionTable`: `0x080AF478` (bit-7 panel-flag regions: `0x080AF4D4`)
  - `DarkChipCheck` (DarkLicense OR DarkHole): `0x080B595C` (called by `DarkAura`, `Bass`, `Bass+`, `Serenade`)

---

#### Executive Summary

| Severity | Count | Chips |
| :--- | :---: | :--- |
| **P0** | **23** | `zeushamr`, `godstone`, `oldwood`, `poltrgst`, `jealousy`, `standout`, `watrline`, `ligtning`, `gaiaswrd`, `salamndr`, `fountain`, `bolt`, `gaiablad`, `deltaray`, `foldrbak`, `navrcycl`, `alpharmsg`, `bass`, `serenade`, `darkaura`, `alpharmom`, `bass-137`, `bassgs` |
| **P1** | **3** | `guardian`, `meteors`, `balance` |
| **P2** | **0** | — |
| **OK** | **0** | — |
| **Total** | **26** | 15 Mega (`mega/init.luau`) + 11 Giga (10 `giga/init.luau` + 1 `pa/init.luau`) |

##### Cross-Cutting Systemic Issues Found in Batch 5
1. **Non-Dimming Chips Wrongly Flagged as Dimming (8 chips)**: `zeushamr`, `standout`, `salamndr`, `watrline`, `fountain`, `ligtning`, `bolt`, and `darkaura` are all **non-dimming** player actions in the ROM (`b11=0x01`/`0x02`), yet Saito implements every one of them with `flags = { "dimming", ... }`.
2. **Missing Style-Specific Requirements (8 chips)**: All four elemental Mega pairs check `[player + 0x17]` (active Style element) and fizzle to action `0x15` if mismatched: Heat (`2`) for `standout`/`salamndr`, Aqua (`3`) for `watrline`/`fountain`, Elec (`1`) for `ligtning`/`bolt`, Wood (`4`) for `gaiaswrd`/`gaiablad`. None are checked in Saito.
3. **Missing DarkHole / DarkLicense Activation Requirement (4 Giga chips)**: `darkaura` (`0x080B1190`), `bass`, `bass-137`, and `serenade` (`0x080474D8..0x080474E6`) all call `0x080B595C` — requiring either NaviCust **DarkLicense** (`[0x02037920 + side*0x40 + 0x20] != 0`) or active **DarkHole** (`CheckBattleFlag(0x16 + side)` set by `Hole` chip), fizzling to action `0x15` otherwise. (`bassgs` intentionally does **not** check `0x080B595C`.)
4. **Placeholder / Fabricated Implementations**: `godstone`, `oldwood`, `poltrgst`, `standout`/`salamndr`, `watrline`/`fountain`, `ligtning`/`bolt`, `deltaray`, `alpharmsg`, `alpharmom`, `bass`, `bass-137`, `bassgs`, and `serenade` use simplified generic bursts instead of their actual ROM field summons, sub-objects, sprites, or input mechanics.

---

#### Per-Chip Audit Table

##### Part 1: Mega Chips (`content/exe3/chips/mega/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `guardian` | `0x0CB` | `content/exe3/chips/mega/init.luau` (`@exelib/guardian/*`) | **P1** | Retaliation deals `element.null` instead of **Elec (`1`)** (`0x080DC0B4`); retaliation strike dim waits `48t` instead of `60t` (`0x3C`); attacker hit masks in `exelib` (`0xAA000000`/`0x55000000`) differ from ROM (`0x52000000`/`0xA4000000`); statue sprite uses `0c-29` (`assets.toml`) instead of `10-1f`; retaliation uses `hit-bomb` (`0x76`) + `zap` (`10-1b`) instead of SFX `0xBB` + lightning FX `0x2C`. | Fam 12 sub 8; Dim `eff=0x0B` (`0x080F9428`), retal Dim `eff=0x0C` (`0x080F94FC`); `AttackObj[0x32]` (`0x080DBEFC`, retal `0x080DC0A4`, `r4=0x22230285`/`0x22230284`, `elem=1`, `hit_mod=3`). Sprite: `10-1f` (statue), FX `0x2C` (strike). SFX: `0x92` (place dim), `0xBB` (strike), shake `(1, 60)`. |
| `zeushamr` | `0x0D0` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch** (ROM is non-dimming!). **Self-damage bug**: Saito directly mutates `user.hp -= dmg`, bypassing barriers/auras/invis/AirShoes/HolyPanel/flinch; ROM hits ALL grounded actors on both sides via normal `AttackObj[0x4D]` collision (`0x48` = `0xC0000010`, regions `0x87`/`0x88`, `hit_mod=3` not `5`). **Hole check missing**: ROM only earthquakes if `(x+facing, y)` is solid (`flags & 0x10`); if hole/off-field, only spawns a 1-panel hit (`0x080D684C`) on `(x+facing, y)` with NO earthquake. Wrong visuals & SFX. | Fam 47 sub 0 (`0x080B3B0E`), `b11=0x01`; `AttackObj[0x4D]` (`0x080DFA14`, coll `0x48`, regions `0x87`/`0x88`, `hit_mod=3`). Attached weapon: `0c-13` pal 1 (`anim 0x24`/`0x25`). SFX: `0x7E` (ground slam / solid hit) or `0xCD` (miss/hole), shake `(2, 60)`. |
| `godstone` | `0x0D1` | `content/exe3/chips/mega/init.luau` | **P0** | **Completely fabricated summon & mechanics**: Saito scans whole field, falls back to non-hole panels, spawns `rockcube` (`0c-2d` anim 9), and fires 5 panel-cracking `METEOR_FX` hits every 16t. ROM drops orb (`0c-0e`) strictly onto `(x+facing, y)` and **requires an empty HOLE** (`forbid=0x3E000010`), else fizzles (SFX `0x112`). Summons invincible GodStone (`0c-2b`) + 2 floating stone hands (`0c-2c`) at `(x, y±1)`; triggers 3 hand slams (every 90t), each dropping **3–5 boulders** (`AttackObj[0x52]`, `0c-2d`, `hit_mod=3`, no crack) on random enemy panels. | Fam 12 sub 41; Dim `eff=0x22` (`0x080FAC2C`); `AttackObj[0x50]` (`0x080DFF78` orb/hole check); `Type1[0x29]` (`0x080C10C8` GodStone); `AttackObj[0x51]` (`0x080E0080` hands, slam `0x080E0206`); `AttackObj[0x52]` (`0x080E02A4` boulders, coll `0x1C/0x13`). Sprites: `0c-0e`, `0c-2b`, `0c-2c`, `0c-2d`. SFX: `0x78` (drop), `0x112` (fizzle), `0x7E`/`0x7F` (hand slam), `0x95` (boulder land), shake `(2, 30)`. |
| `oldwood` | `0x0D2` | `content/exe3/chips/mega/init.luau` | **P0** | **Completely fabricated summon & mechanics**: Shares GodStone's `rockcube` turret without hole check and fires 6 `regions.plus` `wood-vines` shots every 16t. ROM requires an **empty HOLE** strictly at `(x+facing, y)` via `AttackObj[0x50]` (fizzles SFX `0x112` otherwise), summons OldWood (`0c-2e`, fades in 32t), waits 40t, collects & shuffles ALL solid enemy-area panels, and spawns one **single-panel (`0x01`, NOT Plus)** Wood spike (`AttackObj[0x53]` row 4, `0c-31`, `hit_mod=1`) every 15t on every solid enemy panel (up to 9 panels). | Fam 12 sub 42; Dim `eff=0x23` (`0x080FAD8C`); `AttackObj[0x50]` (`0x080DFF78`); `Type1[0x2A]` (`0x080C12B4` OldWood, panel list `0x080C1494`); `AttackObj[0x53]` (`0x080E0430` row 4, region `0x01`, elem `4` Wood, `hit_mod=1`). Sprites: `0c-0e`, `0c-2e`, `0c-31` (anim 0→1→2). SFX: `0x78`, `0x112`, `0x92` (appear), `0x9C` (spike). |
| `meteors` | `0x0D4` | `content/exe3/chips/mega/init.luau` | **P1** | **Wrong count, cadence, targeting, cracking, hit_mod, & presentation**: Saito spawns 18 instant random-panel hits every 4t, cracks every 3rd panel, and uses `hit_mod=1` (final `3`). ROM spawns **`30` (`0x1E`)** meteors every **`10` ticks**, cycling deterministically through enemy columns/rows (`0x080FBA98`, not random). Each meteor shows a 30t target indicator (`0x080EE340`), falls diagonally 16t (`10-0a`), lands with `hit_mod=3` on all hits, coll `0x18/0x19` (piercing+breaking), and does **NOT** crack solid panels (row 1 byte 5 = `0x00`). Waits 90t before undimming. | Fam 12 sub 55; Dim `eff=0x2E` (`0x080FB96C`, panel selector `0x080FBA98`); `AttackObj[0x0A]` row 1 (`0x080D6C2C`, elem `2` Heat, coll `0x18/0x19`, `hit_mod=3`). Sprite: `10-0a` anim 0, landing FX `0x4F`. SFX: `0xE1` (meteor fall), `0x76` (impact), shake `(1, 15)`. |
| `poltrgst` | `0x0D5` | `content/exe3/chips/mega/init.luau` | **P0** | **Missing obstacle flight + wrong alliance filter + wrong collision/scaling**: Saito only queries `field.all_objects(me.alliance)` (misses enemy obstacles), instantly deletes them with `PUFF_FX`, spawns direct non-breaking `collision.piercing` hitboxes at 8t intervals using `me.damage`. ROM sets battle flag `0x1B`/`0x1A`; **ALL obstacles on BOTH sides** lift (`z+=2` for 32t, SFX `0x93`), shake 30t, target a random enemy actor panel (fallback random enemy panel, `0x080B0148`), fly across the field, and impact for hardcoded **150** Null dmg (`0x080B010C`, coll `0x12/0x13` **piercing+breaking**, `hit_mod=3`) before breaking; dim ends 30t after flag `0x19` clears. | Fam 12 sub 44; Dim `eff=0x25` (`0x080FB080`); Obstacle throw FSM `0x080AFFEC..0x080B01E4` (hit `0x080B0102`, `r4=0x12130001`, `r6=150`, `hit_mod=3`). Uses each obstacle's own sprite. SFX: `0x93` (lift). |
| `jealousy` | `0x0D6` | `content/exe3/chips/mega/init.luau` (`@exelib/jealousy/*`) | **P0** | **Missing chip destruction!** ROM spawns `0x080D6876` with `r4=0x2D2E0901` -> coll `0x2D/0x2E` (`self=0x80000495`, `hit=0x1A100081`: bit `0x400` **destroys opponent's chips in hand**, plus piercing+breaking!), `hit_mod=3` on every hit, and waits **`30t` (`0x1E`)** after the last hit before undimming (`0x080FAFC8`). Saito uses `collision.piercing` (never destroys enemy hand chips!), `hit_mod=1` until last hit, and waits `90t` (`0x5A`) after last hit. | Fam 12 sub 43; Dim `eff=0x24` (`0x080FAEEC`, count `0x080FAFE0`, hit `0x080FB016`, `r4=0x2D2E0901`, `hit_mod=3`). Spark `0x09` (hit-spark). |
| `standout` | `0x0D7` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Heat Style check + completely fabricated mechanics**: ROM is **non-dimming**, requires **Heat Style** (`[r5,#0x17]==2`, else fizzle `0x15`), wraps MegaMan in a 75t fire pillar (`AttackObj[0x3B]`, `10-22`, 60t invincibility), and after 15t checks 3 directions `(0,-1)`, `(0,+1)`, `(+1,0)` for a **HOLE panel**; for each direction containing a hole, sends travelling flames (`AttackObj[0xAA]`, `0c-38`, stepping every 12t, `hit_mod=3`) into the hole. Saito wrongly dims, has no Heat Style/hole/invincibility mechanics, and blasts `regions.row` with `FLAME_FX` + `BLAST_SOUND`. | Fam 59 sub 0 (`0x080B43D2`), `b11=0x01`, dir table `0x080B44AC`; `AttackObj[0x3B]` (self pillar) + `AttackObj[0xAA]` (hole-seeking flame). Sprites: `10-22` (pillar), `0c-38` (flame). SFX: `0x80` (fire). |
| `watrline` | `0x0D8` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Aqua Style & front-hole check + fabricated rebound**: ROM is **non-dimming**, requires **Aqua Style** (`[r5,#0x17]==3`, else fizzle `0x15`), checks `(x+facing, y)` for an **empty HOLE** (`req=0x10000, forbid=0x3E000010`); if not a hole, deals 10-dmg fizzle (`0x080D684C`, SFX `0x90`). If hole, spawns `AttackObj[0xAB]` (`10-23`, SFX `0xB2`) -> `AttackObj[0xAC]` (`0c-16`, SFX `0xAE`) stepping **forward only** (12t/panel, NO rebound), **cracking** solid panels (`0x0800B568`, `hit_mod=3`). Saito wrongly dims, skips Aqua Style & hole check, sweeps forward+back, doesn't crack panels, plays `0xAC`. | Fam 60 sub 0 (`0x080B44DA`), `b11=0x01`; `AttackObj[0xAB]` (`0x080ECCB4`) -> `AttackObj[0xAC]` (`0x080ECDDD`, crack `0x080ECE96`, `hit_mod=3`, elem `3` Aqua). Sprites: `10-23`, `0c-16`. SFX: `0x90` (fizzle), `0xB2` (geyser), `0xAE` (water step). |
| `ligtning` | `0x0D9` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Elec Style & obstacle-targeting + wrong AoE**: ROM is **non-dimming**, requires **Elec Style** (`[r5,#0x17]==1`, else fizzle `0x15`), flashes screen (SFX `0xC5`, `SpawnPanelEffect(0x0400)`), sweeps columns every 4t checking `flags & 0x0E000000` (**field obstacles/objects**), and spawns `AttackObj[0xAD]` (`14-0e`, SFX `0xBB`, region **`0x10` = 8 surrounding panels**, `hit_mod=3`, Elec) on each obstacle. Saito wrongly dims, skips Elec Style check, directly targets `enemy_target` instead of obstacles, uses `regions.plus`, and plays `BLAST_SOUND` (`0x76`). | Fam 61 sub 0 (`0x080B459C`), `b11=0x01`; `AttackObj[0xAD]` (`0x080ECF6C`, region `0x10` @ `0x080AF438`, elem `1` Elec, `hit_mod=3`). Sprite: `14-0e`. SFX: `0xC5` (flash), `0xBB` (lightning). |
| `gaiaswrd` | `0x0DA` | `content/exe3/chips/mega/init.luau` | **P0** | **Missing Wood Style check + wrong single-chip absorption**: ROM requires **Wood Style** (`[r5,#0x17]==4`, else fizzle `0x15` at `0x080B1722`). Custom-screen absorption (`0x0800F894`) absorbs **ALL subsequent non-PA chips in hand** (not just the single next chip!), keeps PAs (`id>=0x140`), adds `dmg + atk_bonus` for attack chips (`b11 & 0x15 != 0`, excluding variable-power `CustSwrd`, `Muramasa`, and 9 Virus chips) while consuming all non-PA chips after it. Wrong presentation: uses `0c-0d` (`big-slash`) pal 0, `0c-0b` (`wood-sword`), SFX `0xD6` instead of `0c-0c` (`sword-slash`) anim 0 **pal 8**, `0c-00` (`sword`), SFX `0x75`. | Fam 8 sub 11 (`0x080B16CE`, style check `0x080B1722`); Absorption `0x0800F894..0x0800F960`; Sword table `0x080D65CC` row 11 (`00 08 0c 04 00 02 00 03`: anim 0, pal 8, `0c-0c`, region `0x04` Wide, `hit_mod=3`). Attached: `0c-00`. SFX: `0x75`. |
| `salamndr` | `0x111` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Heat Style check + fabricated `deep_fan` blast**: Shares exact ROM handler with `StandOut` (`Family 59`, `0x080B43D2`, differing only in power `300` vs `220`): **non-dimming**, requires **Heat Style**, 75t self fire-pillar + 60t invincibility, hole-seeking flames in 3 directions `(0,-1)`, `(0,+1)`, `(+1,0)`. Saito wrongly dims and fires an instant `regions.deep_fan` blast. | Fam 59 sub 1 (`0x080B43D2`), `b11=0x01`; `AttackObj[0x3B]` + `AttackObj[0xAA]`. Sprites: `10-22`, `0c-38`. SFX: `0x80`. |
| `fountain` | `0x112` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Aqua Style & front-hole check + wrong `wide` & rebound sweep**: Shares exact ROM handler with `WatrLine` (`Family 60`, `0x080B44DA`, power `240` vs `180`): **non-dimming**, requires **Aqua Style**, requires **empty HOLE** at `(x+facing, y)` (else 10-dmg fizzle), sweeps **single-row forward only** cracking panels. Saito wrongly dims, skips Aqua Style & hole check, uses `regions.wide` with forward+rebound sweep, and doesn't crack panels. | Fam 60 sub 1 (`0x080B44DA`), `b11=0x01`; `AttackObj[0xAB]` -> `AttackObj[0xAC]` (`0x080ECDDD`). Sprites: `10-23`, `0c-16`. SFX: `0x90`, `0xB2`, `0xAE`. |
| `bolt` | `0x113` | `content/exe3/chips/mega/init.luau` | **P0** | **Dimming mismatch + missing Elec Style & obstacle-targeting + wrong `square` AoE**: Shares exact ROM handler with `Ligtning` (`Family 61`, `0x080B459C`, power `210` vs `160`): **non-dimming**, requires **Elec Style**, targets **field obstacles** (`flags & 0x0E000000`) with region `0x10` (8 surrounding panels). Saito wrongly dims, skips Elec Style check, targets `enemy_target` directly with `regions.square` and `BLAST_SOUND`. | Fam 61 sub 1 (`0x080B459C`), `b11=0x01`; `AttackObj[0xAD]` (`0x080ECF6C`, region `0x10`). Sprite: `14-0e`. SFX: `0xC5`, `0xBB`. |
| `gaiablad` | `0x114` | `content/exe3/chips/mega/init.luau` | **P0** | **Missing Wood Style check + wrong single-chip absorption + WRONG SWORD RANGE (2x3 vs 2x1!)**: Same missing Wood Style check (`0x080B1722`) and multi-chip absorption (`0x0800F894`) as `GaiaSwrd`, PLUS ROM sword table `0x080D65CC` row 12 (verified identical in both `bn3` & `bn3b`!) specifies region **`0x02` = Long (2x1)** (`(0,0),(1,0)`), NOT `regions.wide_long` (2x3)! Also wrong sprite/pal/SFX (`0c-0d` pal 0 / `0c-0b` / `0xD6` instead of `0c-0c` anim 1 **pal 8** / `0c-00` / `0x75`). | Fam 8 sub 12 (`0x080B16CE`); Absorption `0x0800F894`; Sword table `0x080D65CC` row 12 (`bn3b 0x080D671C`, bytes `01 08 0c 02 00 02 00 03`: anim 1, pal 8, `0c-0c`, region `0x02` Long 2x1, `hit_mod=3`). Attached: `0c-00`. SFX: `0x75`. |

---

##### Part 2: Giga Chips (`content/exe3/chips/giga/init.luau` & `content/exe3/chips/pa/init.luau`)

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| `deltaray` *(Blue)* | `0x12E` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing A-button timing prompt, Delta warp vertices, & triangle finisher**: Saito always performs 3 unconditional slashes on `(tx-facing, ty)` with `hit_mod` `1,1,3` and `big-sword-swing` (`0xD6`). ROM warps ProtoMan to `(tx-dir, ty-1)` (must be solid & unoccupied, else fails!), executes slash 1 (`hit_mod=3`, Wide `0x04`, coll `0x2B/0x2C`, SFX `0x75`), then polls **A button** (`0x080D2518`) during the timing window; only on timely A-press + valid next triangle vertex (`0x080D250C`: `(+2,+1,flip)` then `(-2,+1,flip)`) does ProtoMan warp for slash 2 & slash 3! Slash 3 spawns Delta triangle FX `0x4C`/`0x4D` (pal `0x35`), screen flash, and SFX `0xDF`. | Fam 20 sub 74; Dim `eff=0x46` (`bn3 0x080FD750` / `bn3b 0x080FD840`); `Type1[0x5C]` (`0x080D1F98`, warp `0x080D24B8`, A-check `0x080D2518`). Sprite: `08-09` (ProtoMan), FX `0x27`/`0x4C`/`0x4D`. SFX: `0x75` (slashes), `0xDF` (Delta finish). |
| `foldrbak` *(Blue)* | `0x12F` | `content/exe3/chips/giga/init.luau` | **P0** | **Fabricated +200 HP heal + missing Custom Gauge fill**: Saito restores folder, fabricates a `+200 HP` heal (`HEAL_FX`, `RECOVER_SOUND 0x72`), and never fills the Custom Gauge. ROM plays SFX `0x120`, shakes camera `(3, 70)`, palette flash (`0x00014600`), waits 70t, calls `0x08010B20` (**fills Custom Gauge to 100% = `0x4000`**), calls `0x08011454` (restores & reshuffles full 30-chip folder including FoldrBak itself), sets `[battle+0x36]=1`, and **NEVER heals HP**. | Fam 12 sub 61; Dim `eff=0x34` (`bn3 0x080FC41C` / `bn3b 0x080FC50C`); FillGauge `0x08010B20`; RestoreFolder `0x08011454`. SFX: `0x120`, shake `(3, 70)`, palette flash `0x00014600`. |
| `navrcycl` *(White)* | `0x130` | `content/exe3/chips/giga/init.luau` | **P0** | **Wrong tracking scope (per-side & records non-Navi Mega chips), drops Atk+ buffs, wrong fizzle**: Saito tracks per-side only, records ANY `class=="mega"` chip (wrongly copying Guardian/Meteors/Jealousy/StandOut/etc.!), drops Atk+/Navi+ bonuses, and dims when nothing is recorded. ROM tracks ONLY **Navi chips** (Fam 20 via `0x08007938`) in a **single global slot** (`0x020380C0`) shared across **both players** (preserving boosted damage word!). Fizzles **non-dimmed** (action `0x15`) if empty. On dim, spawns `EffectObj[0x25]` (`0c-37`, 90t, SFX `0xB1`) then executes saved dimming effect with saved damage. | Fam 53 sub 0 (`0x080B40F8`); Dim `eff=0x37` (`0x080FC6D0`); Recorder `0x08007938` (`0x020380C0`); `EffectObj[0x25]` (`0x080F05EC`). Sprite: `0c-37`. SFX: `0xB1`. |
| `alpharmsg` *(White)* | `0x131` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing Alpha Arm summon sprite + completely fabricated attack**: Saito renders NO sprite at all and fires 8 instant full-enemy-area `BLAST_FX` hits every 5t with `CANNON_SOUND` (`0x7D`). ROM summons Alpha's Arm (`Type1[0x63]` `r4=0`, sprite **`08-07`**, missing from `assets.toml`!) at `(2,2)`/`(5,2)`, clears obstacles, plays anim `0x0C` (SFX `0x93`), and spawns `AttackObj[0xA4]` firing **`16` machinegun/claw volleys** every **`10` ticks** via `AttackObj[0xA5]` (sprite `10-5c`, SFX `0xBC`/`0x139`, `hit_mod=3`, camera shake `(1,1)`). | Fam 20 sub 84; Dim `eff=0x4D` (`0x080FDC18`); `Type1[0x63]` (`0x080D46B0`, `r4=0`); `AttackObj[0xA4]` (`0x080EBF88`) -> `AttackObj[0xA5]` (`0x080EC128`). Sprites: `08-07` (Alpha Arm), `10-5c` (shots). SFX: `0x93`, `0xBC`, `0x139`. |
| `bass` *(White)* | `0x132` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing DarkHole/DarkLicense requirement + fabricated random-panel bursts**: ROM (`0x080474DE -> 0x080B595C`) **requires DarkHole OR DarkLicense** (else fizzles `0x15`!). Summons Bass (`08-12`, anim 6 for 60t, SFX `0x8F` x2), then fires **`15` (`0x0F`) AirBurst projectiles** (`AttackObj[0x93]`, sprite `10-50`, coll `0x12/0x13` piercing+breaking, `hit_mod=3`) from `(x+dir, y+rand(-1..1))` advancing forward every 7t flashing panels, cadence `[8,12,16]`t, SFX `0xE0`. Saito has no Dark check and fires 10 instant random-panel `FLASH_FX` hits every 4t with `CANNON_SOUND` (`0x7D`) and `hit_mod=1..3`. | Fam 20 sub 75 (`0x080474DE` Dark check); Dim `eff=0x47` (`0x080FD7E8`); `Type1[0x5D]` (`0x080D2540`); `AttackObj[0x93]` (`0x080E99E4`, step `0x080E9A94`). Sprites: `08-12` (Bass), `10-50` (AirBurst). SFX: `0x8F`, `0xE0`. |
| `serenade` *(White)* | `0x133` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing DarkHole/DarkLicense check + fabricated full-field bursts**: ROM (`0x080474D8 -> 0x080B595C`) **requires DarkHole OR DarkLicense** (else fizzles `0x15`!) and solid standing panel (`0x0800BF4A`). Summons Serenade (`08-11`, anim 3→4) spawning `AttackObj[0xB1]` (anim `0x0A`, 240t, SFX `0xF3`), which every **15t** (16 waves) picks up to **2 random enemy panels** (`0x080ED7A8`) and spawns HolyShock pillars (`AttackObj[0xA7]`, `10-50`, panel flash telegraph -> explosion, SFX `0xBB`). Saito skips Dark check and blasts the entire enemy area 4 times every 8t with `ENERGY_RING_FX` + `BLAST_SOUND`. | Fam 20 sub 87 (`0x080474D8` Dark check); Dim `eff=0x54` (`0x080FE1EC`); `Type1[0x67]` (`0x080D5900`); `AttackObj[0xB1]` (`0x080ED4E8`, wave `0x080ED7A8`) -> `AttackObj[0xA7]` (`0x080EC67C`). Sprites: `08-11`, `10-50`. SFX: `0xF3`, `0xBB`. |
| `balance` *(White)* | `0x134` | `content/exe3/chips/giga/init.luau` | **P1** | **Rounding bug (floor vs ceil) + misses non-player actors + presentation/timing**: ROM (`0x080FDBBC..0x080FDBD4`) iterates ALL 5 battle actor slots and computes **`(hp + 1) >> 1`** (ceiling half: `101→51`, `3→2`, `1→1`). Saito uses `math.floor(hp / 2)` (`101→50`, `3→1`) and only loops `side=0,1`. ROM plays `SpawnPanelEffect(0x00010400)`, `EffectObj[0x0E]` (`r4=2` -> lightning `0x2C` + SFX `0xBB`), shake `(3, 60)` x3 over `90+20+20+120`t (vs Saito 40t + `ENERGY_RING_FX` + `BLAST_SOUND 0x76`). | Fam 12 sub 72; Dim `eff=0x4C` (`0x080FDAF4`, loop `0x080FDBBC`); `EffectObj[0x0E]` (`0x080EED14`, table `0x080EED0C` entry 2 = `2C BB`). SFX: `0xBB`, shake `(3, 60)` x3. |
| `darkaura` *(Blue)* | `0x135` | `content/exe3/chips/giga/init.luau` | **P0** | **Dimming mismatch + missing DarkHole/DarkLicense requirement + wrong duration & palette**: ROM is **non-dimming** (Fam 3 sub 8), **requires DarkHole OR DarkLicense** (`0x080B1190 -> 0x080B595C`, else fizzles `0x15`!), applies barrier `0x20` with threshold `300` (`0x080AD99E`), duration **`3000` ticks (50s)** (NOT `1800` = 30s!), cleared by Wind (`0x080AD896`), NO HP drain, sprite `0c-04` anim 0 **palette 2** (`0x080EDFF4[8]`), SFX `0xCE`. Saito wrongly dims, skips Dark check, uses `timer=1800`, and uses `barriers.life_aura` (**palette 1**). | Fam 3 sub 8 (`0x080B117C`, Dark check `0x080B1190`); Barrier absorb `0x080AD984` (`cmp r0, #300`); Visual table `0x080EDFF4[8]`. Sprite: `0c-04` anim 0 **pal 2**. SFX: `0xCE`. |
| `alpharmom` *(Blue)* | `0x136` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing Alpha Arm summon sprite + completely fabricated rocket-fist attack**: Saito renders NO sprite at all and fires a single instant `regions.square` `BLAST_FX` hit on the enemy target. ROM summons Alpha's Arm (`Type1[0x63]` `r4=1`, sprite **`08-07`**) playing anim `0x0E` (SFX `0x93`), waits 20t, and fires `AttackObj[0xA6]` (rocket fist, sprite **`10-5d`**, anim 0→1→2, SFX **`0x94`**, flies horizontally at `4 px/t`, coll `0x1C/0x13` breaking, region `0x10` = 8 surrounding panels, `hit_mod=3`) for up to 120t. | Fam 20 sub 85; Dim `eff=0x4E` (`bn3 0x080FDCB0` / `bn3b 0x080FDDA0`); `Type1[0x63]` (`0x080D46B0`, `r4=1` @ `0x080D496C`); `AttackObj[0xA6]` (`0x080EC320`). Sprites: `08-07`, `10-5d`. SFX: `0x93`, `0x94`. |
| `bass-137` (Bass+) *(Blue)* | `0x137` | `content/exe3/chips/giga/init.luau` | **P0** | **Missing DarkHole/DarkLicense requirement + missing teleport + wrong EarthBreaker shape & anim**: ROM (`0x080474DE -> 0x080B595C`) **requires DarkHole OR DarkLicense** (else fizzles `0x15`!). Summons Bass (`08-12`), scans forward for nearest enemy (`0x080D2B94`), **teleports to `(enemy_col - dir, y = 2)`** (fails if blocked!), plays anim **`0x0B`** (not `0x0D`), and spawns 3 `AttackObj[0x94]` (`10-51`, SFX `0xB2`, coll `0x1A/0x19` breaking, `hit_mod=3`, breaks panels via `0x0800B69C`) across the **3-panel vertical column `(x+dir, y=1..3)`** (NOT `regions.plus` 5 panels!) + shake `(2, 60)`. Saito skips Dark check, doesn't teleport, uses anim `0x0D`, and breaks 5 panels in `regions.plus`. | Fam 20 sub 76 (`0x080474DE` Dark check); Dim `eff=0x48` (`bn3 0x080FD880` / `bn3b 0x080FD970`); `Type1[0x5E]` (`0x080D2874`, target `0x080D2B94`, slam `0x080D2C18`); `AttackObj[0x94]` (`0x080E9B3C`). Sprites: `08-12`, `10-51`. SFX: `0xB2`, shake `(2, 60)`. |
| `bassgs` *(Both)* | `0x138` | `content/exe3/chips/pa/init.luau` (`L1923-L1982`) | **P0** | **Missing Bass summon sprite + wrong Vanishing World hitbox**: Saito renders NO Bass sprite, doesn't hide MegaMan, and fires a single full-enemy-area `BLAST_FX` hit with `BLAST_SOUND`/`CRACK_SOUND`. ROM (no DarkHole/DarkLicense check!) checks solid standing panel (`0x0800BF4A`), hides MegaMan, summons Bass (`08-12`, anim `0x0D`), spawns `AttackObj[0x96]` (`10-54`, SFX `0x93`) -> beam (`0x080EA330`, SFX `0xD3`) hitting the **8-panel diamond/hexagonal Vanishing World blast** (`0x080EA4A4`: `x+1` single, `x+2` 3-row, `x+3` 3-row, `x+4` single; coll `0x22/0x23` piercing+breaking, `hit_mod=3`). | Fam 20 sub 77 (no Dark check); Dim `eff=0x49` (`0x080FD918`); `Type1[0x5F]` (`0x080D2C58`); `AttackObj[0x96]` (`0x080E9E1C`) -> beam `0x080EA330` / hitbox `0x080EA4A4` (`0x080EA4EC`: `0x2223FE01/04/04/01`). Sprites: `08-12`, `10-54`. SFX: `0x93`, `0xD3`. |

---

#### Detailed Findings & ROM Evidence Notes

##### 1. Guardian (`0x0CB`) — P1
- **Retaliation Element (`0x080DC0B4`)**: `mov r0, #0x1` is passed into `SpawnSimpleAttackObj` (`0x080D684C`), which writes `r0` to `[obj + 0x17]` (**Element = `1` = Elec**, mapping to Elec spark `0x03` via `0x080ADB80`). Saito's `content/exelib/guardian/statue.luau` uses `element.null` (missing 2x damage vs Aqua enemies and Elec status/style interactions).
- **Retaliation Collision & Targeting (`0x080DC04C`, `0x080DC0F8`)**:
  - Attacker trigger masks at `0x080DC09C`/`0x080DC0A0` are `0x52000000` (side 0) and `0xA4000000` (side 1) vs `exelib` `0xAA000000`/`0x55000000`.
  - Strike `r4 = 0x22230285` (side 0) / `0x22230284` (side 1): collision `0x22/0x23` (`self=0x80000095`, `hit=0x1A000081`: piercing + breaks guard), spark `0x02`, bit-7 region `0x85` (`req=0x00400000`) / `0x84` (`req=0x00800000`) targeting only the opposing Navi's panel, `hit_mod = 3`.
- **Timing & Presentation**: Strike dim `eff=0x0C` (`0x080F94FC`) holds for `60t` (`0x3C`), plays lightning SFX `0xBB` (`0x080DC0E4`), camera shake `(1, 60)`, and spawns visual `0x2C` via `0x08001110`. Statue sprite at `0x080DBF20` is `10-1F` (not `0c-29` in `assets.toml`).

##### 2. ZeusHamr (`0x0D0`) — P0
- **Non-Dimming Action (`0x080B3B0E`)**: Family 47 (`b11=0x01`) is a standard non-dimming player action (anim `0x24` windup with attached hammer `0c-13` pal 1, anim `0x25` recovery). Saito wrongly flags it `"dimming"`.
- **Front-Panel Solid Check**: On anim flag `0x81`, ROM checks `(x + facing, y)` for `flags & 0x10` (solid).
  - **Solid**: Plays SFX `0x7E`, flashes front panel (`0x0800B76C`), shakes camera `(2, 60)`, and spawns `AttackObj[0x4D]` (`0x080DFA14`) with collision `0x48` (`0xC0000010` on both sides — hits ALL grounded actors on both teams!) and regions `0x87` (`req=0x10, forbid=0x20`: all solid side-0 panels) and `0x88` (`req=0x30`: all solid side-1 panels) across ticks 1 & 2, `hit_mod = 3`. Because MegaMan is hit via the normal collision pipeline, **Barriers, Auras, Invis, Shadow, AirShoes (float), HolyPanel, and flinch all apply naturally**!
  - **Hole / Off-Field**: Does **NOT** earthquake! Only spawns a single-panel hit (`0x080D684C`, `0x01020901`, `hit_mod=3`) on `(x + facing, y)` with SFX `0x7E` (if entity on hole) or `0xCD` (miss).

##### 3. GodStone (`0x0D1`) & OldWood (`0x0D2`) — P0
- **Strict Front-Hole Requirement (`AttackObj[0x50]` @ `0x080DFF78`)**: Both chips drop orb `0c-0e` (SFX `0x78`) strictly onto `(x + facing, y)` and test `CheckPanelFlags(require=0, forbid=0x3E000010)` — the panel in front MUST be an **unoccupied Hole** (no solid bit `0x10`, no actors/obstacles). If not, spawns fizzle FX `0x35` + SFX `0x112` and aborts!
- **GodStone (`Type1[0x29]` @ `0x080C10C8`)**: Sprite `0c-2b`, invincible/untargetable; spawns 2 floating hands `AttackObj[0x51]` (`0c-2c`) at `(x, y+1)` and `(x, y-1)`. Triggers 3 slams every `90t` (`0x5A`); each hand slams (SFX `0x7E`/`0x7F`, shake `(2, 30)`) and calls `0x080E0206`: shuffles enemy-area panels (`0x20`), selects `min(count, 3 + rand(0..2))` = **3 to 5** panels, and spawns falling boulders `AttackObj[0x52]` (`0x080E02A4`, sprite `0c-2d`, `z=256`, `vz=-8`, coll `0x1C/0x13` breaking, `hit_mod=3`, SFX `0x95` + debris FX on landing; does not crack panels).
- **OldWood (`Type1[0x2A]` @ `0x080C12B4`)**: Sprite `0c-2e`, fades in over 32t (SFX `0x92`), waits 40t, collects ALL solid enemy-area panels (`0x080C1494`), shuffles them, and spawns one **single-panel** (`region 0x01`) Wood spike (`AttackObj[0x53]` row 4, `0c-31` anim 0→1→2, SFX `0x9C`, `hit_mod=1`, panel flash) every 15t on every solid enemy panel (up to 9 panels).

##### 4. Meteors (`0x0D4`) — P1
- `DimEff[0x2E]` (`0x080FB96C`) spawns **`30` (`0x1E`)** meteors every **`10` ticks** via deterministic enemy-panel iterator `0x080FBA98` (front column top-to-bottom, cycling columns), waiting `90t` after the 30th meteor before undimming.
- `AttackObj[0x0A]` (`0x080D6C2C`, `r4=1`, `elem=2` Heat) spawns ground target indicator (`0x080EE340`), waits `30t`, plays SFX `0xE1`, falls diagonally over `16t` (`10-0a` anim 0), explodes with FX `0x4F` + SFX `0x76` + shake `(1, 15)` + coll `0x18/0x19` + `hit_mod=3`. Table row 1 byte 5 is `0x00` (never cracks solid panels).

##### 5. Poltrgst (`0x0D5`) — P0
- `DimEff[0x25]` (`0x080FB080`) sets battle flag `0x1B` (side 0) or `0x1A` (side 1) and waits until flag `0x19` clears (kept set while any obstacle is mid-flight) + `30t`.
- Obstacle throw FSM (`0x080AFFEC..0x080B01E4`): **Every obstacle on the field (both sides!)** saves its original side (`[r5, #0x84]`), temporarily adopts the caster's side (`[r5, #0x16]`), lifts (`z += 2` for 32t, SFX `0x93`), shakes 30t, picks a random enemy actor panel (fallback random enemy panel via `0x080B0148`), flies across the field, impacts for **hardcoded `150` Null damage** (`0x080B010C`, `r4=0x12130001` = coll `0x12/0x13` piercing+breaking, `hit_mod=3`), and destroys itself.

##### 6. Jealousy (`0x0D6`) — P0
- `DimEff[0x24]` (`0x080FAEEC`) counts remaining chips in the opponent's hand (`0x080FAFE0`) and spawns `0x080D6876` every 10t with `r4 = 0x2D2E0901`, `hit_mod = 3` on opposing Navi panels (`0x00400000`/`0x00800000`).
- Collision table `0x080AF51C` entries `0x2D`/`0x2E`:
  - `0x2D` (self): `0x80000495` / `0x40000495` — bit **`0x00000400`** is the **chip-destruction flag** (destroys opponent's chips in hand on hit!), alongside piercing/breaking bits `0x95`!
  - `0x2E` (hit): `0x1A100081` / `0x2C100081`.
- Post-hit wait before undim at `0x080FAFC8` is `0x1E` (**30 ticks**), not 90 ticks.

##### 7. Elemental Mega Pairs (`standout`/`salamndr`, `watrline`/`fountain`, `ligtning`/`bolt`, `gaiaswrd`/`gaiablad`) — P0
- **Style Checks**: All 8 chips check `[r5, #0x17]` (player's Style element) and fizzle to action `0x15` if mismatched (`2` Heat, `3` Aqua, `1` Elec, `4` Wood).
- **Non-Dimming**: All 8 are non-dimming (`b11=0x01`). Saito wrongly marks the first 6 as `"dimming"`.
- **No Subfamily Split in Heat/Aqua/Elec Pairs**: `Family 59` (`0x080B43D2`), `Family 60` (`0x080B44DA`), and `Family 61` (`0x080B459C`) never read `subfamily`; `StandOut`/`Salamndr`, `WatrLine`/`Fountain`, and `Ligtning`/`Bolt` have **identical mechanics/shapes** and differ only in damage (`220/300`, `180/240`, `160/210`).
- **GaiaSwrd (`0x0DA`) & GaiaBlad (`0x114`) Absorption (`0x0800F894..0x0800F960`)**:
  - Runs at custom-screen hand commit for `0xDA` and `0x114`.
  - Loops over **all chips selected after GaiaSwrd/GaiaBlad in the same turn**:
    - Program Advances (`id >= 0x140`) are **preserved** (not absorbed, not deleted).
    - All other following chips are **consumed (`0xFFFF`)** and compacted out of the hand.
    - Damage (`base + atk_bonus`) is added iff `(b11 & 0x15) != 0` AND `id` is not one of the 11 variable-power (`power=10000`) excluded chips: `0x25` (`CustSwrd`), `0xCA` (`Muramasa`), and the 9 Virus chips (`0x4C, 0x4D, 0xA6, 0xA7, 0xB9..0xBD`).
  - **GaiaBlad Range Verified in BOTH `bn3` (`0x080D662C`) & `bn3b` (`0x080D671C`)**: Row 12 bytes are `01 08 0c 02 00 02 00 03` -> anim `1`, **pal `8`**, sprite `0c-0c`, region **`0x02` = Long (2x1)** (`(0,0), (1,0)` @ `0x080AF3C0`), NOT `0x11` (2x3 LifeSwrd)!

##### 8. Dark Chip Activation Gate (`0x080B595C`) — Affects `darkaura`, `bass`, `bass-137`, `serenade`
- `0x080B595C` returns non-zero iff:
  1. `[0x02037920 + side * 0x40 + 0x20] != 0` (**DarkLicense** NaviCust part installed), **OR**
  2. `CheckBattleFlag(0x16 + side) != 0` (**DarkHole** active on caster's side, set by the `Hole` chip at `0x080E391E`).
- Called at:
  - `0x080B1190`: Family 3 sub 8 (`DarkAura`)
  - `0x080474D8..0x080474E6`: Family 20 sub 75 (`Bass`), sub 76 (`Bass+`), and sub 87 (`Serenade`).
- Note: Family 20 sub 77 (`BassGS`) is intentionally **NOT** gated by `0x080B595C`.

##### 9. Blue-Exclusive Giga Chips (`deltaray`, `foldrbak`, `darkaura`, `alpharmom`, `bass-137`)
- **`deltaray` (`0x12E`)**: `Type1[0x5C]` (`0x080D1F98`) checks A-button press (`[input+0x3C] & 1`) during the post-slash window (`[r5,#0x19] != 0`) at `0x080D2518`. Vertex offset table `0x080D250C`: `{0,0,+1}`, `{+2,+1,-1}`, `{-2,+1,+1}` (in `(dx*dir, dy, facing_flip)`). Missing A or an invalid/occupied vertex terminates the combo early after 1 or 2 slashes.
- **`foldrbak` (`0x12F`)**: `bn3b 0x080FC50C` (`bn3 0x080FC41C`) strictly refills Custom Gauge (`0x08010B20`, `[battle+0x2C] = 0x4000`) and restores/reshuffles the folder (`0x08011454`); zero HP recovery exists anywhere in the handler.
- **`darkaura` (`0x135`)**: Verified in `0x080AD890..0x080AD9B4` that barrier ID `0x20` blocks `< 300` dmg (`cmp r0, #300`), decrements `[r7, #0x16]` (`3000` ticks = 50s) by 1 per frame, breaks on `>= 300` dmg or Wind (`r1 & 0x40`), and has **no HP drain** in BN3. Visual table `0x080EDFF4[8]` uses `0c-04` anim 0 **palette 2** (purple) + SFX `0xCE`.
- **`alpharmom` (`0x136`)**: `Type1[0x63]` (`r4=1`, `08-07`) launches `AttackObj[0xA6]` (`10-5d`, SFX `0x94`) flying horizontally at `4 px/t` with `region 0x10` (8 surrounding panels) and breaking collision `0x1C/0x13`.
- **`bass-137` (`0x137`)**: `Type1[0x5E]` (`08-12`) teleports to `(nearest_enemy_col - dir, y = 2)` (must be unoccupied), anim `0x0B`, and spawns 3 `AttackObj[0x94]` (`10-51`, SFX `0xB2`, breaks panels) in the vertical 3-panel column `(x + dir, y = 1..3)`.

##### 10. White/Shared Giga Chips (`navrcycl`, `alpharmsg`, `bass`, `serenade`, `balance`, `bassgs`)
- **`navrcycl` (`0x130`)**: Global slot `0x020380C0` records `{eff_id, param, boosted_damage_word}` ONLY inside common Navi dimming init `0x0800906E -> 0x08007938` (excluding `eff 0x50, 0x51` and `eff 0x07` with `param >= 5`). Thus it copies **either player's** last Navi chip with its Atk+/Navi+ boost intact, and never copies non-Navi Mega chips.
- **`alpharmsg` (`0x131`)**: `Type1[0x63]` (`r4=0`, `08-07`) spawns `AttackObj[0xA4]` firing 16 volleys every 10t via `AttackObj[0xA5]` (`10-5c`, SFX `0xBC`/`0x139`, `hit_mod=3`).
- **`bass` (`0x132`)**: `Type1[0x5D]` (`08-12`) fires 15 `AttackObj[0x93]` AirBurst shots (`10-50`, SFX `0xE0`, stepping every 7t, coll `0x12/0x13`, `hit_mod=3`).
- **`serenade` (`0x133`)**: `Type1[0x67]` (`08-11`) spawns `AttackObj[0xB1]` (240t, SFX `0xF3`), triggering every 15t (`0x080ED7A8`) to spawn up to 2 `AttackObj[0xA7]` HolyShock pillars (`10-50`, flash telegraph -> explosion, SFX `0xBB`) on random enemy panels.
- **`balance` (`0x134`)**: `0x080FDBBC..0x080FDBD4` halves all 5 battle actor slots using **`(hp + 1) >> 1`** (ceiling half), accompanied by `EffectObj[0x0E]` (`r4=2`, lightning `0x2C`, SFX `0xBB`) and 3 camera shakes `(3, 60)`.
- **`bassgs` (`0x138`)**: `Type1[0x5F]` (`08-12`, anim `0x0D`) spawns `AttackObj[0x96]` (`10-54`, SFX `0x93`) -> beam (`0x080EA330`, SFX `0xD3`) hitting the 8-panel diamond/hexagonal Vanishing World blast (`0x080EA4EC`: col +1 single, col +2 3-row, col +3 3-row, col +4 single; coll `0x22/0x23`, `hit_mod=3`).


---

### 5.6 Group 6：领航员芯片 Part 1（43 张：FlashMan / PlantMan / ProtoMan / BowlMan / MistMan / Roll / GutsMan / BeastMan / BubblMan）

- **ROM Audited**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US, `A6BE`; cross-checked Roll heal & BubblMan shot count against `bn3b.gba` `A3XE`, `exe3.gba`, `exe3b.gba`).
- **Assigned Files**:
  - `content/exe3/chips/navis/init.luau`
  - `content/exe3/chips/navis/helpers.luau`
  - `content/exe3/chips/navis/part1.luau` (43 chips across 9 Navi families: FlashMan V1–V5, PlantMan V1–V5, ProtoMan V1–V5, BowlMan V1–V5, MistMan V1–V5, Roll V1–V3, GutsMan V1–V5, BeastMan V1–V5, BubblMan V1–V5)
  - Consulted: `content/exe3/compat/assets.toml`, `content/exe3/compat/chips.toml`, `content/exe3/rules/collision.luau`, `content/exe3/rules/status.luau`, `content/exe3/rules/init.luau`, `content/exelib/dimming/init.luau`, `content/exelib/regions.luau`, `content/exelib/navi_chips/controller.luau`.

---

#### Executive Summary

| Severity | Count | Navi Families (Chips) |
| :--- | ---: | :--- |
| **P0** | **30** | PlantMan V1–V5 (5), BowlMan V1–V5 (5), MistMan V1–V5 (5), GutsMan V1–V5 (5), BeastMan V1–V5 (5), BubblMan V1–V5 (5) |
| **P1** | **13** | FlashMan V1–V5 (5), ProtoMan V1–V5 (5), Roll V1–V3 (3) |
| **P2** | **0** | *(All 43 chips have P0/P1 mechanics defects; P2 presentation diffs are listed alongside each family)* |
| **OK** | **0** | — |
| **Total** | **43** | All 43 chips in `content/exe3/chips/navis/part1.luau` |

> **Metadata Check (`chips.toml` / ROM `ChipDataTable` `0x08011510`)**: All 43 chips' `codes`, `element`, `power`, `mb`, `rarity`, and `class` (`mega` for V1–V4 / Roll V1–V3, `giga` for V5) match the ROM.

---

#### Cross-Cutting Systemic Findings (`helpers.luau` / `part1.luau`)

1. **Systemic `AntiNavi` Steal/Turnaround Missing (`P0` architectural, affects all 43 chips)**:
   - **ROM**: Family `0x14` (`0x080474A8`) -> `SpawnDimmingChipEffect` (`0x08008E54`) -> every Navi dimming handler (`0x080F9099`, `0x080F9131`, `0x080F92F9`, `0x080F9391`, `0x080F96B5`, `0x080FB2C5`, `0x080FCEC9`, `0x080FD221`, `0x080FD621`) invokes `IsAntiNaviPending` (`0x08007950`) and `CheckAntiNaviTrigger` (`0x08008EA0`), which steals the Navi chip and reverses it to run on the AntiNavi user's side.
   - **Current**: `part1.luau` uses `@exelib/dimming` (`dimming_chips.phases`) instead of `@exelib/navi_chips/controller` (`controller.anti_navi` / `dimming.turn`). In `content/exe3/rules/init.luau:159-163`, `AntiNavi` only **cancels** the opponent's Navi chip with a puff instead of stealing and turning it against the original user.
2. **Systemic Missing User-Panel Solidity Check (`0x0800BF4A`)**:
   - **ROM**: `FlashMan` (`0x080B67E6`), `GutsMan` (`0x080B8F94`), `BeastMan` (`0x080BAE6A`), `PlantMan` (`0x080C244A`), `MistMan` (`0x080CC8EE`), and `BowlMan` (`0x080CDEBE`) all call `0x0800BF4A` (checks solid panel flag `0x10`). If MegaMan is hovering over a hole (AirShoes/FloatShoes), the Navi cannot stand and immediately exits without attacking.
   - **Current**: `helpers.begin_navi` unconditionally materializes the Navi and executes the full attack even over holes.
3. **Roll + `AntiRecv` Ordering Bug**:
   - **ROM**: Roll attacks first (`0x080B6D08..0x080B6DFC`), leaves, and then spawns `RollHeal` (`AttackObj 0x16` @ `0x080D829A`), which applies recovery (`0x080D8246`).
   - **Current**: Because `roll`/`rollv2`/`rollv3` set `family = "recovery"`, `content/exe3/rules/init.luau:164` intercepts and cancels Roll *before* she attacks when the opponent has `AntiRecv`.
4. **Dimming Softlock / Underflow Check**:
   - All 9 controllers in `part1.luau` increment `me.timer += 1` from `0` and terminate via `if me.timer >= ... then h.end_navi(me) end` (calling `dimming_chips.done(me)`). No unsigned underflow or dimming softlocks were found in `part1.luau`.

---

#### Complete Per-Chip Audit Table (All 43 Chips)

*ROM Dispatcher anchor for all 43 chips: Family `0x14` `0x080474A8` -> `NaviDimParamTable` `0x080473E4[sub*2]` -> `DimmingEffectJumpTable` `0x08008D00[eff*4]` -> `SpawnActorObj` (`0x0800304C`) -> `ActorObjJumpTable` (`0x0800369C[obj_id*4]`).*

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :--- | :--- | :---: | :--- | :--- |
| **FlashMan** | `0x0E8` | `navis/part1.luau` | **P1** | Uses `regions.other_area` (misses enemies on stolen panels) instead of ROM `0x85`/`0x84` (all opponent-occupied panels anywhere on field); `hit_mod = 3` (grants flashing i-frames) vs ROM `0` (no i-frames); Paralyze hardcoded 120f vs ROM V1 **90f**; `collision.piercing` missing ROM `0x1F` breaking/guard-pierce bits (`0x80000015`); missing solid-panel check (`0x0800BF4A`); extra camera shake & panel `ELEC_FX`; missing palette cycle & fade-in/out. | `sub=0`, `eff=0x06`, `param=0`; Dim `0x080F9099` -> Spawn `0x080B6A58` -> Actor `0x03` `0x080B6791`; Hitbox `0x080B6944` (`0x080D684C`, `r4=0x1F130385/84`, `r7=0x1000`); Paralyze `0x08019DC0[0]=90f`. Spr: `08-00` (anim 0->6->7). SFX: `0x92` (`appear`), `0xC5` (`flashman-flash`). |
| **FlashMnV2** | `0x0E9` | `navis/part1.luau` | **P1** | Same as FlashMan V1; additionally Paralyze hardcoded 120f vs ROM V2 **140f** (`r7=0x1100`). | `sub=1`, `eff=0x06`, `param=1`; `0x080B6968[1]=0x1100` -> `0x08019DC0[1]=140f`. Spr/SFX same as V1. |
| **FlashMnV3** | `0x0EA` | `navis/part1.luau` | **P1** | Same as FlashMan V1; additionally Paralyze hardcoded 120f vs ROM V3 **190f** (`r7=0x1200`). | `sub=2`, `eff=0x06`, `param=2`; `0x080B6968[2]=0x1200` -> `0x08019DC0[2]=190f`. Spr/SFX same as V1. |
| **FlashMnV4** | `0x0EB` | `navis/part1.luau` | **P1** | Same as FlashMan V1; additionally Paralyze hardcoded 120f vs ROM V4 **190f** (`r7=0x1200`). | `sub=3`, `eff=0x06`, `param=3`; `0x080B6968[3]=0x1200` -> `0x08019DC0[2]=190f`. Spr/SFX same as V1. |
| **FlashMnV5** | `0x0EC` | `navis/part1.luau` | **P1** | Same as FlashMan V1; additionally Paralyze hardcoded 120f vs ROM V5 **190f** (`r7=0x1200`). (Giga class matches ROM.) | `sub=4`, `eff=0x06`, `param=4`; `0x080B6968[4]=0x1200` -> `0x08019DC0[2]=190f`. Spr/SFX same as V1. |
| **PlantMan** | `0x0FC` | `navis/part1.luau` | **P0** | Uses `regions.single` on `enemy_target` instead of ROM `0x88`/`0x87` (ALL solid opponent-area panels); uses `collision.attack` instead of ROM `0x1A` (`0x80000094`, piercing+breaking); sets `hit_mod = 3` on hit 4 (inflicts flashing i-frames, breaking PlantMan's signature bind-combo!) vs ROM `hit_mod = 0` on all 4 hits; applies Elec Paralyze instead of Vine Bind (`0x30`, flag `0x0800`, 120f V1 + vine-wrap visual); missing solid-panel check; wrong anim (`4` vs `3->5`), wrong SFX (`0x74` x4 vs `0xFF` x1), wrong hit interval (10f vs 15f), missing vine effects (`14-17`). | `sub=33`, `eff=0x27`, `param=0`; Dim `0x080FB2C5` -> Spawn `0x080C25F4` -> Actor `0x2E` `0x080C2389`; Vines `0x080C2610` (`EffectObj 0x17` `0x080EF804`), Aura `EffectObj 0x15` `0x080EF5A4`; Hitbox `0x080C252E` (`r4=0x1A190688/87`, `r7=0x3000` on hit 4); Vine Bind `0x080AD560` -> `0x08019DCE[0]=120f`. Spr: `08-04` (anim 0->3->5), `14-17`. SFX: `0x92`, `0xFF`. |
| **PlantMnV2** | `0x0FD` | `navis/part1.luau` | **P0** | Same as PlantMan V1; additionally Vine Bind duration is **180f** (`r7=0x3100`) vs hardcoded 120f Paralyze. | `sub=34`, `eff=0x27`, `param=1`; `0x080C255C[1]=0x3100` -> `0x08019DCE[1]=180f`. Spr/SFX same as V1. |
| **PlantMnV3** | `0x0FE` | `navis/part1.luau` | **P0** | Same as PlantMan V1; additionally Vine Bind duration is **240f** (`r7=0x3200`) vs hardcoded 120f Paralyze. | `sub=35`, `eff=0x27`, `param=2`; `0x080C255C[2]=0x3200` -> `0x08019DCE[2]=240f`. Spr/SFX same as V1. |
| **PlantMnV4** | `0x0FF` | `navis/part1.luau` | **P0** | Same as PlantMan V1; additionally Vine Bind duration is **240f** (`r7=0x3200`) vs hardcoded 120f Paralyze. | `sub=36`, `eff=0x27`, `param=3`; `0x080C255C[3]=0x3200` -> `0x08019DCE[2]=240f`. Spr/SFX same as V1. |
| **PlantMnV5** | `0x100` | `navis/part1.luau` | **P0** | Same as PlantMan V1; additionally Vine Bind duration is **240f** (`r7=0x3200`) vs hardcoded 120f Paralyze. (Giga class matches ROM.) | `sub=37`, `eff=0x27`, `param=4`; `0x080C255C[4]=0x3200` -> `0x08019DCE[2]=240f`. Spr/SFX same as V1. |
| **ProtoMan** | `0x0E3` | `navis/part1.luau` | **P1** | Never checks if landing panel `(opp_x - dir, opp_y)` is solid (`0x10`) and unoccupied (`0x1E000000`/`0x2E000000`) — slashes even when blocked by holes/obstacles/col-1 edge (ROM aborts without attacking!); only targets 1 enemy instead of multi-enemy sequential warp-slash in PvE (`0x080D1BEC`); uses `collision.slash` (non-piercing) vs ROM `0x2B` (`0x8000000C`, sword+piercing); re-queries `enemy_target` on hit frame; plays `SWING_SOUND` (`0xD6`) instead of `0x75` (`sword-swing`), missing warp SFX `0xA8` & warp FX (`14-09` anim 0/1); spawns 3 `SLASH_FX` instead of 1 `0c-0c` at `x+44*dir`; anim 6 follow-through instead of anim 3 warp-out. | `sub=64`, `eff=0x44`, `param=0`; Dim `0x080FD621` -> Spawn `0x080D1D28` -> Actor `0x5A` `0x080D18F1`; Target PvE `0x080D1BEC` / PvP `0x080D1CA4`; Hitbox `0x080D1B28` (`r4=0x2B2CFE04`, `r7=3`, `z=0x20_0000`). Spr: `08-09` (anim 4->0->3->4->5->3), `14-09` (anim 0/1), `0c-0c` (anim 0). SFX: `0x92`, `0xA8`, `0x75`. |
| **ProtoMnV2** | `0x0E4` | `navis/part1.luau` | **P1** | Same as ProtoMan V1 (ROM tier `param=1` unused; only damage 180 scales). | `sub=65`, `eff=0x44`, `param=1`; same handler/spr/SFX as V1. |
| **ProtoMnV3** | `0x0E5` | `navis/part1.luau` | **P1** | Same as ProtoMan V1 (ROM tier `param=2` unused; only damage 200 scales). | `sub=66`, `eff=0x44`, `param=2`; same handler/spr/SFX as V1. |
| **ProtoMnV4** | `0x0E6` | `navis/part1.luau` | **P1** | Same as ProtoMan V1 (ROM tier `param=3` unused; only damage 220 scales). | `sub=67`, `eff=0x44`, `param=3`; same handler/spr/SFX as V1. |
| **ProtoMnV5** | `0x0E7` | `navis/part1.luau` | **P1** | Same as ProtoMan V1 (ROM tier `param=4` unused; only damage 240 scales; Giga class matches ROM). | `sub=68`, `eff=0x44`, `param=4`; same handler/spr/SFX as V1. |
| **BowlMan** | `0x11F` | `navis/part1.luau` | **P0** | Completely fabricated mechanic: sweeps a 1x3 `regions.wide` column hitbox across cols 1–6 with `BLAST_FX` + `CRACK_SOUND` (`0x7B`). ROM places **Bowling Pins** (`AttackObj 0x8A`, 50 dmg fixed) along BowlMan's own row ahead until hitting a hole/obstacle/actor (`req=0x10, forb=0x3E000000`), then rolls a **Bowling Ball** (`AttackObj 0x8B`, chip dmg, `self=0x1C` breaking+piercing, speed 7 px/f, stopped by holes) down that row; each pin hit by the ball launches a **Flying Pin** (`AttackObj 0x89`, 50 dmg, `self=0x18` breaking+piercing) in a parabolic arc homing onto a random enemy panel across all 3 rows! Also missing solid-panel check (`0x0800BF4A`) and wrong anim/SFX. | `sub=59`, `eff=0x40`, `param=0`; Dim `0x080FD221` -> Spawn `0x080CE000` -> Actor `0x52` `0x080CDDC1`; Pin loop `0x080CE01C` -> `BowlPin` `0x8A` `0x080E85F5` -> `FlyingPin` `0x89` `0x080E83BD`; `BowlBall` `0x8B` `0x080E8929`. Spr: `08-0E` (BowlMan anim 0->6->7; Pin anim 11/9/10; FlyingPin anim 12; Ball anim 13/14/15). SFX: `0x92`, `0x93`, `0x11D`, `0x135`, `0x136`. |
| **BowlManV2** | `0x120` | `navis/part1.luau` | **P0** | Same as BowlMan V1 (ROM tier `param=1` unused; only ball damage 120 scales; pin damage stays 50). | `sub=60`, `eff=0x40`, `param=1`; same handler/spr/SFX as V1. |
| **BowlManV3** | `0x121` | `navis/part1.luau` | **P0** | Same as BowlMan V1 (ROM tier `param=2` unused; only ball damage 140 scales; pin damage stays 50). | `sub=61`, `eff=0x40`, `param=2`; same handler/spr/SFX as V1. |
| **BowlManV4** | `0x122` | `navis/part1.luau` | **P0** | Same as BowlMan V1 (ROM tier `param=3` unused; only ball damage 160 scales; pin damage stays 50). | `sub=62`, `eff=0x40`, `param=3`; same handler/spr/SFX as V1. |
| **BowlManV5** | `0x123` | `navis/part1.luau` | **P0** | Same as BowlMan V1 (ROM tier `param=4` unused; only ball damage 180 scales; pin damage stays 50; Giga class matches ROM). | `sub=63`, `eff=0x40`, `param=4`; same handler/spr/SFX as V1. |
| **MistMan** | `0x11A` | `navis/part1.luau` | **P0** | Completely wrong mechanic & sprite: spawns `10-44` on MegaMan's panel and delivers 3 guaranteed `regions.plus` (5-panel cross) strikes on enemy. ROM summons **MistMan's Lamp (`08-0D`)** on MegaMan's panel (with solid-panel check `0x0800BF4A` + mist `SimpleEffect 0x42` `14-1B` anim 2), then spawns **MistMan Ghosts** (`AttackObj 0x85`) on valid solid unoccupied enemy-side panels immediately **in front (`enemy_x - dir`)** and **behind (`enemy_x + dir`)** every enemy (max 2 hits per target; 1 if against wall; 0 if boxed by holes/obstacles). Ghosts materialize sequentially (`10-49` cloud -> `10-44` MistMan anim 3->4->5) and strike the **single** adjacent enemy panel (`region=0x01`, `self=0x12` piercing, `hit_mod=3`). | `sub=54`, `eff=0x3D`, `param=0`; Dim `0x080FCEC9` -> Spawn `0x080CCA00` -> Actor `0x4E` `0x080CC7F1`; Ghost spawner `0x080CCA1C` -> `MistGhost` `0x85` `0x080E7ABD`; Hitbox `0x080E7CA6` (`r4=0x12130901`, `r7=3`). Spr: `08-0D` (lamp), `14-1B` anim 2, `10-49` (mist cloud), `10-44` (anim 3->4->5). SFX: `0x92`, `0xB6` (`arrive`), `0x7F`. |
| **MistManV2** | `0x11B` | `navis/part1.luau` | **P0** | Same as MistMan V1 (ROM tier `param=1` unused; only damage 110 scales). | `sub=55`, `eff=0x3D`, `param=1`; same handler/spr/SFX as V1. |
| **MistManV3** | `0x11C` | `navis/part1.luau` | **P0** | Same as MistMan V1 (ROM tier `param=2` unused; only damage 130 scales). | `sub=56`, `eff=0x3D`, `param=2`; same handler/spr/SFX as V1. |
| **MistManV4** | `0x11D` | `navis/part1.luau` | **P0** | Same as MistMan V1 (ROM tier `param=3` unused; only damage 150 scales). | `sub=57`, `eff=0x3D`, `param=3`; same handler/spr/SFX as V1. |
| **MistManV5** | `0x11E` | `navis/part1.luau` | **P0** | Same as MistMan V1 (ROM tier `param=4` unused; only damage 170 scales; Giga class matches ROM). | `sub=58`, `eff=0x3D`, `param=4`; same handler/spr/SFX as V1. |
| **Roll** | `0x0DB` | `navis/part1.luau` | **P1** | **Heal amount 2x too high**: heals **60 HP** instead of BN3 ROM-verified **30 HP** (`0x080B6F0C[0]=15` * 2 at `0x080D8252`); missing target check (`0x080B6F22`: must be strictly ahead of MegaMan with unoccupied front panel, else skips straight to heal — current falls back onto `tx, ty` and attacks enemies behind MegaMan); re-targets per hit instead of locking once; `AntiRecv` cancels Roll before attack (`rules/init.luau:164`); missing vertical flight arc (`z=0->160->0->160`), falling heart (`10-16`), `10-13` sparkles, `10-15` attack FX, flight SFX `0x10C`, 10f hit interval (uses 8f + `0x74`). | `sub=5`, `eff=0x07`, `param=0`; Dim `0x080F9131` -> Spawn `0x080B7068` -> Actor `0x04` `0x080B6A75`; Target `0x080B6F22`; Hitbox `0x080B7020` (`r4=0x12130801`, `r7=3`, 3 hits @ 10f); Heal `0x080B6EE0` -> `RollHeal` `0x16` `0x080D8179` (`0x080B6F0C[0]=15` -> `30 HP`). Spr: `10-14` (anim 0->6->3->4->5->6), `10-13`, `10-15` (anim 0/1), `10-16` (heart), `0c-06` (`heal`). SFX: `0x92`, `0x10C`, `0x72`. |
| **RollV2** | `0x0DC` | `navis/part1.luau` | **P1** | Same as Roll V1; **heal amount 2x too high**: heals **100 HP** instead of BN3 ROM-verified **50 HP** (`0x080B6F0C[1]=25` * 2 = `50 HP`). | `sub=6`, `eff=0x07`, `param=1`; `0x080B6F0C[1]=25` -> `50 HP`. Same handler/spr/SFX as V1. |
| **RollV3** | `0x0DD` | `navis/part1.luau` | **P1** | Same as Roll V1; **heal amount 1.75x too high**: heals **140 HP** instead of BN3 ROM-verified **80 HP** (`0x080B6F0C[2]=40` * 2 = `80 HP`). | `sub=7`, `eff=0x07`, `param=2`; `0x080B6F0C[2]=40` -> `80 HP`. Same handler/spr/SFX as V1. |
| **GutsMan** | `0x0DE` | `navis/part1.luau` | **P0** | Completely wrong mechanic: instant 1-frame `regions.other_area` hit + cracks only `opp_side` panels. ROM drops GutsMan from `z=140` (aborts if MegaMan's panel is a hole!), checks if front panel `(x+dir, y)` is solid (`flag 0x10`, else NO waves spawn!), then spawns **3 traveling `GutsWave` shockwaves** (`AttackObj 0x1B`, `10-00`) on rows `1, 2, 3` starting at col `x+dir` (step interval randomly chosen per row from `[10, 12, 14, 16]`f at `0x080B918C`). Each wave cracks every panel it crosses (including user's own panels ahead of GutsMan!), deals single-panel `0x18` (breaking+piercing) damage, and **stops on holes**! Missing sky drop, shockwave sprite `10-00`, anims `3->4->0->5->6->3`, SFX `0x7E` & `0x7A`. | `sub=8`, `eff=0x09`, `param=0`; Dim `0x080F92F9` -> Spawn `0x080B9190` -> Actor `0x0A` `0x080B8EA5`; Wave spawner `0x080B9148` -> `GutsWave` `0x1B` `0x080D8AFD` (panel crack `0x0800B568`, `self=0x18` `0x80000084`, `hit_mod=3`). Spr: `08-08` (anim 3->4->0->5->6->3), `10-00` (wave), `14-09` anim 2. SFX: `0x7E` (`crack-shot`), `0x7A`. |
| **GutsManV2** | `0x0DF` | `navis/part1.luau` | **P0** | Same as GutsMan V1 (ROM tier `param=1` unused; only damage 70 scales). | `sub=9`, `eff=0x09`, `param=1`; same handler/spr/SFX as V1. |
| **GutsManV3** | `0x0E0` | `navis/part1.luau` | **P0** | Same as GutsMan V1 (ROM tier `param=2` unused; only damage 90 scales). | `sub=10`, `eff=0x09`, `param=2`; same handler/spr/SFX as V1. |
| **GutsManV4** | `0x0E1` | `navis/part1.luau` | **P0** | Same as GutsMan V1 (ROM tier `param=3` unused; only damage 100 scales). | `sub=11`, `eff=0x09`, `param=3`; same handler/spr/SFX as V1. |
| **GutsManV5** | `0x0E2` | `navis/part1.luau` | **P0** | Same as GutsMan V1 (ROM tier `param=4` unused; only damage 120 scales; Giga class matches ROM). | `sub=12`, `eff=0x09`, `param=4`; same handler/spr/SFX as V1. |
| **BeastMan** | `0x0ED` | `navis/part1.luau` | **P0** | Completely wrong mechanic & targeting: warps in front of enemy front col, homes hit 3 onto enemy, and strikes a massive 2x3 `regions.wide_long` (6 panels) on all 3 hits with `collision.slash`. ROM keeps BeastMan on MegaMan's panel (checks solid `0x0800BF4A`), winds up (`anim 0->3->6`), disappears, and spawns **3 `BeastClaw` flying sweeps** (`AttackObj 0x21`, 10f apart) intersecting at **fixed focal panel `(user_x + 3*dir, user_y)`** (NO enemy homing!): Claw 1 diagonal-down (`08-01` anim 9, `vx=8.575*dir, vy=+5.145`, SFX `0xBD`), Claw 2 diagonal-up (`08-01` anim 9 v-flip, `vx=8.575*dir, vy=-5.145`, SFX `0xBD`), Claw 3 horizontal straight (`08-01` anim 8, `vx=10*dir, vy=0`, SFX `0x94`). Each claw deals single-panel **non-sword piercing** (`self=0x12` `0x80000004`, NOT sword!) damage on panels it crosses (3 hits at focal panel, 1 hit on diagonal/row wings). | `sub=13`, `eff=0x0A`, `param=0`; Dim `0x080F9391` -> Spawn `0x080BB0AA` -> Actor `0x10` `0x080BADBD`; `BeastClaw` `0x21` `0x080D9625` (table `0x080D96FC`, hitbox `0x080D9900`, `r4=0x12130901`, `r7=3`, afterimage `0x080D9924`). Spr: `08-01` (anim 0->3->6; claws anim 9/8), `14-09` (anim 0/1). SFX: `0x92`, `0xBD`, `0x94`. |
| **BeastMnV2** | `0x0EE` | `navis/part1.luau` | **P0** | Same as BeastMan V1 (ROM velocity table `0x080D96FC` identical across tiers 0–4; only damage 50 scales). | `sub=14`, `eff=0x0A`, `param=1`; same handler/spr/SFX as V1. |
| **BeastMnV3** | `0x0EF` | `navis/part1.luau` | **P0** | Same as BeastMan V1 (only damage 60 scales). | `sub=15`, `eff=0x0A`, `param=2`; same handler/spr/SFX as V1. |
| **BeastMnV4** | `0x0F0` | `navis/part1.luau` | **P0** | Same as BeastMan V1 (only damage 70 scales). | `sub=16`, `eff=0x0A`, `param=3`; same handler/spr/SFX as V1. |
| **BeastMnV5** | `0x0F1` | `navis/part1.luau` | **P0** | Same as BeastMan V1 (only damage 80 scales; Giga class matches ROM). | `sub=17`, `eff=0x0A`, `param=4`; same handler/spr/SFX as V1. |
| **BubblMan** | `0x0F2` | `navis/part1.luau` | **P0** | Completely wrong mechanic + broken code: (1) Global-homing instant single-panel hits across any row instead of straight-line horizontal projectiles (`BubblShot`, `AttackObj 0x28`, `9 px/f`) fired down **MegaMan's own row** (`x+dir, y`) that stop on first hit/obstacle; (2) Wrong shot count & fabricated low-HP scaling (`4 + 0/1/3/5` = 4–9 shots) vs ROM-verified **fixed 6 shots** (`0x080BC3EE: tier + 6`, ZERO HP scaling in BN3!); (3) `part1.luau:508` calls `regions.random_panel` with field-condition region `regions.other_area`, which always returns `nil, nil` (`r.panels == nil`) so every shot homes on `enemy_target`; (4) Non-piercing `collision.attack` vs ROM `0x12` piercing; (5) Wrong anims (`0->3` vs `5->6->5` + bubble emerge/submerge `08-02` anims 7/8/9 + droplets `10-1E`), wrong projectile (`10-0C` vs `08-02` anim 22), wrong SFX (`0xAC/0xAD` vs `0xAB/0xAD` + `0x7D` per shot), wrong fire rate (4f vs 8f). | `sub=18`, `eff=0x0E`, `param=0`; Dim `0x080F96B5` -> Spawn `0x080BC58E` -> Actor `0x14` `0x080BC241`; Shot count `0x080BC3EE` (`0+6=6` shots); `BubblShot` `0x28` `0x080DACBD` (`vx=9*dir`, `elem=3` Aqua, `self=0x12` piercing, `hit_mod=3`). Spr: `08-02` (anim 5/6; emerge/submerge 7/8/9; shot 22), `10-1E` droplets. SFX: `0xAB`, `0xAD`, `0x7D` (`cannon`). |
| **BubblMnV2** | `0x0F3` | `navis/part1.luau` | **P0** | Same as BubblMan V1; ROM fires fixed **7 shots** (`1 + 6`), whereas `part1.luau` fires `5 + 0/1/3/5` (5–10) with fabricated HP scaling. | `sub=19`, `eff=0x0E`, `param=1`; `0x080BC3EE` -> `1+6=7` shots. Same handler/spr/SFX as V1. |
| **BubblMnV3** | `0x0F4` | `navis/part1.luau` | **P0** | Same as BubblMan V1; ROM fires fixed **8 shots** (`2 + 6`), whereas `part1.luau` fires `6 + 0/1/3/5` (6–11) with fabricated HP scaling. | `sub=20`, `eff=0x0E`, `param=2`; `0x080BC3EE` -> `2+6=8` shots. Same handler/spr/SFX as V1. |
| **BubblMnV4** | `0x0F5` | `navis/part1.luau` | **P0** | Same as BubblMan V1; ROM fires fixed **9 shots** (`3 + 6`), whereas `part1.luau` fires `7 + 0/1/3/5` (7–12) with fabricated HP scaling. | `sub=21`, `eff=0x0E`, `param=3`; `0x080BC3EE` -> `3+6=9` shots. Same handler/spr/SFX as V1. |
| **BubblMnV5** | `0x0F6` | `navis/part1.luau` | **P0** | Same as BubblMan V1; ROM fires fixed **10 shots** (`4 + 6`), whereas `part1.luau` fires `8 + 0/1/3/5` (8–13) with fabricated HP scaling. (Giga class matches ROM.) | `sub=22`, `eff=0x0E`, `param=4`; `0x080BC3EE` -> `4+6=10` shots. Same handler/spr/SFX as V1. |

---

#### Detailed Verification Notes & ROM Evidence

##### 1. ROM Tables Referenced
- **Family `0x14` (Navi) Dispatcher**: `0x080474A8` (`PlayerActionJumpTable[20]` @ `0x080B0DF4` = `0x080474A9`). Reads `NaviDimParamTable` (`0x080473E4[sub*2]`) -> `(eff_id, param)` -> `SpawnDimmingChipEffect` (`0x08008E54`).
- **Actor Object Jump Table**: `0x0800369C` (indexed by `obj_id * 4`; `0x080036A8` is entry 3 = FlashMan).
- **Attack Object Jump Table**: `0x080038C4` (indexed by `atk_id * 4`).
- **SimpleEffect Table**: `0x080EDB88` (`EffectObj[0]` @ `0x08003C18` = `0x080EDCD9`; 4 bytes/entry: `cc, ii, anim, flags`).
- **Collision Flags Table**: `0x080AF51C` (`0x080AF074`, 8 bytes/entry: `side0, side1`):
  - `0x12`: `0x80000004` / `0x40000004` (piercing)
  - `0x18`: `0x80000084` / `0x40000084` (breaking + piercing)
  - `0x1A`: `0x80000094` / `0x40000094` (piercing + breaking + guard-pierce)
  - `0x1C`: `0x80000014` / `0x40000014` (breaking + piercing)
  - `0x1F`: `0x80000015` / `0x40000015` (piercing + breaking + guard-pierce)
  - `0x2B`: `0x8000000C` / `0x4000000C` (sword + piercing)
- **Field-Condition Regions Table**: `0x080AF4D4` (`0x80..0x89`):
  - `0x84`: `req=0x00800000, forb=0x00000000` (side-0 actor-occupied panels — FlashMan used by side 1)
  - `0x85`: `req=0x00400000, forb=0x00000000` (side-1 actor-occupied panels — FlashMan used by side 0)
  - `0x87`: `req=0x00000010, forb=0x00000020` (all solid side-0 area panels — PlantMan used by side 1)
  - `0x88`: `req=0x00000030, forb=0x00000000` (all solid side-1 area panels — PlantMan used by side 0)
- **Status Duration Tables (`0x080AD476` -> `0x080AD4F2` / `0x080AD560`)**:
  - `0x10 / 0x11 / 0x12` (Paralyze, flag `0x0400`) @ `0x08019DC0`: **`90 / 140 / 190`** frames (used by FlashMan V1 / V2 / V3–V5).
  - `0x30 / 0x31 / 0x32` (Vine Bind, flag `0x0800` + `0x080ADAC2` vine wrap) @ `0x08019DCE`: **`120 / 180 / 240`** frames (used by PlantMan V1 / V2 / V3–V5).

##### 2. Notable Surprising ROM Truths (Why Disassembly Was Essential)
- **Roll V1–V3 Heal Amounts Are `30 / 50 / 80` HP (NOT `60 / 100 / 140`!)**:
  - Table `0x080B6F0C` (identical across `bn3.gba` `0x080B6F0C`, `bn3b.gba` `0x080B6FFC`, `exe3.gba` `0x080B781C`, `exe3b.gba` `0x080B791C`): `[15, 25, 40, 0, 0, 150]`.
  - `RollHeal` (`0x080D8250..0x080D825C`) loads `[r5, #0x2C]`, doubles it once (`add r0, r0, r0`), and calls `0x080ADA0C` (`hp = min(max_hp, hp + r0)`), matching the same `hp/2` u8 encoding used by `Recov10..Recov300` (and yielding `300 HP` for BigHeart `param=5`!). `60/100/140` in `part1.luau` and `en.toml` were mistakenly borrowed from later EXE titles.
- **BubblMan V1–V5 Has NO Low-HP Scaling in EXE3**:
  - Verified across `Family 0x14` (`0x080474A8`), `BubblMan_Dim` (`0x080F96B4`), `BubblMan` Actor `0x14` (`0x080BC240..0x080BC5A8`), and `BubblShot` AttackObj `0x28` (`0x080DACBC..0x080DADF6`) — and confirmed in all 4 EXE3 ROMs (`bn3` `0x080BC3EE`, `bn3b` `0x080BC4DE`, `exe3` `0x080BCCFE`, `exe3b` `0x080BCDFE`): shot count is strictly `ldrb r0, [r5, #0x4]; add r0, #6` = **`6 / 7 / 8 / 9 / 10`** horizontal straight-line shots down MegaMan's row.
  - Furthermore, `part1.luau:508` contains a silent bug: `regions.random_panel(me.panel_x, me.panel_y, regions.other_area[me.alliance], ...)` always returns `nil, nil` because `regions.random_panel` iterates `r.panels`, whereas `regions.other_area` is a `r.field` region.
- **`helpers.luau` / `assets.toml` Asset Naming Notes**:
  - `helpers.BUBBLMAN_SPRITE = asset.sprite("bubble-wrap")` resolves to `08-02` (which IS BubblMan's sprite, just misnamed `"bubble-wrap"` in `assets.toml`).
  - `helpers.MISTMAN_SPRITE = asset.sprite("mistman")` resolves to `10-44` (MistMan ghost); `assets.toml` already defines `mistman-lamp = "08-0d"`, which is the actual Navi actor sprite spawned on MegaMan's panel (`0x080CC81C`).
  - Missing from `assets.toml` for exact ROM parity: `10-00` (GutsMan shockwave), `10-13` (Roll sparkle), `10-15` (Roll attack), `10-16` (Roll heart), `10-1e` (BubblMan droplets), `10-49` (MistMan mist cloud), and SFX `0x7A`, `0x7F`, `0xA8`, `0xAB`, `0xBD`, `0xFF`, `0x10C`, `0x11D`, `0x135`, `0x136`.


---

### 5.7 Group 7：领航员芯片 Part 2（36 张：DesrtMan / FlamMan / DrillMan / MetalMan / Punk / KingMan / DarkMan / JapanMan）

**ROM Audited**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US, `A6BE`)  
**Modules Audited**:
- `content/exe3/chips/navis/helpers.luau`
- `content/exe3/chips/navis/part2.luau` (36 chips across 8 Navi families)
- Cross-checked against `content/exe3/compat/chips.toml` and `content/exe3/compat/assets.toml`

> **Note on file roster**: `part2.luau` actually defines **DesrtMan V1–V5, FlamMan V1–V5, DrillMan V1–V5, MetalMan V1–V5, Punk, KingMan V1–V5, DarkMan V1–V5, and JapanMan V1–V5** (36 chips total). MistMan V1–V5 and BowlMan V1–V5 are defined in `content/exe3/chips/navis/part1.luau` (Batch 6).

---

#### Executive Summary

| Severity | Count | Chips |
| :--- | ---: | :--- |
| **P0** | **36** | All 36 chips in `part2.luau` (DesrtMan V1–V5, FlamMan V1–V5, DrillMan V1–V5, MetalMan V1–V5, Punk, KingMan V1–V5, DarkMan V1–V5, JapanMan V1–V5) |
| **P1** | **0** | *(P1 issues also exist on every chip, rolled into P0)* |
| **P2** | **0** | *(P2 presentation/sprite/SFX mismatches also exist on every chip, rolled into P0)* |
| **OK** | **0** | None (though static chip table metadata in `part2.luau` and `compat/chips.toml` matches ROM 100% for all 36 chips) |

##### Key Systemic Findings (`helpers.luau`, `compat/assets.toml`, `compat/chips.toml`)
1. **Static Chip Metadata (`0x08011510`)**: All 36 chips' `codes`, `element`, `power`, `mb`, `rarity`, and `class` (`mega` for V1–V4/Punk, `giga` for V5) in both `part2.luau` and `compat/chips.toml` match A6BE `ChipDataTable` (`0x08011510`) 100%.
2. **Pool 1 Dispatch Anchor Correction**: In A6BE `bn3.gba`, the object update loop (`0x08002F10`, table pointers at `0x08002F8C`) dispatches Pool 1 (`SpawnPool1Obj` `0x0800304C`) via **`0x0800369C`** (not `0x080036A8`), Pool 3 (`SpawnAttackObj` `0x08003084`) via **`0x080038C4`**, and Pool 4 (`SpawnEffectObj` `0x080030D8`) via **`0x08003C18`**.
3. **Missing Hole-Panel Abort (`IsPanelSolid` `0x0800BF4A`)**: 6 of the 8 Navis in `part2.luau` (**DesrtMan, FlamMan, Punk, KingMan, DarkMan, JapanMan**) check `IsPanelSolid` on the user's panel during appear/landing and immediately abort without attacking if the user is hovering over a hole (AirShoes/FloatShoes). `helpers.luau` has no such check.
4. **No Dimming Timer Underflow**: All 8 controllers in `part2.luau` increment `me.timer += 1` from `0` and terminate via `h.end_navi(me)`, so none suffer from unsigned countdown softlocks.
5. **AntiNavi (`0x08007950` / `0x08008EA0`)**: Every Navi dimming handler in ROM (`0x080FA974`, `0x080FB550`, `0x080FB4B8`, `0x080F9D38`, `0x080FC638`, `0x080FC22C`, `0x080FD6B8`, `0x080FD9B0`) runs the standard 30-frame `IsAntiNaviPending` / `CheckAntiNaviTrigger` gate before spawning the Navi actor.
6. **`helpers.begin_navi` Presentation Over-Generalization**:
   - Hardcodes `s.shadow = "hidden"`, whereas 7/8 Navis in `part2.luau` call `0x08002DB8` (shadow ON); only MetalMan calls `0x08002C80` (shadow hidden, provided by its gear platform overlay `08-0a` anim 9).
   - Hardcodes `battle.play_sound(APPEAR_SOUND)` (`0x92`), whereas DesrtMan plays `0x11B` (sand emerge), DrillMan plays no appear SFX, and KingMan drops from `z=140` playing `0x7E` on landing.
   - `KINGMAN_KIND` and `DARKMAN_KIND` pass `side_only = true` to `dimming_chips.spawn`; harmless while `me.facing` is unused, but dangerous if refactored to use `me.facing`.
7. **Missing Assets in `content/exe3/compat/assets.toml`**:
   - Missing ROM sub-object sprites: `10-22` (FlamMan fire pillar), `10-35` (DrillMan falling boulder), `10-3e` (KingMan CheckMate shockwave), `10-40` (Punk 400-dmg giant roller), `10-42` (DarkMan DarkHole), `10-43` (Punk 100-dmg dual roller).
   - Missing ROM SFX IDs (`gSongTable` `0x08148B6C`): `0x7F`, `0x8F`, `0x95`, `0xB2`, `0xB4`, `0xBD`, `0xF3`, `0x101`, `0x11B`, `0x12C`, `0x131`, `0x132`, `0x133`, `0x137`.

---

#### Complete 36-Chip Audit Table

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **DesrtMan** | `0x0F7` | `part2.luau` | **P0** | Fires 3 instant `regions.line` breaking hitboxes on rows 1–3 from `enemy_front_col`. ROM spawns **2 sand-fist projectiles** on `(x+dir, y±1)` (only if solid), converts every traversed solid panel to **Sand** (`type=10`), supports **Down/Up+Fwd+A diagonal aiming**, uses **piercing** (`0x12`, non-breaking), and aborts on user hole. | Dim `0x080FA974` (sub 28, eff `0x1F`, p=0) -> Actor `0x28` `0x080C0E60` -> Atk `0x4B` `0x080DF428`. Spr: `08-03` (Navi anim 3/0-2/1, Fists anim 9/10->6/7, shadow ON). SFX: `0x11B`, `0xBD`, `0xC8`. |
| **DesrtMnV2** | `0x0F8` | `part2.luau` | **P0** | Same P0 diffs as DesrtMan V1 (ROM tier param unused; differs only in power `140`, MB `52`, rarity `3`). | Dim `0x080FA974` (sub 29, eff `0x1F`, p=1) -> Actor `0x28` `0x080C0E60` -> Atk `0x4B` `0x080DF428`. Spr: `08-03`. SFX: `0x11B`, `0xBD`, `0xC8`. |
| **DesrtMnV3** | `0x0F9` | `part2.luau` | **P0** | Same P0 diffs as DesrtMan V1 (power `160`, MB `64`, rarity `4`). | Dim `0x080FA974` (sub 30, eff `0x1F`, p=2) -> Actor `0x28` `0x080C0E60` -> Atk `0x4B` `0x080DF428`. Spr: `08-03`. SFX: `0x11B`, `0xBD`, `0xC8`. |
| **DesrtMnV4** | `0x0FA` | `part2.luau` | **P0** | Same P0 diffs as DesrtMan V1 (power `180`, MB `72`, rarity `4`). | Dim `0x080FA974` (sub 31, eff `0x1F`, p=3) -> Actor `0x28` `0x080C0E60` -> Atk `0x4B` `0x080DF428`. Spr: `08-03`. SFX: `0x11B`, `0xBD`, `0xC8`. |
| **DesrtMnV5** | `0x0FB` | `part2.luau` | **P0** | Same P0 diffs as DesrtMan V1 (power `210`, MB `80`, rarity `4`, Giga). | Dim `0x080FA974` (sub 32, eff `0x1F`, p=4) -> Actor `0x28` `0x080C0E60` -> Atk `0x4B` `0x080DF428`. Spr: `08-03`. SFX: `0x11B`, `0xBD`, `0xC8`. |
| **FlamMan** | `0x101` | `part2.luau` | **P0** | Fires 3 full-width `regions.row` (cols 1–6, even behind user & over holes!) hitboxes. ROM spawns a **Up/Down-steerable 1-panel fire pillar trail** starting at `(x+dir, y)` advancing every 20f (stops at holes), PLUS **B+D-pad secret command** expanding to 3/5-panel flame waves, and aborts on user hole. | Dim `0x080FB550` (sub 43, eff `0x2A`, p=0) -> Actor `0x3B` `0x080C6038` -> Atk `0x6A` `0x080E344C`, Eff `0x1D`/`0x1E` `0x080EFE00`/`0x080EFF20`. Spr: `08-05` (anim 0/3/4/6/5 + 7,21, shadow ON) & `10-22` (anim 0/1/2). SFX: `0x92`, `0x80`. |
| **FlamManV2** | `0x102` | `part2.luau` | **P0** | Same P0 diffs as FlamMan V1 (ROM tier param unused; power `150`, MB `52`, rarity `3`). | Dim `0x080FB550` (sub 44, eff `0x2A`, p=1) -> Actor `0x3B` `0x080C6038` -> Atk `0x6A` `0x080E344C`. Spr: `08-05`, `10-22`. SFX: `0x92`, `0x80`. |
| **FlamManV3** | `0x103` | `part2.luau` | **P0** | Same P0 diffs as FlamMan V1 (power `180`, MB `62`, rarity `4`). | Dim `0x080FB550` (sub 45, eff `0x2A`, p=2) -> Actor `0x3B` `0x080C6038` -> Atk `0x6A` `0x080E344C`. Spr: `08-05`, `10-22`. SFX: `0x92`, `0x80`. |
| **FlamManV4** | `0x104` | `part2.luau` | **P0** | Same P0 diffs as FlamMan V1 (power `210`, MB `72`, rarity `4`). | Dim `0x080FB550` (sub 46, eff `0x2A`, p=3) -> Actor `0x3B` `0x080C6038` -> Atk `0x6A` `0x080E344C`. Spr: `08-05`, `10-22`. SFX: `0x92`, `0x80`. |
| **FlamManV5** | `0x105` | `part2.luau` | **P0** | Same P0 diffs as FlamMan V1 (power `240`, MB `82`, rarity `4`, Giga). | Dim `0x080FB550` (sub 47, eff `0x2A`, p=4) -> Actor `0x3B` `0x080C6038` -> Atk `0x6A` `0x080E344C`. Spr: `08-05`, `10-22`. SFX: `0x92`, `0x80`. |
| **DrillMan** | `0x106` | `part2.luau` | **P0** | Stands still firing 6 `regions.wide` sweeps across cols 1–6 (1 hit on stationary target, no panel break, `hit_mod=1` on steps 1–4). ROM has **3 distinct phases**: (1) **3 staggered horizontal drills** from back col (`vx=8px/f`, `hit_mod=3`), (2) **3 warning-flashed vertical drills** on random enemy panels that **break/crack panels**, (3) **3 falling boulders** (`10-35`) on random solid enemy panels. | Dim `0x080FB4B8` (sub 38, eff `0x29`, p=0) -> Actor `0x37` `0x080C4F00` -> Eff `0x1B` `0x080EFBA0`, Atk `0x64` `0x080E26EC`, Atk `0x65` `0x080E293C`, Atk `0x5F` `0x080E1E78`. Spr: `08-06` (anim 5/4/6/10-15, shadow ON), `10-35`. SFX: `0x12C`, `0xF3`, `0x95`, `0x101`. |
| **DrillMnV2** | `0x107` | `part2.luau` | **P0** | Same P0 diffs as DrillMan V1 (ROM tier param unused; power `90`, MB `70`, rarity `3`). | Dim `0x080FB4B8` (sub 39, eff `0x29`, p=1) -> Actor `0x37` `0x080C4F00`. Spr: `08-06`, `10-35`. SFX: `0x12C`, `0xF3`, `0x95`, `0x101`. |
| **DrillMnV3** | `0x108` | `part2.luau` | **P0** | Same P0 diffs as DrillMan V1 (power `110`, MB `74`, rarity `4`). | Dim `0x080FB4B8` (sub 40, eff `0x29`, p=2) -> Actor `0x37` `0x080C4F00`. Spr: `08-06`, `10-35`. SFX: `0x12C`, `0xF3`, `0x95`, `0x101`. |
| **DrillMnV4** | `0x109` | `part2.luau` | **P0** | Same P0 diffs as DrillMan V1 (power `130`, MB `78`, rarity `4`). | Dim `0x080FB4B8` (sub 41, eff `0x29`, p=3) -> Actor `0x37` `0x080C4F00`. Spr: `08-06`, `10-35`. SFX: `0x12C`, `0xF3`, `0x95`, `0x101`. |
| **DrillMnV5** | `0x10A` | `part2.luau` | **P0** | Same P0 diffs as DrillMan V1 (power `150`, MB `82`, rarity `4`, Giga). | Dim `0x080FB4B8` (sub 42, eff `0x29`, p=4) -> Actor `0x37` `0x080C4F00`. Spr: `08-06`, `10-35`. SFX: `0x12C`, `0xF3`, `0x95`, `0x101`. |
| **MetalMan** | `0x10B` | `part2.luau` | **P0** | Never moves; smashes `user.x + 2*facing` (fallback `enemy_target`) with non-piercing `breaking` & `hit_mod=5`. ROM has **30f interactive A+D-pad aiming** across full 6×3 field -> **teleports** to aimed panel -> smashes **1 tile ahead** (`tx+dir, ty`) with **piercing+breaking** (`0x1A`), `hit_mod=3`, panel break (`0x0800B64C`), plus 0-dmg enemy-area shockwave (`0x1819FE82/81`) & gear base (`08-0a` anim 9). | Dim `0x080F9D38` (sub 23, eff `0x14`, p=0) -> Actor `0x1C` `0x080BE2C0`, Eff `0x11` `0x080EEFDC`. Spr: `08-0a` (anim 0/3/4, gear anim 9). SFX: `0x92`, `0xB4`, `0x7E`. |
| **MetalMnV2** | `0x10C` | `part2.luau` | **P0** | Same P0 diffs as MetalMan V1 (ROM tier param unused; power `130`, MB `60`, rarity `3`). | Dim `0x080F9D38` (sub 24, eff `0x14`, p=1) -> Actor `0x1C` `0x080BE2C0`. Spr: `08-0a`. SFX: `0x92`, `0xB4`, `0x7E`. |
| **MetalMnV3** | `0x10D` | `part2.luau` | **P0** | Same P0 diffs as MetalMan V1 (power `160`, MB `70`, rarity `4`). | Dim `0x080F9D38` (sub 25, eff `0x14`, p=2) -> Actor `0x1C` `0x080BE2C0`. Spr: `08-0a`. SFX: `0x92`, `0xB4`, `0x7E`. |
| **MetalMnV4** | `0x10E` | `part2.luau` | **P0** | Same P0 diffs as MetalMan V1 (power `190`, MB `80`, rarity `4`). | Dim `0x080F9D38` (sub 26, eff `0x14`, p=3) -> Actor `0x1C` `0x080BE2C0`. Spr: `08-0a`. SFX: `0x92`, `0xB4`, `0x7E`. |
| **MetalMnV5** | `0x10F` | `part2.luau` | **P0** | Same P0 diffs as MetalMan V1 (power `250`, MB `90`, rarity `4`, Giga). | Dim `0x080F9D38` (sub 27, eff `0x14`, p=4) -> Actor `0x1C` `0x080BE2C0`. Spr: `08-0a`. SFX: `0x92`, `0xB4`, `0x7E`. |
| **Punk** | `0x110` | `part2.luau` | **P0** | Rolls straight across cols 1–6 on `enemy_target`'s row (`breaking`, `hit_mod=5`). ROM has **5 distinct moves** (`bonus = dmg - 180`): (1) default **perimeter loop** (`180+b`), (2) **≤25% HP / PvP sine-wave roll** (`360+b`), (3) **B+Left giant ball** (`400+b`, `10-40`, status `0x20`), (4) **2×200+b wide slashes**, (5) **D,F,D,F dual rollers** (`100+b` each, `10-43`); uses `0x1C` piercing+break, `hit_mod=3`, aborts on holes. | Dim `0x080FC638` (sub 53, eff `0x36`, p=0) -> Actor `0x45` `0x080C9B78` -> Atk `0x74` `0x080E557C`, Atk `0x77` `0x080E5DF4`. Spr: `08-0b` (anim 0/3/4/5/6/7, shadow ON), `10-40`, `10-43`. SFX: `0x92`, `0x133`, `0xBD`, `0x7F`. |
| **KingMan** | `0x115` | `part2.luau` | **P0** | Never moves; drops `regions.plus` `breaking` hitbox directly onto `enemy_target` (ignoring holes). ROM **drops from `z=140`** (aborts on user hole), **leaps fixed 3 panels ahead** `(x+3*dir, y)`, **FIZZLES** if landing panel is OOB/hole/occupied (`forbid=0x3E000000`), else spawns center `0x1A` hitbox + **4 solid-only CheckMate waves** (`10-3e`, `0x12` piercing, non-breaking). | Dim `0x080FC22C` (sub 48, eff `0x32`, p=0) -> Actor `0x41` `0x080C8304` -> Atk `0x70` `0x080E4C20`. Spr: `08-0c` (anim 0/3, shadow ON), `10-3e` (anim 0). SFX: `0x7E`, `0xB2`. |
| **KingManV2** | `0x116` | `part2.luau` | **P0** | Same P0 diffs as KingMan V1 (ROM tier param unused; power `170`, MB `48`, rarity `3`). | Dim `0x080FC22C` (sub 49, eff `0x32`, p=1) -> Actor `0x41` `0x080C8304` -> Atk `0x70` `0x080E4C20`. Spr: `08-0c`, `10-3e`. SFX: `0x7E`, `0xB2`. |
| **KingManV3** | `0x117` | `part2.luau` | **P0** | Same P0 diffs as KingMan V1 (power `200`, MB `60`, rarity `4`). | Dim `0x080FC22C` (sub 50, eff `0x32`, p=2) -> Actor `0x41` `0x080C8304` -> Atk `0x70` `0x080E4C20`. Spr: `08-0c`, `10-3e`. SFX: `0x7E`, `0xB2`. |
| **KingManV4** | `0x118` | `part2.luau` | **P0** | Same P0 diffs as KingMan V1 (power `240`, MB `72`, rarity `4`). | Dim `0x080FC22C` (sub 51, eff `0x32`, p=3) -> Actor `0x41` `0x080C8304` -> Atk `0x70` `0x080E4C20`. Spr: `08-0c`, `10-3e`. SFX: `0x7E`, `0xB2`. |
| **KingManV5** | `0x119` | `part2.luau` | **P0** | Same P0 diffs as KingMan V1 (power `280`, MB `84`, rarity `4`, Giga, `b19=0x92`). | Dim `0x080FC22C` (sub 52, eff `0x32`, p=4) -> Actor `0x41` `0x080C8304` -> Atk `0x70` `0x080E4C20`. Spr: `08-0c`, `10-3e`. SFX: `0x7E`, `0xB2`. |
| **DarkMan** | `0x124` | `part2.luau` | **P0** | Fires **3** homing hits with fabricated `blind`/`confuse`/`paralyze` on last hit; no DarkHole. ROM opens portals on enemy cols (`08-0f` anim 3), spawns **16** destructible (1 HP) downward-flying bats (`08-0f` anim 6, every 12f, `hit_mod=3`, **NO status effects!**), and summons a **DarkHole** (`10-42`, enables Bass/Serenade) on `(x+dir, y)`. | Dim `0x080FD6B8` (sub 69, eff `0x45`, p=0) -> Actor `0x5B` `0x080D1D44` -> Atk `0x99` `0x080EA5B8` -> Atk `0x9A` `0x080EA75C` & Atk `0x6C` `0x080E37B0`. Spr: `08-0f` (anim 0/3/6, shadow ON), `10-42`. SFX: `0x92`, `0x93`, `0x137`, `0x8F`. |
| **DarkManV2** | `0x125` | `part2.luau` | **P0** | Same P0 diffs as DarkMan V1; spawns **4** hits vs ROM's **18** bats (`0x080D1EB4[1]=0x12`). | Dim `0x080FD6B8` (sub 70, eff `0x45`, p=1) -> Actor `0x5B` `0x080D1D44` (18 bats). Spr: `08-0f`, `10-42`. SFX: `0x92`, `0x93`, `0x137`, `0x8F`. |
| **DarkManV3** | `0x126` | `part2.luau` | **P0** | Same P0 diffs as DarkMan V1; spawns **5** hits vs ROM's **20** bats (`0x080D1EB4[2]=0x14`). | Dim `0x080FD6B8` (sub 71, eff `0x45`, p=2) -> Actor `0x5B` `0x080D1D44` (20 bats). Spr: `08-0f`, `10-42`. SFX: `0x92`, `0x93`, `0x137`, `0x8F`. |
| **DarkManV4** | `0x127` | `part2.luau` | **P0** | Same P0 diffs as DarkMan V1; spawns **6** hits vs ROM's **22** bats (`0x080D1EB4[3]=0x16`). | Dim `0x080FD6B8` (sub 72, eff `0x45`, p=3) -> Actor `0x5B` `0x080D1D44` (22 bats). Spr: `08-0f`, `10-42`. SFX: `0x92`, `0x93`, `0x137`, `0x8F`. |
| **DarkManV5** | `0x128` | `part2.luau` | **P0** | Same P0 diffs as DarkMan V1; spawns **7** hits vs ROM's **24** bats (`0x080D1EB4[4]=0x18`, Giga). | Dim `0x080FD6B8` (sub 73, eff `0x45`, p=4) -> Actor `0x5B` `0x080D1D44` (24 bats). Spr: `08-0f`, `10-42`. SFX: `0x92`, `0x93`, `0x137`, `0x8F`. |
| **JapanMan** | `0x129` | `part2.luau` | **P0** | Teleports in front of enemy for fixed **4** `regions.long` `slash` hits. ROM stays on **user's panel**, uses **alternating A/B mashing** for **3–6 flurry thrusts** (`regions.wide` at `x+2*dir`, `hit_mod=1`) + **3-tile forward finisher** (`x+1..3*dir`, `hit_mod=3`), uses `0x2B` (piercing+thrown, NOT `slash`/AntiSwrd), aborts on user hole. | Dim `0x080FD9B0` (sub 78, eff `0x4A`, p=0) -> Actor `0x61` `0x080D391C`, Eff `0x0B` `0x080EE9E4`. Spr: `08-10` (anim 0/6/5 + spear overlay `7+anim`, shadow ON). SFX: `0x92`, `0x132`, `0x131`. |
| **JapanMnV2** | `0x12A` | `part2.luau` | **P0** | Same P0 diffs as JapanMan V1; wrongly scales thrust count to **5** (ROM uses same 4–7 hit A/B mash across all tiers; only power `45` scales). | Dim `0x080FD9B0` (sub 79, eff `0x4A`, p=1) -> Actor `0x61` `0x080D391C`. Spr: `08-10`. SFX: `0x92`, `0x132`, `0x131`. |
| **JapanMnV3** | `0x12B` | `part2.luau` | **P0** | Same P0 diffs as JapanMan V1; wrongly scales thrust count to **6** (power `50`, MB `88`, rarity `4`). | Dim `0x080FD9B0` (sub 80, eff `0x4A`, p=2) -> Actor `0x61` `0x080D391C`. Spr: `08-10`. SFX: `0x92`, `0x132`, `0x131`. |
| **JapanMnV4** | `0x12C` | `part2.luau` | **P0** | Same P0 diffs as JapanMan V1; wrongly scales thrust count to **7** (power `55`, MB `91`, rarity `4`). | Dim `0x080FD9B0` (sub 81, eff `0x4A`, p=3) -> Actor `0x61` `0x080D391C`. Spr: `08-10`. SFX: `0x92`, `0x132`, `0x131`. |
| **JapanMnV5** | `0x12D` | `part2.luau` | **P0** | Same P0 diffs as JapanMan V1; wrongly scales thrust count to **8** (power `60`, MB `94`, rarity `4`, Giga). | Dim `0x080FD9B0` (sub 82, eff `0x4A`, p=4) -> Actor `0x61` `0x080D391C`. Spr: `08-10`. SFX: `0x92`, `0x132`, `0x131`. |

---

#### Detailed Per-Navi ROM Verification Notes

##### Reference: ROM Collision Rows (`0x080AF51C`) Used in Batch 7
Decoded directly from `0x080AF074` (`table[row*2 + side]`):
- `0x12` (self) / `0x13` (target): `0x80000004` / `0x1A000006` — **Piercing** (non-breaking), hits navis/enemies/obstacles
- `0x18` (self) / `0x19` (target): `0x80000084` / `0x1A000086` — Piercing + ground/shockwave bit `0x80`
- `0x1A` (self) / `0x19` (target): `0x80000094` / `0x1A000086` — **Piercing + Breaking** (`0x10`) + ground bit (`0x80`)
- `0x1C` (self) / `0x13` (target): `0x80000014` / `0x1A000006` — **Piercing + Breaking** (`0x10`)
- `0x2B` (self) / `0x2C` (target): `0x8000000C` / `0x1A00000E` — **Piercing + Thrown/Spear** (`0x08`), NOT Sword-flagged
- `0x31` (self) / `0x37` (target): `0x84000004` / `0x5A101F76` — Destructible piercing attack entity (DarkMan bats)
- `0x46` (self) / `0x47` (target): `0xC1000014` / `0x3E000006` — Neutral obstacle-clearing check (MetalMan windup)

##### 1. DesrtMan V1–V5 (`0x0F7..0x0FB`)
- **ROM Flow (`0x080C0E60` & `0x080DF428`)**:
  - Emerges from user's panel (`08-03`, anim 3 for 17f, SFX `0x11B`, shadow ON). Checks `IsPanelSolid` (`0x080C0F2E`); if hole, sinks back (`anim 1`, 17f) and exits.
  - Cycles anim `0 -> 1 -> 2` (8f each) for 40f (`0x28`), then calls `0x080C1058` to spawn two `AttackObj 0x4B` sand fists at `(user.x + dir, user.y - 1)` (`r4=0`, anim 9) and `(user.x + dir, user.y + 1)` (`r4=1`, anim 10) wherever `CheckPanelFlags(..., req=0x10)` passes.
  - Every active frame of `AttackObj 0x4B` (`0x080DF4F8`), if its current panel is solid, it calls `SetPanelType(x, y, 0x0A)` (`0x0800B790`), turning the panel into **Sand**!
  - During the 14f emerge (`SFX 0x11B`) + 20f hold + 11f windup (`anim 6/7`), `0x080DF6B6` polls player buttons (`[player_data, #0x3E]`, mirrored via `0x0800710C` for side 1):
    - Upper fist (`r4=0`): pressing `Down + Forward + A` (`0x91` within 2f window) selects `vel[2] = (+4.2875*dir, +2.5725)` and plays SFX `0xC8`.
    - Lower fist (`r4=1`): pressing `Up + Forward + A` (`0x51`) selects `vel[1] = (+4.2875*dir, -2.5725)` and plays SFX `0xC8`.
    - Default (`vel[0]`): `(+5.0*dir, 0.0)`.
  - Launches with SFX `0xBD`, hitbox `region=0x01`, `element=0`, `self=0x12` (piercing), `target=0x13`, `hit_mod=3`, spark `0x02`. Disables hitbox on hit (`0x080DF4DC`) and keeps travelling to edge converting panels to Sand. DesrtMan waits until both fists finish (`[r5,#0x7] >= 2`), then sinks (`anim 1`, 17f).

##### 2. FlamMan V1–V5 (`0x101..0x105`)
- **ROM Flow (`0x080C6038` & `0x080E344C`)**:
  - Appears on user's panel (`08-05`, anim 0, SFX `0x92`, shadow ON, spawns candle overlay `Eff 0x1E` `08-05` anim 7). Checks `IsPanelSolid` (`0x080C617E`).
  - During appear/charge (`0x080C634A`), listens for **B (`0x02`) + D-pad (`0xF0`)** directional sequence (`0x080C64BA`/`0x080C64D4`), counting presses into `[r5, #0x64..0x6B]`.
  - Plays `anim 3` (20f) -> `anim 4` (15f, evaluates command table `0x080C6510` into wave mask `[r5, #0x6C..0x75]`) -> `anim 6` (20f, with breath overlay `Eff 0x1D` `08-05` anim 21) -> `anim 5` (70f).
  - Spawns initial Fire Pillar (`AttackObj 0x6A`, sprite `10-22` anim `0->1->2`, SFX `0x80`, 40f lifetime, `element=2` Fire, `region=0x01`, `self=0x12` piercing, `target=0x13`, `hit_mod=3`, spark `0x05`) at `(user.x + dir, user.y)` if in-bounds & solid.
  - **Normal mode (`[r5,#0x5]==0`)**: Every 20f (`0x080C6390`), moves `x += dir` and applies player's Up (`0x40` -> `dy=-1`) or Down (`0x80` -> `dy=+1`) input (clamped to rows 1..3), spawning the next `AttackObj 0x6A` pillar until hitting OOB or a hole.
  - **Command mode (`[r5,#0x5]==1`)**: Every 20f (`0x080C6406`), fires up to 2 expanding waves from `(user.x + dir, user.y)` using offset table `0x080C649C`: Wave 0 = `(+1,-1), (+1,0), (+1,+1)`; Wave 1 = `(+2,-2), (+2,-1), (+2,0), (+2,+1), (+2,+2)` gated by directional counts.

##### 3. DrillMan V1–V5 (`0x106..0x10A`)
- **ROM Flow (`0x080C4F00`)**:
  - **Phase 1 (Horizontal Drills, `0x080C4F80` + `0x080EFBA0` + `0x080E26EC`)**: Scans back column (`x=1` side 0 / `x=6` side 1, `y=1..3`) filtering `forbid = 0x1E000000` (side 0) / `0x2E000000` (side 1), shuffles valid rows. Every 20f, spawns 2 standalone horizontal drills (`AttackObj 0x64` `r4=0`, `08-06` anim `14->15`, SFX `0x12C`, `vx=8.0*dir`) and 1 DrillMan charge (`08-06` anim 5 + attached `AttackObj 0x64` `r4=1` anim `11->13`) with dirt `Eff 0x1A` (`08-06` anim 6). Each drill hitbox has `self=0x1C` (piercing+break), `target=0x13`, `hit_mod=3`, resetting hit tracking per panel crossed.
  - **Phase 2 (3 Vertical Emerging Drills, `0x080C50B4` + `0x080C52A8` + `0x080E293C`)**: Picks up to 3 random enemy-side panels (`req=0x20` / `forbid=0x20`), spawns 3 `AttackObj 0x65` (`08-06` anim `10->12`; 1 with DrillMan anim 4 launching upward at `vz=+10.0 px/f` to `z>160`). Flashes target panels for 30f (`0x0800B76C`), then erupts (SFX `0xF3` + `0x12C`), calls `CrackPanel` (`0x0800B64C`) + dirt effect `0x15`, and hits with `self=0x1A` (piercing+break+ground), `target=0x19`, `hit_mod=3`.
  - **Phase 3 (3 Falling Boulders, `0x080C5232` + `0x080C52F8` + `0x080E1E78`)**: Shakes camera (`0x08022B80(0,10)`), picks up to 3 random **solid** enemy panels (`req=0x30` / `req=0x10, forbid=0x20`), drops 3 `AttackObj 0x5F` boulders (sprite `10-35`, `z=160`, accel `0x1000/f²`, activates hitbox at `z < 30`, `self=0x12`, `target=0x13`, `hit_mod=3`, spark `0x08`, SFX `0x95` ground / `0x101` hit).

##### 4. MetalMan V1–V5 (`0x10B..0x10F`)
- **ROM Flow (`0x080BE2C0`)**:
  - Appears on user's panel (`08-0a`, anim 0, shadow hidden, spawns gear overlay `Eff 0x11` `08-0a` anim 9 with shadow, warp puff `0x03`, SFX `0x92`).
  - **30-frame Aiming (`0x080BE3F0`)**: Flashes target panel (`0x0800B76C`). While holding `A` (`0x01`), D-pad moves `(tx, ty)` freely across `x∈[1..6], y∈[1..3]`.
  - Checks `0x080BE77E` (enemy occupancy on `(tx, ty)`). Teleports with warp puffs (`SpawnSimpleEffect` 3 & 4), plays `anim 3` + SFX `0xB4` (40f windup, spawning 0-dmg `0x46/0x47` hitbox on MetalMan's panel), then `anim 4` (6f) -> smashes **`(metal.x + dir, metal.y)`** (`region=0x01`, `self=0x1A` piercing+break, `target=0x19`, `hit_mod=3`, spark `0x09`).
  - If `(metal.x + dir, metal.y)` is solid: breaks/cracks panel (`0x0800B64C`), spawns 0-dmg enemy-area shockwave (`0x1819FE82/81`, `element=5`), shakes camera `(1, 65)`, spawns effect `0x24` at `(x + 40*dir, y)`, plays SFX `0x7E` (`crack-shot`). Waits 90f, warps out.

##### 5. Punk (`0x110`)
- **ROM Flow (`0x080C9B78`)**:
  - Spawner `0x080CA32E` subtracts `180` (`0xB4`) from damage word so `[r5, #0x2C]` stores bonus damage `b`.
  - Appears on user's panel (`08-0b`, anim 0, shadow ON, SFX `0x92`), waits 31f while `0x080CA4A0` polls command inputs. Aborts on user hole (`IsPanelSolid`).
  - `0x080CA528` selects among **5 moves**:
    1. **Dual Rollers + 2× Wide Slash (`Action 0x14` -> `0x10`)**: Requires Link/special flag `[r5,#0xD]`, back-center panel `[r5,#0xE]`, special state `[r5,#0xF]`, AND command `Down -> Fwd -> Down -> Fwd`. Spawns 2× `AttackObj 0x77` (`10-43`) on rows 1 & 3 dealing `100 + b` each (`self=0x12`, `target=0x13`, `hit_mod=3`), then executes `Action 0x10`.
    2. **Giant Ball (`Action 0x0C`)**: Triggered by **B + Back (`0x02` + `0x20`)**. Plays `anim 3` + SFX `0xBD` (24f), spawns `AttackObj 0x74` (`10-40`) sweeping the row for **`400 + b` damage**, `self=0x12`, `target=0x13`, `hit_mod=0`, status byte **`0x20`**.
    3. **2× Wide Slash (`Action 0x10`)**: Triggered by `[r5,#0xE] & [r5,#0xF]`. Teleports ahead of front-most enemy col on row 2 (`0x080CA388`), plays `anim 4 -> 5`, SFX `0x7F`, fires two `region=0x04` (wide) hitboxes (7f apart) for **`200 + b` damage** each (`self=0x12`, `target=0x13`, `hit_mod=3`) + slash FX `0x3F`.
    4. **Sine-Wave Roll (`Action 0x08`)**: Triggered when `HP <= MaxHP / 4` (`0x080CA578`) OR `[r5,#0xD] != 0`. Plays `anim 6` (9f) -> `anim 7`, SFX `0x133`, rolls forward in a sine wave (`vx=4px/f`) dealing **`360 + b` damage** per panel (`self=0x1C`, `target=0x13`, `hit_mod=3`), afterimages every 4f, aborts on hole.
    5. **Perimeter Loop Roll (`Action 0x04`, Default)**: Plays `anim 6` (9f) -> `anim 7`, SFX `0x133`, rolls vertically (`8px/f`) to edge -> horizontally to nearest enemy column (`0x080CA038`) -> vertically across rows -> back to start panel, dealing **`180 + b` damage** per panel (`self=0x1C`, `target=0x13`, `hit_mod=3`), afterimages every 4f, aborts on hole.

##### 6. KingMan V1–V5 (`0x115..0x119`)
- **ROM Flow (`0x080C8304` & `0x080E4C20`)**:
  - Spawns at `z = 140` (`0x8C << 16`) over user's panel (`08-0c`, anim 0, shadow ON), drops with gravity `-0xAAAB/f` (no `0x92` SFX). If user's panel is a hole (`IsPanelSolid`), puffs out (`SpawnSimpleEffect` id 4) immediately. On landing: SFX `0x7E` + `ShakeCamera(1, 10)`.
  - `anim 3` for 30f, then jumps in parabolic arc (`vz=+7.0`, `g=-0.375`) to **fixed `(user.x + 3*dir, user.y)`**, flashing the destination panel (`0x0800B76C`).
  - On landing (`0x080C84E8`), requires BOTH `IsPanelSolid` AND `CheckPanelFlags(dest_x, dest_y, req=0x10, forbid=0x3E000000)` (panel must be solid, in-bounds, and **completely unoccupied**!). If failed -> **fizzles** in puff (`SpawnSimpleEffect` id 6) without dealing damage!
  - On valid landing: SFX `0x7E`, `ShakeCamera(0, 10)`, spawns center hitbox (`region=0x01`, `self=0x1A` piercing+break, `target=0x19`, `hit_mod=3`) + 4× `AttackObj 0x70` (`10-3e`, anim 0, SFX `0xB2`, 20f) at `(-1,0), (+1,0), (0,+1), (0,-1)` (`region=0x01`, `self=0x12` piercing non-breaking, `target=0x13`, `hit_mod=3`, spark `0x05`; skips holes via `IsPanelSolid`).

##### 7. DarkMan V1–V5 (`0x124..0x128`)
- **ROM Flow (`0x080D1D44`, `0x080EA5B8`, `0x080EA75C`, `0x080E37B0`)**:
  - Appears on user's panel (`08-0f`, anim 0, shadow ON, SFX `0x92`), waits 31f, checks `IsPanelSolid` (`0x080D1E3E`), then waits 60f (`0x3C`).
  - **Only Navi in `part2.luau` that reads V1–V5 tier param `[r5, #4]` (`0x080D1E96`)**: looks up total bat count from `0x080D1EB4` = **`{16, 18, 20, 22, 24}`** (`0x10, 0x12, 0x14, 0x16, 0x18`)!
  - Spawns `AttackObj 0x99` (`0x080EA5B8`, interval 12f, count `16..24`, damage `30`), which opens dark portals (`Eff 0x29`, `08-0f` anim 3, SFX `0x93`) at the top of all enemy columns containing an enemy-owned panel (`0x080EA6CA`).
  - Every 12f, spawns a destructible (1 HP) `AttackObj 0x9A` bat (`08-0f` anim 6, SFX `0x137` every 32f) flying vertically down the column (`vy = +1.6 px/f`, ~15f/panel) with `region=0x01`, `element=0`, `self=0x31` (`0x84000004`), `target=0x37` (`0x5A101F76`), `hit_mod=3`, and **NO status effects** (`[hitbox,#0x4]` never set).
  - When fading out (`0x080D1F18`), clears any existing player DarkHole (`0x080B5712`) and spawns **DarkHole (`AttackObj 0x6C`, `0x080E37B0`, sprite `10-42`, SFX `0x93` appear / `0x8F` open)** at `(user.x + dir, user.y)` if solid & unoccupied (`req=0x10, forbid=0x3E080080`). Stays open 300f then active 100f setting battle flag `0x16 + side` (`0x080012B4`), satisfying `0x080B595C` required to activate Bass / Bass+ / Serenade.

##### 8. JapanMan V1–V5 (`0x129..0x12D`)
- **ROM Flow (`0x080D391C`)**:
  - Appears on **user's panel** (`08-10`, anim 0, shadow ON, SFX `0x92`, spawns attached spear overlay `Eff 0x0B` `08-10` anim `7 + japanman.anim`). Checks `IsPanelSolid` (`0x080D3A2E`).
  - Waits 30f, switches to `anim 6` (spear flurry) for 48f firing a hitbox every 16f (`0x080D3C7A` -> 3 guaranteed hits).
  - **Alternating A/B Button Mashing (`0x080D3C4E`, table `0x080D3C78 = {0x01, 0x02}`)**:
    - During Stage 1 (48f): `≥ 8` alternating presses adds `+32f` (+2 hits); `6–7` presses adds `+16f` (+1 hit); `< 6` goes to finisher.
    - During Stage 2 (16f or 32f): `≥ 3` alternating presses adds `+16f` (+1 hit) in Stage 3.
    - Total flurry hits = **3 to 6** (plus 1 finisher = **4 to 7 total hits**) identically across all V1–V5 tiers; only damage per hit scales (`40/45/50/55/60`).
  - **Flurry Hitbox (`0x080D3C98`)**: Centered at **`(user.x + 2*dir, user.y)`**, `region = 0x04` (`regions.wide`, 3-tall vertical column!), `z = 0x16_0000`, `element = 0`, `self = 0x2B` (`0x8000000C`, piercing+thrown, NOT sword!), `target = 0x2C`, `hit_mod = 1`, spark `0x09`, SFX `0x132`.
  - **Finisher (`0x080D3B0E..0x080D3B98`)**: 20f pause (`anim 0`), then `anim 5` + SFX `0x131`, spawning 3 sequential `region=0x01` hitboxes (4f apart, 4f active each) at `(user.x + 1*dir, y)`, `(user.x + 2*dir, y)`, `(user.x + 3*dir, y)` with `self = 0x2B`, `target = 0x2C`, `hit_mod = 3`.


---

### 5.8 Group 8：Program Advance 程序强化（全部 32 张，含配方表校验）

- **ROM Audited**: `/home/harp/projects/gba/roms/bn3.gba` (Mega Man Battle Network 3 White US — `A6BE`)
- **Target Code**: `/home/harp/projects/saito/content/exe3/chips/pa/init.luau` (plus `compat/assets.toml`, `compat/chips.toml`, `lib/anims.luau`, `lib/swords.luau`, `lib/barriers.luau`, `objects/shuriken/init.luau`, `objects/bullet/init.luau`)
- **Scope**: All **32 Program Advances** (`0x140` `Z-Canon1` through `0x15F` `MstrStyl`) + ROM PA Recipe Tables (`0x0800D564..0x0800D758`).
  - *(Note: `bassgs` `0x138` also resides at the bottom of `pa/init.luau` lines 1926–1981, but is a Giga chip outside the 32 PA IDs `0x140..0x15F`.)*

---

#### Executive Summary

| Severity | Count | Chips |
| :--- | :---: | :--- |
| **P0** | **27** | All 12 Zeta PAs (`Z-Canon1..3`, `Z-Punch`, `Z-Strght`, `Z-Impact`, `Z-Varibl`, `Z-Yoyo1..3`, `Z-Step1..2`); 5 non-dimming PAs wrongly dimmed or missing core mechanics (`H-Burst`, `EvilCut`, `HyperRat`, `EverCrse`, `500Barr`); 10 dimming PAs with severe mechanics/summon/object/hit-count mismatches (`TimeBom+`, `2xHero`, `GelRain`, `PoisPhar`, `BigHeart`, `GtsShoot`, `DeuxHero`, `MomQuake`, `PrixPowr`, `MstrStyl`) |
| **P1** | **4** | `BubSprd`, `HeatSprd`, `ElemSwrd`, `BodyGrd` |
| **P2** | **0** | *(All presentation mismatches occur on PAs already classified P0 or P1 and are documented in each row.)* |
| **OK** | **1** | `LifeSwrd` (`0x150`) |
| **Total** | **32** | `0x140..0x15F` |

##### Key Verified ROM Table Anchors (`A6BE` / `bn3.gba`)
- `PlayerActionJumpTable`: `0x080B0DA4` (`family * 4`)
- `DimmingEffectJumpTable`: `0x08008D00` (`effect_id * 4`)
- `ActorObjJumpTable` (Type 1, `0x0800304C`): **`0x0800369C`** (0-indexed; index 0 = `0x080B02C9` MegaMan)
- `AttackObjJumpTable` (Type 3, `0x08003084`): `0x080038C4`
- `EffectObjJumpTable` (Type 4, `0x080030D8`): `0x08003C18`
- `CollisionFlagTable` (`0x080AF074`): `0x080AF51C` (`0x01`=normal `0x80000000`, `0x12`=breaking `0x80000004`, `0x16`=piercing+breaking+invis-pierce `0x8000009D`, `0x18`=thrown+breaking `0x80000084`, `0x26`=slash `0x80000008`, `0x28`=thrown+slash `0x80000088`, `0x2B`=slash+breaking `0x8000000C`)
- `PA Recipe Table List`: `0x0800D564` (`[0]=0x0800D57C` seq3, `[1]=0x0800D640` seq4, `[2]=0x0800D674` seq5, `[3]=0x0800D680` run3, `[4]=0x0800D750` run4, `[5]=0x0800D754` run5)
- `CheckAntiNaviTrigger`: `0x08008EA0` (called by `BigHeart`, `2xHero`, `DeuxHero`, `GtsShoot`, `PrixPowr`, `MstrStyl`)
- `PlaySoundEffect`: `0x080005B8`
- **BN3 Element IDs**: `0=Null, 1=Elec, 2=Fire, 3=Aqua, 4=Wood, 5=Drain` *(Saito uses `0=Null, 1=Fire, 2=Aqua, 3=Elec, 4=Wood` in `rules/elements.luau`)*.

---

#### Complete 32-PA Audit Table

| Chip Name | ID (hex) | File | Severity | Current Implementation vs ROM Behavior Diff | Verified ROM Addresses & Correct Sprite / SFX IDs |
| :--- | :---: | :--- | :---: | :--- | :--- |
| **Z-Canon1** | `0x140` | `chips/pa/init.luau` | **P0** | Locks MegaMan in place (`using_action=true`), lasts 180t, uses `flash`+`invisible`, caps at 6 single-hit `hit_mod=3` shots with auto-fire. **ROM**: Grants **300t (5s)** true Invis (flashing alpha, `0x91`) while MegaMan **moves freely** and fires unlimited hidden chip `0x16E` (`AttackObj[0x15]`, **3-hit** 40x3 cannon per A-press: initial `hit_mod=0`, +8t `hit_mod=0`, +8t `hit_mod=3`). | Fam 23 `0x080B23FC`, Zeta table `0x080B2450` (`0x16E`, 300t), Timer `0x080B5444`, A-reuse `0x080B4750`, Cannon `0x080B1218`→`0x080D8158` (`AttackObj[0x15]` `0x080D8018`). Sprite: `0c-01` pal `0` (`0x080B1228`), SimpleFx `0x08`/`0x16`. SFX: Invis `0x91`, shot `0x74` (NOT `0x7D`). |
| **Z-Canon2** | `0x141` | `chips/pa/init.luau` | **P0** | Same Zeta state bugs as Z-Canon1 (locked in place, 180t vs 300t, fake invis, 6-shot cap, single-hit `hit_mod=3`). **ROM**: 300t free-move Invis + unlimited `0x16F` (`AttackObj[0x15]`, 60x3 per A-press). | `0x080B23FC`, `0x080B2458` (`0x16F`, 300t), `0x080B1218`→`0x080D8018`. Sprite: `0c-01` pal `1`, SimpleFx `0x08`/`0x16`. SFX: `0x91`, `0x74`. |
| **Z-Canon3** | `0x142` | `chips/pa/init.luau` | **P0** | Same Zeta state bugs as Z-Canon1 (locked in place, 180t vs 300t, fake invis, 6-shot cap, single-hit `hit_mod=3`). **ROM**: 300t free-move Invis + unlimited `0x170` (`AttackObj[0x15]`, 80x3 per A-press). | `0x080B23FC`, `0x080B2460` (`0x170`, 300t), `0x080B1218`→`0x080D8018`. Sprite: `0c-01` pal `2`, SimpleFx `0x08`/`0x16`. SFX: `0x91`, `0x74`. |
| **Z-Punch** | `0x143` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 6-shot cap, spawns simplified single-panel breaking hitbox (`hit_mod=3`) with no obstacle push/slide mechanic. **ROM**: 300t free-move Invis + unlimited real `GutPunch` (`0x2F`) on every A-press (pushes RockCubes/obstacles across row). | `0x080B23FC`, `0x080B2468` (`0x2F` `GutPunch`, 300t), `0x080B5444`/`0x080B4750`. Uses real `GutPunch` anim/sprites/SFX + Invis `0x91`. |
| **Z-Strght** | `0x144` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 6-shot cap, simplified 2-panel `long` hitbox with no obstacle push/slide. **ROM**: 300t free-move Invis + unlimited real `GutStrgt` (`0x30`) on every A-press. | `0x080B23FC`, `0x080B2470` (`0x30` `GutStrgt`, 300t). Uses real `GutStrgt` anim/sprites/SFX + Invis `0x91`. |
| **Z-Impact** | `0x145` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 6-shot cap, simplified single-panel hitbox without real `GutImpct` behavior (push + windup/hit). **ROM**: 300t free-move Invis + unlimited real `GutImpct` (`0x31`) on every A-press. | `0x080B23FC`, `0x080B2478` (`0x31` `GutImpct`, 300t). Uses real `GutImpct` anim/sprites/SFX + Invis `0x91`. |
| **Z-Varibl** | `0x146` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 6-shot cap, only swings basic `regions.wide` with **NO VarSwrd command inputs**! **ROM**: 300t free-move Invis + unlimited real `VarSwrd` (`0x26`) with **full command inputs** (SonicBoom, DreamSword, FighterSword, ElementalSonic, Wide, Long) on every A-press. | `0x080B23FC`, `0x080B2480` (`0x26` `VarSwrd`, 300t). Uses real `VarSwrd` command-input handler/sprites/SFX + Invis `0x91`. |
| **Z-Yoyo1** | `0x147` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 9-shot cap, fires instant 3-panel `regions.line` hitboxes instead of physical boomerang projectiles! **ROM**: 300t free-move Invis + unlimited real `Yo-Yo1` (`0x45`) physical 3-hit boomerang projectiles on every A-press. | `0x080B23FC`, `0x080B2488` (`0x45` `Yo-Yo1`, 300t). Uses real `Yo-Yo1` projectile/sprites/SFX + Invis `0x91`. |
| **Z-Yoyo2** | `0x148` | `chips/pa/init.luau` | **P0** | Same as Z-Yoyo1: locked in place, 180t, 9-shot cap, instant `regions.line` instead of real `Yo-Yo2` (`0x46`) boomerangs. **ROM**: 300t free-move Invis + unlimited real `Yo-Yo2` (`0x46`). | `0x080B23FC`, `0x080B2490` (`0x46` `Yo-Yo2`, 300t). Uses real `Yo-Yo2` projectile/sprites/SFX + Invis `0x91`. |
| **Z-Yoyo3** | `0x149` | `chips/pa/init.luau` | **P0** | Same as Z-Yoyo1: locked in place, 180t, 9-shot cap, instant `regions.line` instead of real `Yo-Yo3` (`0x47`) boomerangs. **ROM**: 300t free-move Invis + unlimited real `Yo-Yo3` (`0x47`). | `0x080B23FC`, `0x080B2498` (`0x47` `Yo-Yo3`, 300t). Uses real `Yo-Yo3` projectile/sprites/SFX + Invis `0x91`. |
| **Z-Step1** | `0x14A` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 5-shot cap, **never steps forward** (just swings `wide` from current tile!). **ROM**: 300t free-move Invis + unlimited real `StepSwrd` (`0x27`, 2-panel step-forward WideSwrd + warp back) on every A-press. | `0x080B23FC`, `0x080B24A0` (`0x27` `StepSwrd`, 300t). Uses real `StepSwrd` handler/sprites/SFX + Invis `0x91`. |
| **Z-Step2** | `0x14B` | `chips/pa/init.luau` | **P0** | Locked in place, 180t, 5-shot cap, **never steps forward** (just swings `x` from current tile!). **ROM**: 300t free-move Invis + unlimited real `StepCros` (`0x28`, 2-panel step-forward X-slash + warp back) on every A-press. | `0x080B23FC`, `0x080B24A8` (`0x28` `StepCros`, 300t). Uses real `StepCros` handler/sprites/SFX + Invis `0x91`. |
| **TimeBom+** | `0x14C` | `chips/pa/init.luau` | **P0** | Indestructible fake timer at fixed `(enemy_front_col, 2)`, 120t countdown, deals `piercing_break` (`hit_mod=5`) on detonation. **ROM**: Spawns **destructible 200-HP obstacle** on a **random valid unoccupied enemy panel**, **210t** countdown (60+60+60+30), detonates as **normal attack** (`0x01/0x02`, NOT piercing/breaking!) across enemy area (`0x82/0x81`), 500 dmg, **`hit_mod=3`**. | Dim eff `0x12` p=1 `0x080F9A90`, HP table `0x080F9BEC[1]=200`, `AttackObj[0x29]` `0x080DADF8`. Sprite: **`0c-39`** (NOT `0c-02` `bomb`). SFX: ticks `0x8A`×3 + `0x8B`, blast **`0xED`** + field explosions `0x080EEDA8` + shake `(1,60)`. |
| **H-Burst** | `0x14D` | `chips/pa/init.luau` | **P0** | Implemented as **dimming** chip auto-targeting `enemy_target` with `regions.plus`/`regions.square`, `hit_mod=1/3`, `FLASH_FX`, `BLAST_SOUND`. **ROM**: **NON-DIMMING** buster shot (`b0B=0x01`) travelling down MegaMan's row (`1 panel/tick`); on hitting a target, triggers **5** alternating explosions (regions `0x0B`/`0x09`, 8t apart, 100 dmg each, **`hit_mod=0`** non-flinching!). | Fam 24 `0x080B24B0`, `AttackObj[0x14]` `0x080D7E92`, count `0x080D7E90[0]=5`. SimpleFx `0x18` muzzle, region fx `0x17`. SFX: **`0x7D`** (`cannon`), NOT `0x74`/`0x76`. |
| **BubSprd** | `0x14E` | `chips/pa/init.luau` | **P1** | **Missing 3 ROM code-run recipes** (`Bubbler C/D/E`, `Bub-V D/E/F`, `BublSide E/F/G`); uses `hit_mod=3` instead of ROM `0`; uses `SPREADER_SOUND` (`0x74`) for launch & impact and omits muzzle flash. **ROM**: Non-dimming, 300 Aqua, 3x3 (`0x0F`), `hit_mod=0`, launch SFX `0x7D` + SimpleFx `0x1F`, impact fx `0x0F` (`bubble-splash`) + SFX `0xAE`. | Fam 6 sub 7 `0x080B14D2`, `AttackObj[0x02]` `0x080D5E50` (`0x080D5FC0` `r3=0`), recipes `0x0800D57C[0]` & `0x0800D680[12..14]`. Fx `0x1F`/`0x0F`. SFX: launch `0x7D`, impact **`0xAE`**. |
| **HeatSprd** | `0x14F` | `chips/pa/init.luau` | **P1** | **Missing 3 ROM code-run recipes** (`HeatShot H/I/J`, `Heat-V I/J/K`, `HeatSide J/K/L`); uses `hit_mod=3` instead of ROM `0`; uses `SPREADER_SOUND` (`0x74`) for launch & impact and omits muzzle flash. **ROM**: Non-dimming, 300 Fire, 3x3 (`0x0F`), `hit_mod=0`, launch SFX `0x7D` + SimpleFx `0x1F`, impact fx `0x10` (`heat-blast`) + SFX `0xA9`. | Fam 6 sub 11 `0x080B14D2`, `AttackObj[0x02]` `0x080D5E50`, recipes `0x0800D57C[1]` & `0x0800D680[15..17]`. Fx `0x1F`/`0x10`. SFX: launch `0x7D`, impact **`0xA9`**. |
| **LifeSwrd** | `0x150` | `chips/pa/init.luau` | **OK** | Matches ROM mechanics, recipe (`Sword+WideSwrd+LongSwrd`), non-dimming 400 Null, `regions.wide_long` (2x3), `collision.slash`, `hit_mod=3`, blade `0c-00`, slash `0c-0D` anim 0 pal 0, SFX `0xD6`. | Fam 8 sub 13 `0x080B16CE`, `AttackObj[0x06]` `0x080D65CC[13]` (`00 00 0D 11 00 02 00 03`), recipe `0x0800D57C[2]`. Blade `0c-00`, slash `0c-0D`, SFX `0xD6`. |
| **ElemSwrd** | `0x151` | `chips/pa/init.luau` | **P1** | Uses **`regions.wide_long` (2x3)** instead of ROM **`0x04` `wide` (1x3)**; `hit_mod=1,1,1,3` instead of ROM **`0`** + `[coll+4]=0x10`; fixed `timer%8==2` instead of full swing anim per slash; SFX `0xD6` instead of `0x75`; hit-sparks instead of `0c-0C` slash. **ROM**: 4 non-dimming **1x3 Wide** slashes (150 each: Fire→Aqua→Elec→Wood), blades `0c-08..0b`, slash `0c-0C`, `hit_mod=0`, SFX `0x75`. | Fam 8 sub 14 `0x080B16CE` (`0x080B180A..0x080B189E`), elem table `0x080B2F04`, `AttackObj[0x06]` `0x080D65CC[14]` (`00 00 0C 04 10 02 00 00`). Blades `0c-08..0b`, slash **`0c-0C`**, SFX **`0x75`**. |
| **EvilCut** | `0x152` | `chips/pa/init.luau` | **P0** | Implemented as **dimming** chip auto-targeting `enemy_target` with no step-forward or panel check. **ROM**: **NON-DIMMING** (`b0B=0x01`) Step-family! Steps forward **2 panels** (fails if target panel invalid/hole/occupied), performs 3 slashes (150 each, dark flash `0x200`): `0x04` Wide (`0c-0C` anim 0 pal 0, `hit_mod=1`), `0x06` X-Cross (`0c-0C` anim 4 pal 2, `hit_mod=1`), `0x12`+`0x13` (`0c-35` anim 0 pal 0, `hit_mod=3`), then warps back. | Fam 41 sub 3 `0x080B373C`, step dist `0x080B3824[3]=2`, `AttackObj[0x06]` rows `0x11/0x12/0x10` + `AttackObj[0x07]` `0x26270213`. Sprites: `0c-0C` (anim 0 pal 0 & anim 4 pal 2), **`0c-35`** anim 0 pal 0. SFX: **`0x75`**. |
| **2xHero** | `0x153` | `chips/pa/init.luau` | **P0** | Hits `regions.wide` around `enemy_target` with plain `slash` and `hit_mod=1..1,3`; no AntiNavi trigger check; no MegaMan/ProtoMan actors. **ROM**: Calls `CheckAntiNaviTrigger` (`0x08008EA0`), spawns MegaMan (`00-00`) + ProtoMan (`08-09`), deals **10 hits × 70 = 700** across the **ENTIRE ENEMY AREA** (`0x82/0x81`) as **slash+breaking (`0x2B/0x2C`)** with **`hit_mod=3`**. | Dim eff `0x41` p=1 `0x080FD2B8`, `ActorObj[0x54]` `0x080CF558` & `[0x55]` `0x080CF860`, table `0x080CF774` (`[7,10,20,10]`), hitbox `0x080CF7C8` (`0x2B2CFE82/81`). Sprites: `00-00`, `08-09`, SimpleFx `0x49`. SFX: `0x6E` (buster), `0x75` (sword). |
| **HyperRat** | `0x154` | `chips/pa/init.luau` | **P0** | Implemented as **dimming** chip auto-targeting `enemy_target` with piercing `regions.plus` blast (`hit_mod=5`). **ROM**: **NON-DIMMING** (`b0B=0x01`) Ratton missile! MegaMan plays `0x23` (`THROW_LOW`), spawns physical Ratton (`0c-1F` pal 1) on `(x+fwd, y)` (requires non-hole), moves `4 px/tick`, turns once vertically toward enemy, hits single target for **500 Null non-piercing** (`0x01/0x02`, `hit_mod=2`). | Fam 29 sub 3 `0x080B2BE4`, `AttackObj[0x23]` `0x080D9C4C`. Sprite: **`0c-1F` pal `1`** (NOT `0c-20`!). SimpleFx `0x00` + `0x4F`, shake `(1,20)`. SFX: launch **`0xB0`** (`zapring`), explode `0x76`. |
| **EverCrse** | `0x155` | `chips/pa/init.luau` | **P0** | Implemented as **dimming** chip dealing instant 5×100 piercing hits to `enemy_target` with no shield or counter stance. **ROM**: **NON-DIMMING** (`b0B=0x01`) CrsShld counter! Raises shield (`0c-03` anim 6 pal 0, 26t); if hit while guarding, fires CrsShld bite (`0c-03` anim 9 pal 2, 12 px/tick, 100 dmg, `0x111`), then spawns persistent **`AttackObj[0x71]`** lasting **600t (10s)** that repeatedly counter-bites when attacked. | Fam 5 sub 6 `0x080B12D4`, `AttackObj[0x44]` `0x080DE380` → `AttackObj[0x71]` `0x080E4D68` (`0x258`=600t). Sprite: **`0c-03`** (anim 6 pal 0 shield, anim 9 pal 2 bite). SFX: `0x8C` (guard), **`0x111`** (bite). |
| **GelRain** | `0x156` | `chips/pa/init.luau` | **P0** | Only 3×150 `regions.wide` non-breaking hits + steals 1 column on hit 3; adds bogus `MetaGel1+2+3` sequence recipe; wrong sprite/SFX. **ROM**: Drops **6 individual MetaGel objects** (spaced 32t, `04-1C` anim 2→3 random pal 0..3), each hitting 1 panel for **150 Aqua thrown+breaking (`0x18/0x19`)** `hit_mod=3` and stealing that panel if unoccupied, advancing across up to **2 columns (6 panels)**. Recipes are code-runs ONLY (`MetaGel1 B/C/D`, `MetaGel2 E/F/G`, `MetaGel3 S/T/U`). | Dim eff `0x4F` p=0 `0x080FDD48`, `AttackObj[0x3D]` `0x080DD580`, recipes `0x0800D680[48..50]`. Sprite: **`04-1C`** (anim 2→3, pal 0..3). SFX: fall **`0xDA`**, land **`0xAC`** (`bubble`). |
| **PoisPhar** | `0x157` | `chips/pa/init.luau` | **P0** | Indestructible controller lasting 540t that directly subtracts `opp.hp -= 2` every 2t (bypasses hitbox/objects/viruses); wrong sprite `anubis` (`0c-2B`). **ROM**: Drops **destructible 300-HP** Pharaoh statue lasting **6000t (100s)**, falls 20t + shake `(2,30)` + breaks on invalid panel; every **1 tick** spawns enemy-area drain hitbox (`elem=5`, `0x3C/0x02`, 1 dmg, `hit_mod=0` = **60 HP/s**). | Dim eff `0x21` p=1 `0x080FAB04`, `AttackObj[0x4C]` `0x080DF75C`, table `0x080DF74C[1]` (HP 300, period 1, anim 1, lifetime `0x1770`). Sprite: **`0c-29` anim `1`** (NOT `0c-2B`!). Poison cloud `0x080EE26E` every 16t. SFX: land `0x7E`, destroy `0x76`. |
| **BodyGrd** | `0x158` | `chips/pa/init.luau` | **P1** | Only throws **12 shurikens** (1200 dmg) at 16t intervals with `hit_mod=3` on final hit; uses anim `1` + SFX `sword-swing`. **ROM**: Undims after 20t; throws **18 shurikens (1800 dmg)** every **20 ticks**, ALL 18 hits have **`hit_mod=1`** (thrown+slash `0x28/0x29`), 15t warning highlight, visible at `t=7`, hits at `t=0`, stays 30t. | Dim eff `0x50` p=0 `0x080FDEC4`, `EffectObj[0x2C]` `0x080F0C68` (`[r5+0x22]=19`→18 throws, 20t), `AttackObj[0x13]` `0x080D7CFC`. Sprite: `0c-1D` **anim `0`** (NOT `1`), SimpleFx `0x15`. SFX: throw **`0xB7`**, impact `0x7B`. |
| **500Barr** | `0x159` | `chips/pa/init.luau` | **P0** | Implemented as **dimming** chip (`flags={"dimming"}`), metadata `damage=0`, uses `barriers.barrier_200` visual (**palette `2`**). **ROM**: **NON-DIMMING** (`b0B=0x02`, `pwr=500`) instant barrier! Barrier ID `4` (`0x10`), HP 500, sprite `0c-04` anim 4 **palette `3`**, SFX `0xCE`. | Fam 3 sub 4 `0x080B117C`, barrier table `0x080B11D0[4]=0x10`, visual row 4 `0x080EE034`. Sprite: `0c-04` anim `4` **pal `3`** (NOT pal `2`!). SFX: `0xCE`. |
| **BigHeart** | `0x15A` | `chips/pa/init.luau` | **P0** | Only **6×50 = 300 dmg** `piercing` hits; heals to **full `max_hp` BEFORE attacking**; no Roll actor; no AntiNavi check. **ROM**: Calls `CheckAntiNaviTrigger`, summons Roll (`10-14`), performs **9×50 = 450 dmg** single-panel **breaking (`0x12/0x13`)** hits (3 cycles × 3 hits, 10t apart, `hit_mod=3`), then **AFTER attacking** spawns `AttackObj[0x16]` healing **300 HP** (`0x080B6F0C[5]=150`, doubled to `300` at `0x080D8252` → `0x080ADA0C`). | Dim eff `0x07` p=5 `0x080F9130`, `ActorObj[0x04]` `0x080B6A74`, heal table `0x080B6F0C[5]=0x96`, `AttackObj[0x16]` `0x080D823E` → `0x080ADA0C`. Sprite: **`10-14`** (Roll), SimpleFx `0x13` (heal). SFX: `0x92` (appear), `0x72` (heal). |
| **GtsShoot** | `0x15B` | `chips/pa/init.luau` | **P0** | Sweeps instant hitboxes across **all 6 columns `1..6`** (including behind MegaMan!), never stops on first hit, `hit_mod=5`, no GutsMan/MegaMan visuals, no AntiNavi check, wrong SFX. **ROM**: Calls `CheckAntiNaviTrigger`, summons GutsMan (`08-08`), throws MegaMan (`AttackObj[0x72]`, `00-00` anim `0x31`→`0x32`, `10 px/tick`) forward from MegaMan's tile, **breaking (`0x12/0x13`)**, 300 dmg, **`hit_mod=3`**, **stops on first hit** with 5 explosions + shake `(1,15)`. Does NOT break panels. | Dim eff `0x33` p=0 `0x080FC2C4`, `ActorObj[0x42]` `0x080C8688`, `AttackObj[0x72]` `0x080E4FA8`. Sprites: **`08-08`** (GutsMan), **`00-00`** anim `0x31/0x32` (MegaMan), SimpleFx `0x00`. SFX: `0x92`, throw **`0x94`**, boom `0x76`. |
| **DeuxHero** | `0x15C` | `chips/pa/init.luau` | **P0** | Deals **8 hits = 560 dmg** (`hero_combo_hook(8)`) instead of **7 hits = 490 dmg**! Hits `regions.wide` around `enemy_target` with plain `slash` (`hit_mod=1..1,3`); missing AntiNavi check & actors. **ROM**: Same controller as `2xHero` with `p=0`: `CheckAntiNaviTrigger`, MegaMan+ProtoMan actors, **7 hits × 70 = 490 dmg** across **ENTIRE ENEMY AREA** (`0x82/0x81`), **slash+breaking (`0x2B/0x2C`)**, **`hit_mod=3`**. | Dim eff `0x41` p=0 `0x080FD2B8`, `ActorObj[0x54]` `0x080CF558` & `[0x55]` `0x080CF860`, table `0x080CF774[0]=7`, hitbox `0x080CF7C8` (`0x2B2CFE82/81`). Sprites: `00-00`, `08-09`, SimpleFx `0x49`. SFX: `0x6E`, `0x75`. |
| **MomQuake** | `0x15D` | `chips/pa/init.luau` | **P0** | Completely wrong: does 3×200 dimmed full-area blasts + cracks all enemy panels on hit 3! **ROM**: Undims after 30t, spawns persistent **`AttackObj[0xAF]`** lasting **480t (8s)**; every 25t shakes `(2,25)`, emits 0-dmg tremor, and drops a falling **200-dmg breaking (`0x12/0x13`) boulder** (`AttackObj[0x5F]`, `hit_mod=3`, ~19 rocks) on a random enemy panel; boulders shatter on solid panels (`0x0800BF4A`) and do **NOT** crack panels. | Dim eff `0x52` p=0 `0x080FDFD4`, `AttackObj[0xAF]` `0x080ED1E0`, `AttackObj[0x5F]` `0x080E1E78` (`0x080E1FC8`, `0x0800BF4A`). Sprite: **`10-35`** (boulder), debris `0x080EFB80`, hole fx `0x06`. SFX: quake **`0xD8`** (every 32t), shatter **`0x95`**. |
| **PrixPowr** | `0x15E` | `chips/pa/init.luau` | **P0** | Completely fabricated: 3×300 dimmed `piercing_break` blasts (`hit_mod=1,1,5`) auto-targeting `enemy_target`! Plus **4 bogus recipes** accepting `KingMan V1..V4`! **ROM**: Calls `CheckAntiNaviTrigger`, summons **BowlMan (`08-0E`)** (White), rolls **6** (`14t` apart) **300-dmg breaking (`0x12/0x13`, `hit_mod=3`)** bowling projectiles (`AttackObj[0xAE]`, `5 px/tick`) down rows `y-1, y, y+1` twice! Recipes ONLY allow **`KingManV5`, `MistManV5`, `BowlManV5`**. | Fam 20 sub 86, Dim eff `0x51` p=0 `0x080FDF3C`, `ActorObj[0x64]` `0x080D4B0C`, `AttackObj[0xAE]` `0x080ED054`, recipes `0x0800D57C[15..17]`. Sprites: **`08-0E`** (BowlMan), **`10-44`** anim 1→2 (pin/ball), SimpleFx `0x43`. SFX: `0x92`, roll **`0xE0`**. |
| **MstrStyl** | `0x15F` | `chips/pa/init.luau` | **P0** | Only **6 hits = 600 dmg**, assigns **Fire/Aqua/Elec/Wood** elements to hits 1–4, all `regions.wide_long` plain slash, missing Style clones, 9th full-field finisher & AntiNavi check! **ROM**: Calls `CheckAntiNaviTrigger`, spawns 3 Style orbs (`0x109`), performs **8 Style-clone strikes** (every 30t, ALL **Null element `r0=0`**, 100 dmg, `hit_mod=3`: Punch 1-panel breaking / Sword 1x3 slash+breaking / Dash 1-panel breaking), charges 60t cycling palettes (`0xB5`), then unleashes **9th full-field blast** (`0x80`, **piercing+breaking `0x16/0x17`**, Null, 100 dmg, `hit_mod=3`) = **900 total dmg**. | Dim eff `0x53` p=0 `0x080FE0A0`, `ActorObj[0x66]` `0x080D5620`, `AttackObj[0xB2]` `0x080ED890`, orbs `0x080F0F84`, field blasts `0x080EEDA8`. Sprites: `00-00` (clones + palette cycle), flash `0x10400`/`0x12000`, shake `(3,90)`. SFX: orbs **`0x109`**, clone **`0xC5`**, charge **`0xB5`**, finisher **`0xE6`**. |

---

#### Detailed Findings by Subsystem

##### 1. Zeta PAs (`0x140..0x14B`, 12 chips — All P0)
- **ROM architecture (`0x080B23FC`, table `0x080B2450`, tick `0x080B5444`, A-reuse `0x080B4750`)**:
  - Activating any Zeta PA writes `0x012C` (**300 ticks / 5.0s**) to `[player_ext + 0x2E]`, calls `0x080B57DA(side, 300, 1)` to grant **true Invis** (`[player_ext + 0x2A] = 300`, collision flag `0x2`, flashing alpha `[ext + 0xD] = 1`) with SFX **`0x91`**, stores the underlying chip ID at `[player_ext + 0x32]` and PA ID at `[player_ext + 0x34]`, and immediately executes the underlying chip once.
  - While `[player_ext + 0x2E] > 0`, MegaMan is **NOT locked in place** — he can walk freely across panels and press **A** as many times as desired to execute the **real underlying chip** with no shot cap.
  - **Underlying chips (`0x080B2450`, 8-byte stride `[u16 chip, u16 pa, u16 ticks=300, u16 0]`)**:
    - `Z-Canon1..3` (`0x140..0x142`) → internal chips `0x16E/0x16F/0x170` (Cannon fam 4 sub 3/4/5, dmg 40/60/80). At `0x080B1282`, `sub >= 3` dispatches to `0x080D8158` (`AttackObj[0x15]` @ `0x080D8018`) instead of `AttackObj[0x00]`, playing SFX **`0x74`** (`spreader`). Each shot scans forward 1 panel/tick (`hit_mod=0`) and on impact triggers **2 follow-up hits** on that panel (+8t `hit_mod=0`, +8t `hit_mod=3`, SimpleEffect `0x16`), dealing **3 hits per trigger pull** (`40x3` / `60x3` / `80x3`).
    - `Z-Punch/Strght/Impact` (`0x143..0x145`) → real `GutPunch` (`0x2F`), `GutStrgt` (`0x30`), `GutImpct` (`0x31`) (preserves RockCube pushing!).
    - `Z-Varibl` (`0x146`) → real `VarSwrd` (`0x26`) (**full command inputs work on every swing!**).
    - `Z-Yoyo1..3` (`0x147..0x149`) → real `Yo-Yo1..3` (`0x45..0x47`) (physical 3-hit boomerang projectiles!).
    - `Z-Step1..2` (`0x14A..0x14B`) → real `StepSwrd` (`0x27`), `StepCros` (`0x28`) (real 2-panel step-forward slashes!).
- **Saito diff (`pa/init.luau` lines 162–640)**:
  - Sets `using_action = true` for the whole 180-tick duration (freezing MegaMan in place!), uses `flash` i-frames + `invisible = true` instead of Invis (`0x91`), caps shot counts (`6 / 9 / 5`) with fake auto-fire (`held("a") or timer % cd == 0`), and substitutes simplified fake hitboxes for every family (breaking `Z-Varibl` command inputs, `Z-Yoyo` boomerangs, `Z-Step` stepping, `Z-Punch` obstacle pushing, and `Z-Canon` 3-hit behavior).

##### 2. Non-Dimming PAs Wrongly Implemented as Dimming (5 chips — All P0)
In `bn3.gba`, `ChipData[+0x0B]` has bit `0x04` clear for ALL 9 non-dimming PAs (`0x14D..0x152`, `0x154`, `0x155`, `0x159`). Saito erroneously marks **5** of them as `flags = { "dimming", ... }`:
1. **`H-Burst` (`0x14D`)**: Real-time buster shot (`0x080B24B0`, SFX `0x7D`) travelling down MegaMan's row; 5 alternating non-flinching (`hit_mod=0`) explosions on target impact.
2. **`EvilCut` (`0x152`)**: Real-time 2-panel step-forward (`0x080B373C`) with panel validation (`CheckPanelFlags(0x10000, 0x38000000)`), 3 distinct slash patterns (`0x04` Wide, `0x06` Cross, `0x12+0x13` wide/extended with `0c-35`), and warp back.
3. **`HyperRat` (`0x154`)**: Real-time physical Ratton missile (`0x080B2BE4`, `AttackObj[0x23]`, `0c-1F` pal 1, `4 px/tick`, single homing turn, non-piercing 500 dmg `hit_mod=2`, launch SFX `0xB0`).
4. **`EverCrse` (`0x155`)**: Real-time CrsShld counter stance (`0x080B12D4`, `0c-03` anim 6 pal 0 for 26t) spawning a 100-dmg bite (`0c-03` anim 9 pal 2, `0x111`) and persistent 600-tick counter aura (`AttackObj[0x71]` `0x080E4D68`) when hit.
5. **`500Barr` (`0x159`)**: Real-time instant 500-HP barrier (`0x080B117C`, `0c-04` anim 4 **pal `3`**, SFX `0xCE`).

##### 3. Dimming PAs & Missing `CheckAntiNaviTrigger` (`0x08008EA0`)
- **6 Navi-summoning PAs** check `CheckAntiNaviTrigger` (`0x08008EA0`) in ROM and are counterable/stealable by AntiNavi: **`2xHero` (`0x153`), `BigHeart` (`0x15A`), `GtsShoot` (`0x15B`), `DeuxHero` (`0x15C`), `PrixPowr` (`0x15E`), `MstrStyl` (`0x15F`)**. None of them check AntiNavi in Saito.
- **Persistent/destructible field objects missing**:
  - `TimeBom+` (`0x14C`): 200-HP destructible obstacle (`0c-39`) on random valid enemy panel, 210t countdown.
  - `PoisPhar` (`0x157`): 300-HP destructible obstacle (`0c-29` anim 1), 6000t lifetime, real enemy-area `elem=5` drain hitbox at 1 dmg/tick (60 HP/s).
  - `MomQuake` (`0x15D`): Undims after 30t, spawns 480-tick real-time earthquake controller (`AttackObj[0xAF]`) dropping ~19 falling 200-dmg breaking boulders (`10-35`, no panel cracking).
- **Major damage / hit-count / heal / element discrepancies**:
  - `DeuxHero` (`0x15C`): Saito deals **8×70 = 560** vs ROM **7×70 = 490** (`0x080CF774[0]=7`).
  - `BodyGrd` (`0x158`): Saito throws **12×100 = 1200** vs ROM **18×100 = 1800** (`0x080F0C68`, `[r5+0x22]=19`).
  - `BigHeart` (`0x15A`): Saito deals **6×50 = 300** piercing and heals **full `max_hp` BEFORE attacking**, vs ROM **9×50 = 450** breaking and heals **300 HP AFTER attacking** (`0x080B6F0C[5]=150` → `×2 = 300` at `0x080D8252` → `0x080ADA0C`).
  - `GelRain` (`0x156`): Saito deals **3×150** wide + 1-column steal vs ROM **6×150** individual thrown-breaking MetaGels stealing up to 2 columns.
  - `PrixPowr` (`0x15E`): Saito deals **3×300** `piercing_break` blasts vs ROM BowlMan rolling **6×300** breaking bowling projectiles (`10-44`) across rows `y-1, y, y+1` twice.
  - `MstrStyl` (`0x15F`): Saito deals **6×100 = 600** with Fire/Aqua/Elec/Wood elements on hits 1–4 vs ROM **9×100 = 900 ALL Null element** (8 Style-clone hits + 9th full-field piercing+breaking blast).

##### 4. PA Recipe Table Audit (`0x0800D564..0x0800D758` vs `pa/init.luau`)
Decoded directly from the binary-search tables read by `0x08007A50` / `0x08007AE8`:

| PA | ROM Recipe Table (`bn3.gba`) | Saito (`pa/init.luau`) | Diff |
| :--- | :--- | :--- | :--- |
| **`BubSprd` (`0x14E`)** | `0x0800D57C[0]`: `Bubbler + Bub-V + BublSide` **AND** `0x0800D680[12..14]`: `Bubbler C/D/E`, `Bub-V D/E/F`, `BublSide E/F/G` | Only `sequence = { bubbler, bub-v, bublside }` | **P1: Missing 3 code-run recipes** |
| **`HeatSprd` (`0x14F`)** | `0x0800D57C[1]`: `HeatShot + Heat-V + HeatSide` **AND** `0x0800D680[15..17]`: `HeatShot H/I/J`, `Heat-V I/J/K`, `HeatSide J/K/L` | Only `sequence = { heatshot, heat-v, heatside }` | **P1: Missing 3 code-run recipes** |
| **`GelRain` (`0x156`)** | `0x0800D680[48..50]` ONLY: `MetaGel1 B/C/D`, `MetaGel2 E/F/G`, `MetaGel3 S/T/U` (NO sequence recipe!) | 3 `code_run` entries + extra `sequence = { metagel1, metagel2, metagel3 }` (order 23) | **P1: Non-ROM sequence recipe** |
| **`PrixPowr` (`0x15E`)** | `0x0800D57C[15..17]` ONLY: `Team1 + Team2 + KingManV5 / MistManV5 / BowlManV5` | Adds 4 extra recipes accepting `KingMan V1..V4` (orders 45–48) | **P1: 4 non-ROM V1–V4 recipes** |
| **All code-run PAs** | `0x0800D680` restricts each chip to specific 3-letter runs (e.g., `Cannon` A/B/C, B/C/D, C/D/E; `TimeBomb` J/K/L, K/L/M, L/M/N; max one `*` wildcard enforced at `0x08007BA4`) | `code_run = { chip = ..., count = 3 }` accepts any 3 consecutive codes | **Note**: Check PA matcher in rules if specific letter-run filtering & single-`*` limit are enforced. |
| **5-chip ROM table curiosities** | `0x0800D674[0]`: `Cannon + HolyPanl + Roll + Roll + Roll → BigHeart`; `0x0800D754[0]`: `Cannon A/B/C/D/E → Z-Canon1` | Not present | *(Historical/debug 5-chip entries in BN3 ROM; informational.)* |

##### 5. Missing / Misconfigured Assets in `compat/assets.toml`
Several ROM sprites required by PAs are either unused or missing from `pa/init.luau`:
- `0c-39` (`TimeBom+` obstacle — currently using `0c-02` `bomb`)
- `0c-29` anim `1` (`PoisPhar` statue — currently using `0c-2B` `anubis`)
- `0c-1F` pal `1` (`HyperRat` — currently using `0c-20` via `RATTON_FX`)
- `0c-03` (`EverCrse` shield anim 6 pal 0 & bite anim 9 pal 2)
- `0c-35` (`EvilCut` 3rd slash)
- `04-1C` (`GelRain` falling MetaGel, anim 2→3, random pal 0..3)
- `0c-04` pal `3` (`500Barr` — `lib/barriers.luau` uses `barrier_200` pal `2`)
- `08-08` (GutsMan), `08-09` (ProtoMan), `10-14` (Roll), `08-0E` (BowlMan), `10-44` (BowlMan bowling pins/ball), `10-35` (MomQuake falling boulders).
