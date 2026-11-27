--!strict
-- EXE3 ShotGun, V-Gun, SideGun, and Spreader (compat/chips.toml IDs 0x00A..0x00D).

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local spreader = require("@self/spreader")
local pink_flash_sprite = asset.sprite("pink-flash")
local spreader_sprite = asset.sprite("spreader")

local GUN = attachment.look { sprite = spreader_sprite, palette = 0 }
local FLASH = new.effect { sprite = pink_flash_sprite, anim = 0, palette = 0, steps = 10 }
local SOUND = asset.sound("spreader")

local SHOTGUN_BURST = bullet.variant { region = regions.long, hit_mod = 0, effect = FLASH, sound = SOUND }
local VGUN_BURST = bullet.variant { region = regions.fork, hit_mod = 0, effect = FLASH, sound = SOUND }
local SIDEGUN_BURST = bullet.variant { region = regions.wide, hit_mod = 0, effect = FLASH, sound = SOUND }
local SPREADER_BURST = bullet.variant { region = regions.square, hit_mod = 0, effect = FLASH, sound = SOUND }

local shotgun: Chip = {
    action = spreader.action { id = "shotgun/action", gun = GUN, delay = 1, z = 0x10, shot = SHOTGUN_BURST },
    codes = { "B", "F", "J", "N", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local v_gun: Chip = {
    action = spreader.action { id = "v-gun/action", gun = GUN, delay = 1, z = 0x10, shot = VGUN_BURST },
    codes = { "D", "G", "L", "P", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local sidegun: Chip = {
    action = spreader.action { id = "sidegun/action", gun = GUN, delay = 1, z = 0x10, shot = SIDEGUN_BURST },
    codes = { "C", "H", "M", "S", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local spreader_chip: Chip = {
    action = spreader.action { id = "spreader/action", gun = GUN, delay = 1, z = 0x10, shot = SPREADER_BURST },
    codes = { "M", "N", "O", "P", "Q", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    shotgun = shotgun,
    ["v-gun"] = v_gun,
    sidegun = sidegun,
    spreader = spreader_chip,
}
