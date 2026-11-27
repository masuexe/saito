--!strict
-- EXE3's guard shield visual (Shield & Reflect B+Left specials).

local common = require("@exelib/rflectr/shield")

local shield = {}

local SPRITE = asset.sprite("reflector-shield")
local FADE_TICKS = 14

local made = common.make {
    id = "guard-shield",
    sound = asset.sound("shield"),
    animates_while_dimmed = true,
    runs_while_paused = false,
    hides_with_vanished_owner = false,
    side_only = true,
}

shield.kind = made.kind

function shield.look(spec: { anim: number }): ReflectorShieldLook
    local anim = spec.anim
    return common.look { sprite = SPRITE, anim = anim, palette = 0, fade_anim = anim, fade_ticks = FADE_TICKS }
end

function shield.spawn(owner: Object, offset: Vec3, look: ReflectorShieldLook): Object?
    return made.spawn(owner, offset, look)
end

return shield
