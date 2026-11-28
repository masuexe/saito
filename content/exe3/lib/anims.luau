--!strict
-- EXE3 Battle MegaMan (sprite 00-00, 63 anims) animation indices, verified against A6BE (`bn3.gba`):
--   0x00..=0x07 : Style idle (0=Normal, 1=Guts, 2=Custom, 3=Team, 4=Shield, 5=Ground, 6=Shadow, 7=Bug) [0x080B042A]
--   0x08        : Flinch / drag [0x080B09E2, 0x080B0B0A]
--   0x09        : Hitstun / paralyze / freeze / bubble / deletion [0x080B06A4, 0x080B073C]
--   0x0A        : Move arrive / warp in [0x080B0CEE]
--   0x0B        : Move depart / warp out [0x080B0C9A]
--   0x0C..=0x13 : Buster shoot WITH built-in muzzle flash (0x0C + style_index) [0x080B0432, 0x080B0F12]
--   0x14..=0x1B : Buster blank / no-flash shoot (0x14 + style_index) [0x080B0436, 0x080B0EFA]
--   0x1D        : Sword swing [0x080B174A]
--   0x20 / 0x21 : Cannon raise / lower [0x080B1218, 0x080B12B6]
--   0x22        : Bomb throw [0x080B16A2]
--   0x24        : PanlOut / panel crack [0x080B2308, 0x080B31EC]
--   0x2C        : GutsPunch / punch [0x080B2736]
--   0x36        : ShotGun / Spreader gun hold [0x080B1BC2]
--   0x38        : AirShot shooter [0x080B258A]
--   0x39        : Dash / lunge [0x080B2938]

local api = require("../rules/api")

local anims = {}

anims.STYLE_INDEX = {
    normal = 0,
    guts = 1,
    custom = 2,
    team = 3,
    shield = 4,
    ground = 5,
    shadow = 6,
    bug = 7,
}

anims.FLINCH = 0x08
anims.REACTION = 0x09
anims.MOVE_ARRIVE = 0x0A
anims.MOVE_DEPART = 0x0B
anims.SWORD_SWING = 0x1D
anims.CANNON_RAISE = 0x20
anims.CANNON_LOWER = 0x21
anims.BOMB_THROW = 0x22
anims.PANLOUT = 0x24
anims.PUNCH = 0x2C
anims.SPREADER = 0x36
anims.AIRSHOT = 0x38
anims.DASH = 0x39

function anims.style_index(me: Object): number
    return anims.STYLE_INDEX[api.style_type(me.alliance)] or 0
end

function anims.idle(me: Object): number
    return anims.style_index(me)
end

function anims.buster_shoot(me: Object): number
    return 0x0C + anims.style_index(me)
end

function anims.buster_blank(me: Object): number
    return 0x14 + anims.style_index(me)
end

return anims
