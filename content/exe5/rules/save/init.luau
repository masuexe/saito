--!strict
-- What an EXE5 player's save brings to the navi's stats, which nothing else gives (docs/frontend.md §6): MegaMan's
-- base HP (HP Memories raise it, +0x3E), the Regular memory (RegUps, +0x09), fighting in the sun (+0x22: the
-- overworld's 0x080358DC, from EnterMap, sets MegaMan's to 1 on the maps it lists, 0x0803591C, and to 0 elsewhere;
-- the reset keeps it, 0x08133DBC; GunDelSol reads it), and a team navi's level (`level`: the count of the save's
-- story flags 0x300 to 0x305, 0 to the navi's story's last, which the init exchange sends; none for MegaMan, who
-- takes none). The engine knows no level: EXE5's rules read it (exe5.navi_level, ../api: the team navis' damage
-- rows, lib/navi_level). The rest of a round's stats are derived as the round is set up, from the navi's fresh stats
-- (`fresh_stats`, its definition): this module first, then the NaviCust's compile (../navicust) and the patch cards
-- (../patch_cards).
--
-- `round_setup` builds a side's stats only when its setup gives the rules something to build them from:
--
-- - a NaviCust to compile (MegaMan): the Regular memory, the sun, the base HP and MegaMan's variant, his base HP in
--   hundreds (+0x2B; the console sets it as the battle starts); the compile then makes the maximum the base and the
--   HP programs;
-- - a team navi (one with a `story`) at its level: the Regular memory, and its HP (current, maximum and base) as the
--   story leaves it at the level (the game's routine sets it from the table by the story's progress, at a new game's
--   start and from a map script as the story moves on: a level below the last is the progress, and at the last the
--   story is taken as done). The base HP and the sun stated here are MegaMan's alone (the overworld writes his
--   block's sun, a team navi's never).
--
-- Otherwise the stats are as the setup gives them, whole (a recording's MegaMan without a recorded NaviCust; a
-- test's).

local fresh_stats = require("./fresh_stats")

type Setup = { hp: number, reg_up: number, sun: boolean, level: number? }

-- A team navi's HP at `level` by its story (none past its last level).
local function story_hp(story: { hp: { number }, max_level: number }, level: number): number?
    if level < story.max_level then
        return story.hp[level + 1]
    elseif level == story.max_level then
        return story.hp[#story.hp]
    end
    return nil
end

-- Saito link-battle indoor flag (`setup::effects::NO_SUN = 0x0010_0000`): overrides default `sun = true` so
-- GunDelSol, Django, and Sol Cross run in shade/indoor mode.
local NO_SUN = 0x0010_0000

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = rules.setup() :: Setup
    local story = stats.navi.story
    local level = setup.level
    -- (The navi that changes form compiles a NaviCust, where the setup gives one: ../navicust.)
    local compiles = rules.setup_of(side).navicust_expansions ~= nil and stats.navi.forms ~= nil
    local team_navi = story ~= nil and level ~= nil
    if not compiles and not team_navi then
        return
    end
    stats.regular_memory = setup.reg_up
    if story ~= nil and level ~= nil then
        local hp = story_hp(story, level)
        if hp ~= nil then
            stats.max_base_hp = hp
            stats.max_hp = hp
            stats.hp = hp
        end
    else
        stats.sun = setup.sun and not (battle.link() and bit32.btest(battle.effects(), NO_SUN))
        stats.max_base_hp = setup.hp
        stats.navi_variant = setup.hp // 100
    end
end

-- What a save brings at its highest, which a setup that says nothing has (the rules' `setup_defaults`): the base
-- HP (HP Memories to their last: 1000, or 997 for a dark MegaMan who used at least 3 dark chips to reach karma < 470)
-- and the Regular memory (RegUps to theirs).
local MAX_HP = 1000
local DARK_MAX_HP = 997
local DARK_KARMA_BELOW = 470
local MAX_REG_UP = 50

-- `validate`'s part: the navi's level and the base HP as what the save brings allows: a team navi's level, its
-- story's (0 to its last), always stated (the tools state its last when a side states none); none for a navi without
-- a story. The base HP is MegaMan's (the navi that compiles a NaviCust): a team navi's HP is its level's, so a side of
-- one that states another than the default states what has no effect.
local function validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side)
    local navi: NaviDef? = setup.navi
    if navi == nil then
        return
    end
    local name = battle.name(navi)
    local level: number? = setup.level
    local story = navi.story
    -- (A party battle's level is MegaMan's side's: ../party's validate holds it to its range.)
    local in_party = #((setup :: any).party or {}) > 0
    if story ~= nil then
        if level == nil then
            problem({ kind = "missing", field = "level", text = string.format("%s has no level (0 to %d)", name, story.max_level) })
        elseif level > story.max_level then
            problem({
                kind = "range",
                field = "level",
                value = level,
                least = 0,
                most = story.max_level,
                text = string.format("level %d: %s's level is 0 to %d", level, name, story.max_level),
            })
        end
    elseif level ~= nil and not in_party then
        problem({ kind = "not_taken", field = "level", text = string.format("level %d: %s takes no level", level, name) })
    end
    if navi.forms == nil and setup.hp ~= MAX_HP then
        problem({
            kind = "not_taken",
            field = "hp",
            text = string.format("hp: %s's HP is its level's; a side states the base HP of the navi that compiles a NaviCust", name),
        })
    elseif navi.forms ~= nil and (setup.hp < 1 or setup.hp > MAX_HP) then
        problem({
            kind = "range",
            field = "hp",
            value = setup.hp,
            least = 1,
            most = MAX_HP,
            text = string.format("hp %d: MegaMan's HP is 1 to %d", setup.hp, MAX_HP),
        })
    end
end

-- `build_check`'s part: a dark MegaMan (karma < 470) must have used at least 3 dark chips, so base HP is at most 997.
local function build_check(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side)
    local navi: NaviDef? = setup.navi
    if navi == nil or navi.forms == nil then
        return
    end
    local karma: number = (setup :: any).karma or 500
    if karma < DARK_KARMA_BELOW and setup.hp > DARK_MAX_HP and setup.hp <= MAX_HP then
        problem({
            kind = "range",
            field = "hp",
            value = setup.hp,
            least = 1,
            most = DARK_MAX_HP,
            text = string.format("hp %d: dark MegaMan's HP is 1 to %d", setup.hp, DARK_MAX_HP),
        })
    end
end

return {
    round_setup = round_setup,
    validate = validate,
    build_check = build_check,
    MAX_HP = MAX_HP,
    MAX_REG_UP = MAX_REG_UP,
}
