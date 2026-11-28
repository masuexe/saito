--!strict
-- EXE3 Plus & Utility chips:
-- Atk+10 (0x0C3), Atk+30 (0x0CD), Fire+30 (0x0C4), Aqua+30 (0x0C5), Elec+30 (0x0C6), Wood+30 (0x0C7),
-- Navi+20 (0x0C8), Navi+40 (0x0CE), AirShoes (0x0A8), FullCust (0x0D3).

local dimming_chips = require("@exelib/dimming")
local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local GAUGE_FULL_SOUND = asset.sound("gauge-full")
local BONUS_SOUND = asset.sound("damage-bonus")
local HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }

-- A6BE Family 45 (0x080B3A10, b11=0x02): FullCust is an instant non-dimming action.
local FULLCUST_ACTION = new.action {
    id = "plus/fullcust-action",
    update = function(me: Object)
        battle.fill_custom_gauge()
        battle.play_sound(GAUGE_FULL_SOUND)
        me:exit_attack()
    end,
}

local AIRSHOES_KIND = new.kind {
    id = "plus/airshoes-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "AirShoes",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                if user then
                    user:set_status("air_shoe", true)
                    battle.navi(user.alliance).air_shoes = true
                    battle.effect(user.pos, HEAL_FX)
                end
                battle.play_sound(BONUS_SOUND)
                me.timer = 20
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local atk_10: Chip = {
    modifier = "attack_plus",
    action = spawners.action { id = "atk-10/action", spawn = plus.spawner(10) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local atk_30: Chip = {
    modifier = "attack_plus",
    action = spawners.action { id = "atk-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    auto_battle = { class = "at_once" },
}

local fire_30: Chip = {
    modifier = "fire_plus",
    action = spawners.action { id = "fire-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local aqua_30: Chip = {
    modifier = "aqua_plus",
    action = spawners.action { id = "aqua-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 21,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local elec_30: Chip = {
    modifier = "elec_plus",
    action = spawners.action { id = "elec-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 23,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local wood_30: Chip = {
    modifier = "wood_plus",
    action = spawners.action { id = "wood-30/action", spawn = plus.spawner(30) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local navi_20: Chip = {
    modifier = "navi_plus",
    action = spawners.action { id = "navi-20/action", spawn = plus.spawner(20) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 34,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "at_once" },
}

local navi_40: Chip = {
    modifier = "navi_plus",
    action = spawners.action { id = "navi-40/action", spawn = plus.spawner(40) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 2,
    mb = 60,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    auto_battle = { class = "at_once" },
}

local airshoes: Chip = {
    codes = { "H", "I", "N", "U", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 26,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(AIRSHOES_KIND, user, spec, true)
    end,
    auto_battle = { class = "at_once" },
}

local fullcust: Chip = {
    action = FULLCUST_ACTION,
    codes = { "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    auto_battle = { class = "at_once" },
}

return {
    ["atk-10"] = atk_10,
    ["atk-30"] = atk_30,
    ["fire-30"] = fire_30,
    ["aqua-30"] = aqua_30,
    ["elec-30"] = elec_30,
    ["wood-30"] = wood_30,
    ["navi-20"] = navi_20,
    ["navi-40"] = navi_40,
    airshoes = airshoes,
    fullcust = fullcust,
}
