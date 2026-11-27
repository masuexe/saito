--!strict
-- EXE3 rules internal API: exposes controlled access to per-side rules state and setup
-- for chips and weapons (AntiDmg stance, traps, supports, Shadow immunity, FoldrBak, NavRcycl).

local api = {}

export type FolderEntry = { chip: Chip, code: string }
export type TrapKind = "none" | "anti_damage" | "anti_sword" | "anti_navi" | "anti_recovery"

export type RulesState = {
    bug_counts: { [string]: number },
    bug_stopped: boolean,
    bug_base_atk: number,
    bug_base_rapid: number,
    bug_base_charge: number,
    bug_base_blanks: number,
    bug_base_move: number,
    bug_base_hp_drain: number,
    bug_base_custom_drain: number,
    bug_base_panel_trail: number,
    bug_hand_delta: number,
    rush_armed: boolean,
    beat_armed: boolean,
    tango_armed: boolean,
    rush_used: boolean,
    beat_used: boolean,
    tango_used: boolean,
    anti_dmg_stance: number,
    trap_kind: TrapKind,
    trap_damage: number,
    shadow_ticks: number,
    last_navi_chip: Chip?,
}

function api.state(side: number): RulesState
    return rules.state_of(side) :: RulesState
end

function api.arm_anti_dmg_stance(side: number, ticks: number)
    local s = api.state(side)
    s.anti_dmg_stance = ticks
end

function api.clear_anti_dmg_stance(side: number)
    local s = api.state(side)
    s.anti_dmg_stance = 0
end

function api.arm_trap(side: number, kind: TrapKind, damage: number)
    local s = api.state(side)
    s.trap_kind = kind
    s.trap_damage = math.clamp(damage, 0, 0xFFFF)
end

function api.clear_trap(side: number)
    local s = api.state(side)
    s.trap_kind = "none"
    s.trap_damage = 0
end

function api.set_shadow_ticks(side: number, ticks: number)
    local s = api.state(side)
    s.shadow_ticks = ticks
end

function api.last_navi_chip(side: number): Chip?
    return api.state(side).last_navi_chip
end

function api.set_last_navi_chip(side: number, chip: Chip)
    api.state(side).last_navi_chip = chip
end

-- Restores every chip from side's setup folder back into the battle folder (FoldrBak Giga chip).
function api.restore_folder(side: number)
    local setup = rules.setup_of(side)
    local folder = setup.folder :: { FolderEntry }
    local count = math.min(#folder, 30)
    for i = 1, count do
        local entry = folder[i]
        if entry and entry.chip then
            custom.set_folder(side, i, entry.chip, entry.code)
        end
    end
end

return api
