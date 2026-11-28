--!strict
-- EXE3 ShotGun, V-Gun, SideGun, Spreader, Bubbler/Bub-V/BublSide, HeatShot/Heat-V/HeatSide
-- (compat/chips.toml IDs 0x00A..0x013).

local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local spreader = require("@self/spreader")
local bubble_splash_sprite = asset.sprite("bubble-splash")
local heat_blast_sprite = asset.sprite("heat-blast")
local pink_flash_sprite = asset.sprite("pink-flash")

-- Verified against A6BE `0x080D5FAC` (`[0x0E, 0x0F, 0x10]` -> Effect #0 table `0x080EDBC0..0x080EDBC8`):
--   ShotGun/V-Gun/SideGun/Spreader: `14-04` anim 0 (green burst, 7f/12t)
--   Bubbler/Bub-V/BublSide:         `0c-16` anim 0 (bubble splash, 14f/26t)
--   HeatShot/Heat-V/HeatSide:       `0c-15` anim 0 (heat blast, 19f/36t)
local FLASH = new.effect { sprite = pink_flash_sprite, anim = 0, palette = 0, steps = 12 }
local AQUA_FLASH = new.effect { sprite = bubble_splash_sprite, anim = 0, palette = 0, steps = 26 }
local FIRE_FLASH = new.effect { sprite = heat_blast_sprite, anim = 0, palette = 0, steps = 36 }
local SOUND = asset.sound("spreader")
local AQUA_BURST_SOUND = asset.sound("freeze") -- A6BE 0x080D5FAF[1] = 0xAE
local FIRE_BURST_SOUND = asset.sound("heat-blast") -- A6BE 0x080D5FAF[2] = 0xA9

local SHOTGUN_BURST = bullet.variant { region = regions.long, hit_mod = 0, effect = FLASH, sound = SOUND }
local VGUN_BURST = bullet.variant { region = regions.fork, hit_mod = 0, effect = FLASH, sound = SOUND }
local SIDEGUN_BURST = bullet.variant { region = regions.wide, hit_mod = 0, effect = FLASH, sound = SOUND }
local SPREADER_BURST = bullet.variant { region = regions.square, hit_mod = 0, effect = FLASH, sound = SOUND }

-- A6BE 0x080D5FE2: Bubbler and HeatShot lines all use hit_mod = 0 (no flinch / no mercy flash).
local BUBBLER_BURST = bullet.variant { region = regions.long, hit_mod = 0, effect = AQUA_FLASH, sound = AQUA_BURST_SOUND }
local BUBV_BURST = bullet.variant { region = regions.fork, hit_mod = 0, effect = AQUA_FLASH, sound = AQUA_BURST_SOUND }
local BUBLSIDE_BURST = bullet.variant { region = regions.wide, hit_mod = 0, effect = AQUA_FLASH, sound = AQUA_BURST_SOUND }

local HEATSHOT_BURST = bullet.variant { region = regions.long, hit_mod = 0, effect = FIRE_FLASH, sound = FIRE_BURST_SOUND }
local HEATV_BURST = bullet.variant { region = regions.fork, hit_mod = 0, effect = FIRE_FLASH, sound = FIRE_BURST_SOUND }
local HEATSIDE_BURST = bullet.variant { region = regions.wide, hit_mod = 0, effect = FIRE_FLASH, sound = FIRE_BURST_SOUND }

local shotgun: Chip = {
    action = spreader.action { id = "shotgun/action", delay = 1, z = 0x00, shot = SHOTGUN_BURST },
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
    action = spreader.action { id = "v-gun/action", delay = 1, z = 0x00, shot = VGUN_BURST },
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
    action = spreader.action { id = "sidegun/action", delay = 1, z = 0x00, shot = SIDEGUN_BURST },
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
    action = spreader.action { id = "spreader/action", delay = 1, z = 0x00, shot = SPREADER_BURST },
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

local bubbler: Chip = {
    action = spreader.action { id = "bubbler/action", delay = 1, z = 0x00, shot = BUBBLER_BURST },
    codes = { "A", "C", "D", "E", "P", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bub_v: Chip = {
    action = spreader.action { id = "bub-v/action", delay = 1, z = 0x00, shot = BUBV_BURST },
    codes = { "D", "E", "F", "S", "V", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bublside: Chip = {
    action = spreader.action { id = "bublside/action", delay = 1, z = 0x00, shot = BUBLSIDE_BURST },
    codes = { "B", "E", "F", "G", "R", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local heatshot: Chip = {
    action = spreader.action { id = "heatshot/action", delay = 1, z = 0x00, shot = HEATSHOT_BURST },
    codes = { "B", "H", "I", "J", "P", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 16,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local heat_v: Chip = {
    action = spreader.action { id = "heat-v/action", delay = 1, z = 0x00, shot = HEATV_BURST },
    codes = { "F", "I", "J", "K", "V", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local heatside: Chip = {
    action = spreader.action { id = "heatside/action", delay = 1, z = 0x00, shot = HEATSIDE_BURST },
    codes = { "C", "J", "K", "L", "T", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    shotgun = shotgun,
    ["v-gun"] = v_gun,
    sidegun = sidegun,
    spreader = spreader_chip,
    bubbler = bubbler,
    ["bub-v"] = bub_v,
    bublside = bublside,
    heatshot = heatshot,
    ["heat-v"] = heat_v,
    heatside = heatside,
}
