--!strict
-- The 5DS-only patch card Sol-Cross MegaMan (ソルクロスロックマン, Boktai 2/3 Slot-2 bonus), 70 MB.
-- Grants +200 HP, SuperArmor, MegaFolder -2, Sol GunDelSol charged shot, and Hub Style 3 (corner sun in sunlight).

local cards = require("../rules/patch_cards/cards")
local sol_gundelsol = require("../navis/megaman/weapons/sol_gundelsol")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.hp(200),
        cards.bug(cards.mega_folder(-2)),
        cards.super_armor(true),
        cards.charged_shot(sol_gundelsol.weapon),
        cards.hub_style(3),
    },
}

return { ["sol-cross-megaman"] = card }
