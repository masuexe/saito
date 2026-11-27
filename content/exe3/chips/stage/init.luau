--!strict
-- EXE3 Stage & Panel Control chips: HolyPanl, LavaStge, IceStage, GrassStg, SandStge, MetlStge, Snctuary.

local panel_changer = require("@exelib/panel_changer")
local panel_chips = require("@exelib/panel_chips/controller")

local CHANGER = panel_changer.make_kind("stage/panel-changer")
local CONTROLLER = panel_chips.make { id = "stage/controller", changer = CHANGER, side_only = true }

local SOUND = asset.sound("appear")
local TICK_SOUND = asset.sound("panel-crack")

local function stage_change(panel: PanelType, sides: { [number]: PanelCondition }, scope: "all" | "front"): PanelChange
    return panel_changer.change {
        panels = scope,
        panel = panel,
        ticks = 30,
        flicker = true,
        sides = sides,
        tick_sound = TICK_SOUND,
        change_sound = SOUND,
    }
end

local holypanl: Chip = {
    codes = { "E", "J", "L", "R", "U", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("holy", panel_chips.ANY, "front")),
}

local lavastge: Chip = {
    codes = { "A", "E", "R", "T", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("lava", panel_chips.ANY, "all")),
}

local icestage: Chip = {
    codes = { "C", "G", "M", "Q", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("ice", panel_chips.ANY, "all")),
}

local grassstg: Chip = {
    codes = { "E", "J", "R", "W", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("grass", panel_chips.ANY, "all")),
}

local sandstge: Chip = {
    codes = { "B", "C", "Q", "U", "W", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("sand", panel_chips.ANY, "all")),
}

local metlstge: Chip = {
    codes = { "D", "G", "M", "O", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 26,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("metal", panel_chips.ANY, "all")),
}

local snctuary: Chip = {
    codes = { "A", "C", "E", "L", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 54,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.hook(stage_change("holy", panel_chips.OWN_AREA, "all")),
}

return {
    holypanl = holypanl,
    lavastge = lavastge,
    icestage = icestage,
    grassstg = grassstg,
    sandstge = sandstge,
    metlstge = metlstge,
    snctuary = snctuary,
}
