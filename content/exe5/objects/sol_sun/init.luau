--!strict
-- EXE5DS Sol Cross corner Sun object (0x02098B54, 0x02200A4C..0x02200B64): spawned at round start when Sol Cross
-- MegaMan fights in sunlight (`hub_style == 3 and stats.sun`). While unpaused, undimmed, and MegaMan is the active
-- navi (in Base Sol Cross OR any Double Soul; pauses while switched to a Team Navi), ticks down from 5 (starting at 0)
-- by 1 per frame (or by 2 when Otenko is active on the same side) and grants +1 Attack bonus to the current front
-- damaging hand chip, up to +20 (`0x14`) per chip instance.

local chip_hand = require("@exelib/chip_hand")
local megaman = require("../navis/megaman").megaman
local otenko = require("../chips/otenko/identity")

local SPRITE = asset.sprite("sol-sun")
local EVERY = 5
local MAX_BONUS = 20

type State = {
    entry: number,
    turn: number,
    count: number,
}

local function has_otenko(side: number): boolean
    for _, o in field.objects(side) do
        if o.identity == otenko then
            return true
        end
    end
    return false
end

local function tick(me: Object)
    if battle.paused() or battle.dimmed() or battle.over() then
        return
    end
    local side = me.alliance
    if battle.navi(side).navi ~= megaman then
        return
    end
    local dec = if has_otenko(side) then 2 else 1
    local t = me.timer - dec
    if t > 0 then
        me.timer = t
        return
    end
    me.timer = EVERY
    local i = battle.hand_cursor(side)
    if not chip_hand.damages(side, i) then
        return
    end
    local turn = battle.hand_entry(side, i, "turn")
    local s = me.state :: State
    local same = i == s.entry and turn == s.turn
    s.entry, s.turn = i, turn
    if not same then
        s.count = 0
    elseif s.count >= MAX_BONUS then
        return
    end
    chip_hand.add_bonus(side, i, "attack_bonus", 1)
    s.count += 1
end

local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- 0x02200A64 (`bl 0x2001710`): part 0 is the top-right 32x32 tile of `shl00.bin`, not a ground shadow.
    sprite.shadow = "with_sprite"
    -- 0x02200A68 (`bl 0x2001878`): screen-space corner sprite (fixed during Custom Screen slide & shakes).
    sprite.screen_space = true
    -- 0x02200A74..0x02200AA0: local side's sun sits at top-right (screen x=240, y=0, unflipped);
    -- opponent's sun sits at top-left (screen x=0, y=0, horizontally flipped). Because the renderer
    -- mirrors `pos.x` and XORs `hflip` by `view.mirror`, side 0 (+120, hflip=false) and side 1
    -- (-120, hflip=true) project to each console's own top-right and opponent's top-left identically.
    local is_right = me.alliance == 1
    sprite.hflip = is_right
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me.visible = true
    me.pos = Vec3.px(if is_right then -120 else 120, -80, 0)
    me.timer = 0
    local s = me.state :: State
    s.entry, s.turn, s.count = 0xFF, 0, 0
    me:set_lifecycle("update")
end

local sol_sun = {}

sol_sun.kind = new.kind {
    id = "sol-sun",
    pool = "effect",
    state = { entry = "u8", turn = "u32", count = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

function sol_sun.spawn(side: number): Object?
    local o = battle.spawn(sol_sun.kind, Vec3.zero)
    if o then
        o.alliance = side
        o.flip = 0
        -- 0x02200B7E..0x02200B84 (`flags |= 0x14`): initialize and animate through Intro / Custom Screen pause.
        o.run_while_paused = true
    end
    return o
end

return sol_sun
