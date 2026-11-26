--!strict
-- EXE5's MegaMan's base form (`base`, the engine's base form of EXE5's game: rules-in-luau.md P1c): MegaMan in no
-- soul (NaviStats +0x2C 0), his sprite by his soul (0x0800DA58[0]). His weapons in it are his navi's, from his stats.
-- In Hub Style (NaviStats +0x4C, patch card 111's) he wears its shade.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")
local overlays = require("@exelib/overlays")

-- Hub Style's shade (./hub_shade), as an identity wears it.
local HUB = require("./hub_shade")
local megaman_sprite = asset.sprite("megaman")
local solcross_sprite = asset.sprite("solcross-megaman")

-- 0x0800EE1C (the base form's put-on routine, 0x0800F038's row 0, and MegaMan's actor record's init hook, 0x0800ED90's
-- row 11, as his init ends): in his base form (+0x2C 0, read again) in Hub Style, what he wore down and the shade on
-- (Param3 0). In EXE5DS Sol Cross (hub_style == 3), load 00-0D (`solcross-megaman`) with no HubShade overlay.
local function put_on(navi: Object)
    local stats = battle.navi(navi.alliance)
    if not stats.form.base or stats.hub_style == 0 then
        return
    end
    overlays.take_off_form(navi, stats.form)
    if stats.hub_style == 3 then
        local s = navi.sprite
        local shadow, white, palette = s.shadow, s.white, s.palette
        local anim = navi.anim
        s:load(solcross_sprite)
        s:set_animation(anim)
        s.shadow = shadow
        s:set_flip(bit32.bxor(navi.alliance, navi.flip))
        s.white = white
        s.palette = palette
        return
    end
    navi:add_parts(HUB, 0)
end

local form: FormDef = {
    base = true,
    sprite = megaman_sprite,
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- MegaMan's faces in the emotion window, by his emotion (0x0801AF64: the table 0x0801AFB4 by EXE5's emotion,
    -- 0x08012740; each brings its box): worried for a mood under 65, the dark face for a mood of 0 (a dark
    -- MegaMan's); and Hub Style's (NaviStats +0x4C, the navi's `hub_style`: 0x0801AF8E adds 11), plus EXE5DS Sol Cross
    -- (`variant3`, `hub_style == 3`) and Danger faces (`danger`).
    mugshot = {
        normal = asset.mugshot("megaman"),
        angry = asset.mugshot("megaman-angry"),
        worried = asset.mugshot("megaman-worried"),
        full_synchro = asset.mugshot("megaman-full-synchro"),
        evil = asset.mugshot("megaman-dark"),
        danger = asset.mugshot("megaman-danger"),
        variant = {
            normal = asset.mugshot("megaman-hub"),
            angry = asset.mugshot("megaman-hub-angry"),
            worried = asset.mugshot("megaman-hub-worried"),
            full_synchro = asset.mugshot("megaman-hub-full-synchro"),
            evil = asset.mugshot("megaman-hub-dark"),
            danger = asset.mugshot("megaman-hub-danger"),
        },
        variant2 = {
            normal = asset.mugshot("megaman-hub-colonel"),
            angry = asset.mugshot("megaman-hub-colonel-angry"),
            worried = asset.mugshot("megaman-hub-colonel-worried"),
            full_synchro = asset.mugshot("megaman-hub-colonel-full-synchro"),
            evil = asset.mugshot("megaman-hub-colonel-dark"),
            danger = asset.mugshot("megaman-hub-colonel-danger"),
        },
        variant3 = {
            normal = asset.mugshot("megaman-sol"),
            angry = asset.mugshot("megaman-sol-angry"),
            worried = asset.mugshot("megaman-sol-worried"),
            full_synchro = asset.mugshot("megaman-sol-full-synchro"),
            evil = asset.mugshot("megaman-sol-dark"),
            danger = asset.mugshot("megaman-sol-danger"),
        },
    },
    buster_arm = { anim = 0 },
    put_on = put_on,
}

return { base = form }
