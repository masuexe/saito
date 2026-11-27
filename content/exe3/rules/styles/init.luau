--!strict
-- EXE3 Style Change pre-battle setup (save byte 0x1881: `(style_type << 3) | element`; NaviStatsInit 0x0804735E).
-- Applies Style form, element, elemental/Ground charge shot, and Style passive abilities before NaviCust compilation.

local barriers = require("../lib/barriers")
local base_form = require("../navis/megaman/forms/base").base
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local guards = require("../navis/megaman/weapons/guards")
local style_forms = require("../navis/megaman/forms/styles")

local styles = {}

local FORM_BY_TYPE: { [string]: FormDef } = {
    normal = base_form,
    guts = style_forms.guts,
    custom = style_forms.custom,
    team = style_forms.team,
    shield = style_forms.shield,
    ground = style_forms.ground,
    shadow = style_forms.shadow,
    bug = style_forms.bug,
}

-- Engine element numbers: 0=null, 1=fire(heat), 2=aqua, 3=elec, 4=wood.
local ELEMENT_ID: { [string]: number } = {
    null = 0,
    heat = 1,
    aqua = 2,
    elec = 3,
    wood = 4,
}

local ELEMENT_CHARGE: { [string]: Weapon } = {
    heat = charged_shot.heat,
    aqua = charged_shot.aqua,
    elec = charged_shot.elec,
    wood = charged_shot.wood,
}

-- Style-dependent NaviCust bonus color (on top of universal white, pink, yellow).
local STYLE_COLOR: { [string]: string? } = {
    normal = nil,
    guts = "red",
    custom = "blue",
    team = "green",
    shield = "blue",
    ground = "green",
    shadow = "red",
    bug = "dark",
}

function styles.allowed_color(style_type: string, color: string): boolean
    if color == "white" or color == "pink" or color == "yellow" or color == "orange" or color == "purple" then
        return true
    end
    return STYLE_COLOR[style_type] == color
end

function styles.round_setup(side: number)
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    local setup = rules.setup()
    local st: string = setup.style_type or "normal"
    local el: string = setup.style_element or "null"

    local form = FORM_BY_TYPE[st] or base_form
    stats.form = form
    stats.starting_form = form
    stats.element = ELEMENT_ID[el] or 0
    stats.buster_mult = 1

    local elem_charge = ELEMENT_CHARGE[el]
    if elem_charge ~= nil then
        stats.charge_shot_weapon = elem_charge
    elseif st == "ground" then
        stats.charge_shot_weapon = charged_shot.ground
    end

    if st == "guts" then
        stats.buster_mult = 2
        stats.super_armor = true
    elseif st == "custom" then
        stats.custom_level = math.min(stats.custom_level + 1, 8)
    elseif st == "team" then
        stats.mega_level = math.min(stats.mega_level + 1, 10)
    elseif st == "shield" then
        stats.first_barrier = barriers.first_barrier
        stats.back_special_weapon = guards.shield
    elseif st == "shadow" then
        stats.back_special_weapon = guards.anti_dmg
    elseif st == "bug" then
        stats.bug_style = true
    end
end

-- Battle MegaMan (00-00) palette indices (stride 3: +0 normal, +1 dimmed/no-charge, +2 white flash).
local ELEMENT_PALETTE: { [string]: number } = {
    elec = 3,
    heat = 6,
    aqua = 9,
    wood = 12,
}

local STYLE_PALETTE: { [string]: number } = {
    normal = 0,
    guts = 6,
    custom = 9,
    team = 27,
    shield = 24,
    ground = 18,
    shadow = 15,
    bug = 21,
}

function styles.navi_palette(side: number, _navi: Object): number?
    local setup = rules.setup_of(side)
    local st: string = setup.style_type or "normal"
    local el: string = setup.style_element or "null"
    if st == "normal" then
        return 0
    end
    return ELEMENT_PALETTE[el] or STYLE_PALETTE[st] or 0
end

return styles
