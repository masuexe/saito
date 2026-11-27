--!strict
-- EXE3's AirShot action wrapper over @exelib/airshot.

local attachment = require("../objects/attachment")
local common = require("@exelib/airshot")
local projectile = require("./projectile")
local air_shooter_sprite = asset.sprite("air-shooter")

local airshot = {}

airshot.shooter = attachment.look { sprite = air_shooter_sprite, palette = 0 }

local NAVI_NUMBER = 0

local LOOK: AirShotLook = {
    sound = asset.sound("spreader"),
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, NAVI_NUMBER)
    end,
    projectiles = projectile,
    anim = 12,
    slot = "related",
}

function airshot.action(spec: AirShotSpec): Action
    return common.action(spec, LOOK)
end

return airshot
