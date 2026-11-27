--!strict
-- EXE3's thrown bombs (MiniBomb, SnglBomb, DublBomb, TrplBomb, CannBall).

local attachment = require("../objects/attachment")
local bomb = require("@exelib/bombs/bomb")
local throw = require("@exelib/bombs/throw")

local bombs = {}

local SPRITE = asset.sprite("bomb")
local explosion_sprite = asset.sprite("explosion")
local puff_sprite = asset.sprite("puff")
local small_puff_sprite = asset.sprite("small-puff")

bombs.held = attachment.look { sprite = SPRITE, palette = 0 }
bombs.variant = bomb.variant

local made = bomb.make {
    id = "bomb",
    sprite = SPRITE,
    hit_sound = asset.sound("hit-bomb"),
    splash = new.effect { sprite = small_puff_sprite, anim = 0, palette = 0, steps = 16 },
    explosion = new.effect { sprite = explosion_sprite, anim = 0, palette = 0, steps = 27 },
    puff = new.effect { sprite = puff_sprite, anim = 1, palette = 0, steps = 4 },
}

bombs.thrower = made.thrower

local LOOK: ThrowLook = {
    sound = asset.sound("minibomb-throw"),
    attachment = attachment.kind,
    hold = function(me: Object, held: AttachmentLook, anim: number)
        attachment.attach(me, held, "related", anim)
    end,
    releases_by_side = true,
}

function bombs.throw(spec: ThrowSpec): Action
    return throw.action(spec, LOOK)
end

return bombs
