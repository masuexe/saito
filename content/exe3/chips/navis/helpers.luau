--!strict
-- Shared helpers, effects, sounds, and chip constructor for EXE3 Navi chips (V1-V5).

local dimming_chips = require("@exelib/dimming")

local helpers = {}

local HIT_SPARKS = asset.sprite("hit-sparks")

helpers.FIRE_FX = new.effect { sprite = HIT_SPARKS, anim = 2, palette = 0, steps = 16 }
helpers.AQUA_FX = new.effect { sprite = HIT_SPARKS, anim = 3, palette = 0, steps = 14 }
helpers.ELEC_FX = new.effect { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 }
helpers.WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 16 }
helpers.HEAVY_FX = new.effect { sprite = HIT_SPARKS, anim = 8, palette = 0, steps = 10 }
helpers.BREAK_FX = new.effect { sprite = HIT_SPARKS, anim = 1, palette = 0, steps = 12 }
helpers.SLASH_FX = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0, steps = 12 }
helpers.BIG_SLASH_FX = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0, steps = 15 }
helpers.BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
helpers.IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 18 }
helpers.BURST_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 14 }
helpers.PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
helpers.DUST_FX = new.effect { sprite = asset.sprite("dust"), anim = 0, palette = 0, steps = 14 }
helpers.HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }
helpers.FLASH_FX = new.effect { sprite = asset.sprite("pink-flash"), anim = 1, palette = 0, steps = 10 }
helpers.BUBBLE_FX = new.effect { sprite = asset.sprite("grab-shot"), anim = 0, palette = 0, steps = 14 }
helpers.TARGET_FX = new.effect { sprite = HIT_SPARKS, anim = 0, palette = 0, steps = 14 }
helpers.CONFUSE_FX = new.effect { sprite = asset.sprite("confusion"), anim = 0, palette = 0, steps = 16 }
helpers.BLIND_FX = new.effect { sprite = asset.sprite("blindness"), anim = 0, palette = 0, steps = 16 }

helpers.APPEAR_SOUND = asset.sound("appear")
helpers.SWING_SOUND = asset.sound("big-sword-swing")
helpers.SWORD_SOUND = asset.sound("sword-swing")
helpers.BLAST_SOUND = asset.sound("hit-bomb")
helpers.SPREADER_SOUND = asset.sound("spreader")
helpers.CRACK_SOUND = asset.sound("panel-crack")
helpers.CRACK_SHOT_SOUND = asset.sound("crack-shot")
helpers.RECOVER_SOUND = asset.sound("sfx-recover")
helpers.BUBBLE_SOUND = asset.sound("bubble")
helpers.BUBBLE_POP_SOUND = asset.sound("bubble-pop")
helpers.THROW_SOUND = asset.sound("obstacle-throw")
helpers.BURST_SOUND = asset.sound("burst")
helpers.CONFUSE_SOUND = asset.sound("confusion")
helpers.ARRIVE_SOUND = asset.sound("arrive")
helpers.CANNON_SOUND = asset.sound("cannon")

function helpers.dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

function helpers.enemy_target(side: number): (number, number)
    local opp = battle.player(bit32.bxor(side, 1))
    if opp and not opp:status("dead") then
        return opp.panel_x, opp.panel_y
    end
    local actors = battle.alive_actors(bit32.bxor(side, 1))
    if #actors > 0 then
        return actors[1].panel_x, actors[1].panel_y
    end
    return if side == 0 then 5 else 2, 2
end

function helpers.enemy_front_col(side: number): number
    local opp_side = bit32.bxor(side, 1)
    if side == 0 then
        for x = 1, 6 do
            for y = 1, 3 do
                local p = field.panel(x, y)
                if p and p.alliance == opp_side then
                    return x
                end
            end
        end
        return 4
    else
        for x = 6, 1, -1 do
            for y = 1, 3 do
                local p = field.panel(x, y)
                if p and p.alliance == opp_side then
                    return x
                end
            end
        end
        return 3
    end
end

function helpers.navi_chip(
    codes: { string },
    elem: string,
    fam: string,
    power: number,
    mb: number,
    rarity: number,
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
    class_override: string?
): Chip
    return {
        codes = codes,
        element = elem,
        family = fam,
        class = class_override or "mega",
        rarity = rarity,
        mb = mb,
        damage = power,
        hit_param = 0,
        lockout = 0,
        flags = { "dimming", "has_damage", "navi", "library" },
        dimming = hook,
    }
end

function helpers.simple_hook(kind: Kind, side_only: boolean?): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(kind, user, spec, side_only)
    end
end

return helpers
