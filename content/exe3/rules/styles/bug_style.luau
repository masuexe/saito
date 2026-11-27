--!strict
-- EXE3 Bug Style per-turn custom and battle bug/perk rolls (deterministic via battle.rng_positive).
-- Suppressed automatically when BugStop is installed in the NaviCust.

local api = require("../api")
local barriers = require("../../lib/barriers")

local bug_style = {}

-- Snapshot baseline stats after NaviCust compilation so each turn's Bug Style roll starts clean.
function bug_style.snapshot(side: number, bug_stopped: boolean)
    local stats = battle.navi(side)
    local s = api.state(side)
    s.bug_stopped = bug_stopped
    s.bug_base_atk = stats.attack
    s.bug_base_rapid = stats.rapid
    s.bug_base_charge = stats.charge
    s.bug_base_blanks = stats.buster_blanks
    s.bug_base_move = stats.move_bug
    s.bug_base_hp_drain = stats.hp_drain
    s.bug_base_custom_drain = stats.custom_drain
    s.bug_base_panel_trail = stats.panel_trail
    s.bug_hand_delta = 0
end

local function restore_base(stats: Navi, side: number)
    local s = api.state(side)
    stats.attack = s.bug_base_atk
    stats.rapid = s.bug_base_rapid
    stats.charge = s.bug_base_charge
    stats.buster_blanks = s.bug_base_blanks
    stats.move_bug = s.bug_base_move
    stats.hp_drain = s.bug_base_hp_drain
    stats.custom_drain = s.bug_base_custom_drain
    stats.panel_trail = s.bug_base_panel_trail
    s.bug_hand_delta = 0
end

-- Roll authentic EXE3 Bug Style custom + battle perks/bugs at round start and each custom screen return.
function bug_style.roll_turn(side: number)
    local stats = battle.navi(side)
    if not stats.bug_style then
        return
    end
    local s = api.state(side)
    restore_base(stats, side)

    -- BugStop negates all negative bugs from Bug Style (still grants positive perks on lucky rolls).
    local stopped = s.bug_stopped

    -- Custom-screen bug/perk roll (8 outcomes).
    local c_roll = battle.rng_positive() % 8
    if c_roll == 0 then
        -- Full Custom perk (+3 chips in custom screen)
        s.bug_hand_delta = 3
    elseif c_roll == 1 then
        s.bug_hand_delta = 2
    elseif c_roll == 2 then
        s.bug_hand_delta = 1
    elseif c_roll == 3 and not stopped then
        s.bug_hand_delta = -1
    elseif c_roll == 4 and not stopped then
        s.bug_hand_delta = -2
    elseif c_roll == 5 and not stopped then
        stats.custom_drain = 6
    elseif c_roll == 6 then
        -- Fast gauge perk for the turn
        battle.set_gauge_rate(0x200)
    end

    -- Battle bug/perk roll (8 outcomes).
    local b_roll = battle.rng_positive() % 8
    local navi = battle.player(side)
    if b_roll == 0 then
        -- Max Buster perk
        stats.attack = 4
        stats.rapid = 4
        stats.charge = 4
        stats.buster_blanks = 0
    elseif b_roll == 1 then
        -- Auto 100-HP Barrier perk
        if navi then
            barriers.raise(navi, barriers.barrier_100)
        end
    elseif b_roll == 2 then
        -- Brief opening Invis perk
        if navi then
            navi:set_status_timer("flash", 180)
            navi:set_status("invisible", true)
        end
    elseif b_roll == 3 and not stopped then
        -- Buster misfire bug
        stats.buster_blanks = math.max(stats.buster_blanks, 6)
    elseif b_roll == 4 and not stopped then
        -- Battle HP drain bug
        stats.hp_drain = 10
    elseif b_roll == 5 and not stopped then
        -- Movement slide bug
        stats.move_bug = 0x10
    elseif b_roll == 6 and not stopped then
        -- Cracked panel trail bug
        stats.panel_trail = 3
    elseif b_roll == 7 then
        -- Overclocked Buster (+2 Attack & Rapid)
        stats.attack = math.min(4, stats.attack + 2)
        stats.rapid = math.min(4, stats.rapid + 2)
    end
end

function bug_style.hand_delta(side: number): number
    local stats = battle.navi(side)
    if not stats.bug_style then
        return 0
    end
    return api.state(side).bug_hand_delta
end

return bug_style
