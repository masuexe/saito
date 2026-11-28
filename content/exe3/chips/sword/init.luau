--!strict
-- EXE3 Swords: Sword, WideSwrd, LongSwrd, FireSwrd, AquaSwrd, ElecSwrd, BambSwrd,
-- CustSwrd, VarSwrd, StepSwrd, StepCros, Slasher, Muramasa, HeroSwrd.

local anims = require("../lib/anims")
local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")
local varswrd_lib = require("@self/varswrd")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local SLASH = asset.sprite("sword-slash")
local BIG_SLASH_SPRITE = asset.sprite("big-slash")

-- EXE3 `0c-0c` (`sword-slash`) animations & palettes verified against A6BE (`bn3.gba` `0x080D65CC`):
--   anim 0 = WideSwrd (1x3, 40x76), anim 1 = LongSwrd (2x1, 72x42),
--   anim 2 = Sword (1x1, 40x24),    anim 3 = FighterSwd/Muramasa (3x1, 120x50),
--   anim 4 = HeroSwrd (3x1, 121x56).
--   pal 0 = white-blue, pal 2 = gold (HeroSwrd), pal 3 = pink-purple (Muramasa),
--   pal 5 = fire orange, pal 6 = aqua ice-blue, pal 7 = elec yellow, pal 8 = wood green.
local SWORD_SLASH = new.effect { sprite = SLASH, anim = 2, palette = 0, steps = 12 }
local WIDE_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 0, steps = 15 }
local LONG_SLASH = new.effect { sprite = SLASH, anim = 1, palette = 0, steps = 15 }
local HERO_SLASH = new.effect { sprite = SLASH, anim = 4, palette = 2, steps = 15 }
local MURAMASA_SLASH = new.effect { sprite = SLASH, anim = 3, palette = 3, steps = 15 }
local FIRE_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 5, steps = 15 }
local AQUA_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 6, steps = 15 }
local ELEC_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 7, steps = 15 }
local WOOD_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 8, steps = 15 }
local BIG_SLASH = new.effect { sprite = BIG_SLASH_SPRITE, anim = 0, palette = 0, steps = 15 }
local CUST_SLASH = new.effect { sprite = BIG_SLASH_SPRITE, anim = 0, palette = 1, steps = 15 }

local function slash(id: string, region: Region, effect: Effect, sound: Sound?, blade: AttachmentLook?, hit_mod: number?): Action
    return swords.slash {
        id = id,
        blade = blade or swords.blade,
        hit = { region = region, collision = HIT, hit_mod = hit_mod or 3 },
        effect = effect,
        sound = sound or swords.swing,
    }
end

local sword: Chip = {
    action = slash("sword/action", regions.single, SWORD_SLASH),
    codes = { "E", "H", "L", "S", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local wideswrd: Chip = {
    action = slash("wideswrd/action", regions.wide, WIDE_SLASH),
    codes = { "C", "E", "L", "Q", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local longswrd: Chip = {
    action = slash("longswrd/action", regions.long, LONG_SLASH),
    codes = { "E", "I", "L", "R", "Y" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local fireswrd: Chip = {
    action = slash("fireswrd/action", regions.wide, FIRE_SLASH, nil, swords.fire_blade),
    codes = { "F", "N", "P", "R", "U" },
    element = "fire",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local aquaswrd: Chip = {
    action = slash("aquaswrd/action", regions.wide, AQUA_SLASH, nil, swords.aqua_blade),
    codes = { "A", "H", "N", "P", "T" },
    element = "aqua",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elecswrd: Chip = {
    action = slash("elecswrd/action", regions.wide, ELEC_SLASH, nil, swords.elec_blade),
    codes = { "E", "K", "N", "P", "V" },
    element = "elec",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bambswrd: Chip = {
    action = slash("bambswrd/action", regions.wide, WOOD_SLASH, nil, swords.wood_blade, 1),
    codes = { "B", "L", "N", "P", "W" },
    element = "wood",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 140,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

-- CustSwrd (0x025): 3x2 blade whose damage scales with the Custom Gauge!
local custswrd: Chip = {
    damage = { formula = "gauge", full = 0 },
    action = slash("custswrd/action", regions.wide_long, CUST_SLASH, swords.big_swing),
    codes = { "B", "F", "R", "V", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 21,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
}

-- VarSwrd (0x026): Variable Sword with classic command inputs!
local varswrd: Chip = {
    action = varswrd_lib.make_action(sword, wideswrd, longswrd),
    codes = { "B", "C", "D", "E", "F" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 68,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local stepswrd: Chip = {
    action = swords.step_slash {
        id = "stepswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide, collision = HIT, hit_mod = 3 },
        effect = WIDE_SLASH,
        sound = swords.swing,
    },
    codes = { "L", "M", "N", "O", "P" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 43,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

-- StepCros (0x028): Steps 2 panels forward and slashes an X pattern!
local stepcros: Chip = {
    action = swords.step_slash {
        id = "stepcros/action",
        blade = swords.blade,
        hit = { region = regions.x, collision = HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
    codes = { "P", "Q", "R", "S", "T" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 74,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

-- Slasher (0x02B): Hold A to wait; slashes any enemy entering user's own area for 240!
type SlasherState = { timer: number, target_x: number }
local SLASHER_START, SLASHER_WAIT, SLASHER_HIT, SLASHER_RECOVER = 0x0, 0x4, 0x8, 0xC
local SLASHER_SEARCH = {
    [0] = { require = 0x00200000, forbid = 0x00000020 },
    [1] = { require = 0x00400020, forbid = 0x00000000 },
}

local function find_intruder_col(me: Object): number
    local s = SLASHER_SEARCH[me.alliance]
    local x = 1 + 5 * me.alliance
    while x >= 1 and x <= 6 do
        for y = 3, 1, -1 do
            if field.check(x, y, s.require, s.forbid) then
                return x
            end
        end
        x += me.facing
    end
    return 0
end

local SLASHER_ACTION = new.action {
    id = "slasher/action",
    state = { timer = "u16", target_x = "u8" },
    update = function(me: Object, s: SlasherState)
        local step = me.step
        if step == SLASHER_START then
            me:set_animation(anims.idle(me))
            local x = find_intruder_col(me)
            if x ~= 0 then
                s.target_x = x
                me.step, me.step_init = SLASHER_HIT, 0
            else
                me.step, me.step_init = SLASHER_WAIT, 0
            end
        elseif step == SLASHER_WAIT then
            if me:held("a") then
                local x = find_intruder_col(me)
                if x ~= 0 then
                    s.target_x = x
                    me.step, me.step_init = SLASHER_HIT, 0
                end
            else
                me:exit_attack()
            end
        elseif step == SLASHER_HIT then
            if me.step_init == 0 then
                me.anim = anims.SWORD_SWING
                battle.play_sound(swords.big_swing)
                attachment.attach(me, swords.blade, "related", 0)
                me:set_status("using_action", true)
                s.timer = 21
                local chip = me.attack_chip
                if chip ~= nil then
                    battle.show_used_chip(me.alliance, chip, me.attack_damage, me.extra)
                end
                me.step_init = 4
            end
            if s.timer == 12 then
                local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = s.target_x,
                    panel_y = 2,
                    element = me.attack_element,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = nil,
                    target = HIT.hits,
                    self_type = HIT.is,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = 3,
                })
                local cx, cy = field.center(s.target_x, 2)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), WIDE_SLASH, me.alliance)
            end
            local t = s.timer - 1
            s.timer = int.u16(t)
            if t < 0 then
                me.step, me.step_init = SLASHER_RECOVER, 0
            end
        elseif step == SLASHER_RECOVER then
            if me.step_init == 0 then
                s.timer = 5
                me.step_init = 4
            end
            local t = s.timer - 1
            s.timer = int.u16(t)
            if t < 0 then
                me.related1 = nil
                me:exit_attack()
            end
        else
            me:exit_attack()
        end
    end,
}

local slasher: Chip = {
    action = SLASHER_ACTION,
    codes = { "B", "D", "G", "R", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 12,
    damage = 240,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    traits = { "not_named_on_use" },
}

-- Muramasa (0x0CA): Mega sword dealing damage equal to user's missing HP (cap 999), 2x1 LongSwrd (A6BE 0x080D65CC row 9)!
local muramasa: Chip = {
    damage = { formula = "hp_lost", cap = 999 },
    action = slash("muramasa/action", regions.long, MURAMASA_SLASH, swords.big_swing),
    codes = { "M" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 74,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
}

-- HeroSwrd (0x0CF): Mega sword cutting 3 panels ahead for 180!
local heroswrd: Chip = {
    action = slash("heroswrd/action", regions.line, HERO_SLASH, swords.big_swing),
    codes = { "P" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 65,
    damage = 180,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return {
    sword = sword,
    wideswrd = wideswrd,
    longswrd = longswrd,
    fireswrd = fireswrd,
    aquaswrd = aquaswrd,
    elecswrd = elecswrd,
    bambswrd = bambswrd,
    custswrd = custswrd,
    varswrd = varswrd,
    stepswrd = stepswrd,
    stepcros = stepcros,
    slasher = slasher,
    muramasa = muramasa,
    heroswrd = heroswrd,
}
