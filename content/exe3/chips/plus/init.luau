--!strict
-- EXE3 Plus & Utility chips: Atk+10 (0x0C3), Atk+30 (0x0CD), Navi+20 (0x0C8), Navi+40 (0x0CE), FullCust (0x0D3).

local dimming_chips = require("@exelib/dimming")
local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local GAUGE_FULL_SOUND = asset.sound("gauge-full")

local FULLCUST_KIND = new.kind {
    id = "plus/fullcust-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "FullCust",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.fill_custom_gauge()
                battle.play_sound(GAUGE_FULL_SOUND)
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

local fullcust: Chip = {
    codes = { "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(FULLCUST_KIND, user, spec, true)
    end,
    auto_battle = { class = "at_once" },
}

return {
    ["atk-10"] = atk_10,
    ["atk-30"] = atk_30,
    ["navi-20"] = navi_20,
    ["navi-40"] = navi_40,
    fullcust = fullcust,
}
