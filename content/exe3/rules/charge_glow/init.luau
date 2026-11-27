--!strict
-- EXE3's charge glow effect over a charging navi.

local visibility = require("@exelib/visibility")

local glow = {}

local B_GLOW = asset.sprite("charge-glow")
local CHARGING = asset.sound("buster-charge")
local CHARGED = asset.sound("buster-charged")

export type State = {
    source: number,
    shown: boolean,
}

local function finish(me: Object, navi: Object)
    navi.charge_glow = nil
    me:free()
end

local function tick(me: Object)
    local navi = me.related1 :: Object
    local state = me.state :: State
    me.visible = true
    visibility.hide_from_blind(me)
    if not state.shown then
        me.visible = false
    end
    if navi.charge_source ~= state.source or navi.charge_level == 0 then
        return finish(me, navi)
    end
    local before = me.anim
    local anim = if navi.charge_level == 2 then 2 else 0
    me.anim = anim
    if anim ~= before and anim == 2 then
        battle.play_sound(CHARGED)
    end
    me.pos = navi.pos
    me:update_sprite()
end

local function init(me: Object)
    local navi = me.related1 :: Object
    local state = me.state :: State
    local source = navi.charge_source
    state.source = source
    if source == 0 then
        return finish(me, navi)
    end
    local sprite = me.sprite
    sprite:load(B_GLOW)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me.visible = true
    battle.play_sound(CHARGING)
    me:set_lifecycle("update")
    tick(me)
end

glow.kind = new.kind {
    id = "charge-glow",
    pool = "effect",
    state = { source = "u8", shown = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

return glow
