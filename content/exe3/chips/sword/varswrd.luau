--!strict
-- EXE3 VarSwrd helper: SonicBoom piercing wave, Elemental Sonic wave, FighterSword, LifeSword, and command inputs.

local anims = require("../../lib/anims")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sonic_boom = require("@exelib/swords/sonic_boom")
local sparks = require("../../lib/sparks")
local swords = require("../../lib/swords")
local varswrd_action = require("@exelib/varswrd/action")

local varswrd = {}

local SLASH_SPRITE = asset.sprite("sword-slash")
local BIG_SLASH_SPRITE = asset.sprite("big-slash")

local SWORD_HIT = { hits = collision.hits_navis, is = collision.slash }
local BIG_SLASH = new.effect { sprite = BIG_SLASH_SPRITE, anim = 0, palette = 0, steps = 15 }
local LINE_SLASH = new.effect { sprite = SLASH_SPRITE, anim = 3, palette = 0, steps = 15 }

local BOOM_KIND = sonic_boom.make_kind("varswrd/sonic-boom", {
    sprite = SLASH_SPRITE,
    z = 0x10_0000,
    piercing_palette = 0,
    element_mask = 0x7,
    palettes = { [0] = 0, [1] = 1, [2] = 2, [3] = 3, [4] = 4 },
    is = collision.slash,
    hits = collision.hits_navis,
})

type BoomState = { timer: number, thrown: number }

local function make_sonic_action(id: string, count: number, elements: { number }): Action
    return new.action {
        id = id,
        state = { timer = "u16", thrown = "u8" },
        update = function(me: Object, s: BoomState)
            if me.step_init == 0 then
                me:set_animation(anims.SWORD_SWING)
                me:set_status("using_action", true)
                s.timer = 0
                s.thrown = 0
                me.step_init = 4
            end
            if s.timer % 6 == 0 and s.thrown < count then
                s.thrown += 1
                local elem = elements[s.thrown] or 0
                local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.play_sound(swords.swing)
                local x = me.panel_x + me.facing
                if field.valid(x, me.panel_y) then
                    sonic_boom.throw(BOOM_KIND, me, x, me.panel_y, elem, dmg, {
                        hit_mod = if s.thrown == count then 3 else 1,
                        pierces = true,
                    })
                end
            end
            s.timer += 1
            if s.timer >= count * 6 + 10 then
                me:exit_attack()
            end
        end,
    }
end

-- Sub-chips used internally by VarSwrd's command inputs (not in library).
local ftrsword_pick: Chip = {
    id = "varswrd/ftrsword",
    action = swords.slash {
        id = "varswrd/ftrsword-action",
        blade = swords.blade,
        hit = { region = regions.line, collision = SWORD_HIT, hit_mod = 3 },
        effect = LINE_SLASH,
        sound = swords.swing,
    },
    damage = 160,
    element = "null",
    family = "sword",
}

local lifeswrd_pick: Chip = {
    id = "varswrd/lifeswrd",
    action = swords.slash {
        id = "varswrd/lifeswrd-action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = SWORD_HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
    damage = 160,
    element = "null",
    family = "sword",
}

local sonicbom_pick: Chip = {
    id = "varswrd/sonicbom",
    action = make_sonic_action("varswrd/sonicbom-action", 3, { 0, 0, 0 }),
    damage = 160,
    element = "null",
    family = "sword",
}

local elemsonic_pick: Chip = {
    id = "varswrd/elemsonic",
    action = make_sonic_action("varswrd/elemsonic-action", 4, { 1, 2, 3, 4 }),
    damage = 160,
    element = "null",
    family = "sword",
}

local GAME: VarSwordGame = {
    sound = asset.sound("damage-bonus"),
    waits = function(_me: Object): boolean
        return false
    end,
}

-- Build VarSwrd's action with both facing-right ([0]) and facing-left ([1]) command sequences:
--   1. LongSword:  Down -> Down+Fwd -> Fwd
--   2. FighterSwd: Back -> Down+Back -> Down -> Down+Fwd -> Fwd
--   3. WideSword:  Up -> Fwd -> Down
--   4. LifeSword:  Down -> Back -> Up -> Fwd -> Down
--   5. SonicBoom:  Back -> B -> Fwd -> B  (and Back -> Down -> Fwd -> B gives Elemental Sonic!)
-- Joypad bits: A=0x001, B=0x002, Right=0x010, Left=0x020, Up=0x040, Down=0x080.
function varswrd.make_action(sword: Chip, wideswrd: Chip, longswrd: Chip): Action
    -- Note: @exelib/varswrd/action allocates progress = "u8[5]", supporting up to 5 sequences!
    return varswrd_action.make({
        id = "varswrd/action",
        sequences = {
            -- Facing right (forward = Right 0x10, back = Left 0x20)
            [0] = {
                { 0x81, 0x91, 0x11, 0 },                   -- LongSwrd: Down, Down+Right, Right
                { 0x21, 0xA1, 0x81, 0x91, 0x11, 0 },       -- FighterSwd: Left, Down+Left, Down, Down+Right, Right
                { 0x41, 0x11, 0x81, 0 },                   -- WideSwrd: Up, Right, Down
                { 0x81, 0x21, 0x41, 0x11, 0x81, 0 },       -- LifeSwrd: Down, Left, Up, Right, Down
                { 0x21, 0x03, 0x11, 0x03, 0 },             -- SonicBoom (ElemSonic): Left, A+B, Right, A+B
            },
            -- Facing left (forward = Left 0x20, back = Right 0x10)
            [1] = {
                { 0x81, 0xA1, 0x21, 0 },                   -- LongSwrd: Down, Down+Left, Left
                { 0x11, 0x91, 0x81, 0xA1, 0x21, 0 },       -- FighterSwd: Right, Down+Right, Down, Down+Left, Left
                { 0x41, 0x21, 0x81, 0 },                   -- WideSwrd: Up, Left, Down
                { 0x81, 0x11, 0x41, 0x21, 0x81, 0 },       -- LifeSwrd: Down, Right, Up, Left, Down
                { 0x11, 0x03, 0x21, 0x03, 0 },             -- SonicBoom (ElemSonic): Right, A+B, Left, A+B
            },
        },
        picks = { longswrd, ftrsword_pick, wideswrd, lifeswrd_pick, elemsonic_pick },
        sword = sword,
    }, GAME)
end

varswrd.sonicbom_pick = sonicbom_pick
varswrd.sparks = sparks

return varswrd
