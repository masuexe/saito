--!strict
-- EXE3's Japanese lettering from White Japan (`A6BJ`, `ROCKMANEXE3`).
-- Extracts the Japanese 8x16 HUD font, HUD text lines, banners that differ from English, and Japanese Custom Screen
-- OK / OK_PICKED / ADD pictures.

local hud = require("./hud")
local custom = require("./custom")

local lettering = {}

lettering.LANGUAGE = "ja"

local function same_banner(a: BannerLayout, b: BannerLayout): boolean
    return a.x == b.x and a.y == b.y and a.kind == b.kind and a.glyphs == b.glyphs
end

function lettering.hud(set: Set, base: Hud, names: AssetNames): HudLettering
    local rom = set.A6BJ
    local languages: { [string]: { glyphs: { string }?, dialogue_glyphs: { string }? } } = names.language_glyphs or {}
    local glyphs = languages[lettering.LANGUAGE]
    local cell: { string } = if glyphs and glyphs.glyphs then glyphs.glyphs else {}
    local dialogue: { string } = if glyphs and glyphs.dialogue_glyphs then glyphs.dialogue_glyphs else {}
    local font, font_chars = hud.font(rom, cell, dialogue)
    local dialogue_font = hud.dialogue_font(rom, cell, dialogue)
    local texts = hud.texts(rom)
    local own_banners = base.banners or {}
    local banners: { [number]: BannerLayout, n: number } = { n = #own_banners }
    for i, own in own_banners do
        local b = hud.banner(rom, i - 1)
        if not same_banner(b, own) then
            banners[i - 1] = b
        end
    end
    return {
        font = font,
        font_chars = font_chars,
        dialogue_font = dialogue_font,
        texts = texts,
        banners = banners,
        banner_palette = base.banner_palette,
        gauge_tiles = base.gauge_tiles,
    }
end

function lettering.custom(set: Set, _base: CustomScreen): CustomLettering
    -- EXE3's Custom Screen OK / OK_PICKED pictures are full-color MegaMan/transmission artwork (no localized text;
    -- identical 8x7 tiles between A6BE and A6BJ), so only the ADD button picture needs a JP override if palettes vary.
    local rom = set.A6BJ
    local add_pic = custom.picture(rom, custom.ADD_PICTURE)
    return {
        buttons = {
            { name = custom.ADD_BUTTON, value = { picture = add_pic } },
        },
    }
end

return lettering
