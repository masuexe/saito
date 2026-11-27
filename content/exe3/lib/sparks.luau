--!strict
-- EXE3's hit sparks: what shows where a hit lands (`collision:set_hit_effect`).

local sparks = {}

local HIT_SPARKS = asset.sprite("hit-sparks")
local chip_destruction_spark_sprite = asset.sprite("chip-destruction-spark")
local explosion_sprite = asset.sprite("explosion")
local hit_sprite = asset.sprite("hit")
local impact_sprite = asset.sprite("impact")
local pink_flash_sprite = asset.sprite("pink-flash")

-- By element (0=null, 1=elec, 2=heat/fire, 3=aqua, 4=wood, 5=drain/charged).
local BY_ELEMENT: { [number]: Spark } = {
    [0] = new.spark { sprite = HIT_SPARKS, anim = 0, palette = 0, steps = 5 },
    [1] = new.spark { sprite = HIT_SPARKS, anim = 4, palette = 0, steps = 14 },
    [2] = new.spark { sprite = HIT_SPARKS, anim = 2, palette = 0, steps = 18 },
    [3] = new.spark { sprite = HIT_SPARKS, anim = 3, palette = 0, steps = 14 },
    [4] = new.spark { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 16 },
    [5] = new.spark { sprite = pink_flash_sprite, anim = 0, palette = 0, steps = 10 },
}

sparks.plain = BY_ELEMENT[0]
sparks.charged = BY_ELEMENT[5]
sparks.elec, sparks.fire, sparks.aqua, sparks.wood = BY_ELEMENT[1], BY_ELEMENT[2], BY_ELEMENT[3], BY_ELEMENT[4]
sparks.heavy = new.spark { sprite = HIT_SPARKS, anim = 8, palette = 0, steps = 5 }
sparks.impact = new.spark { sprite = impact_sprite, anim = 0, palette = 0, steps = 26 }
sparks.guard = new.spark { sprite = hit_sprite, anim = 0, palette = 0, steps = 10 }
sparks.chip_destruction = new.spark { sprite = chip_destruction_spark_sprite, anim = 0, palette = 0, steps = 15 }
sparks.breaking = new.spark { sprite = HIT_SPARKS, anim = 1, palette = 0, steps = 10 }
sparks.blast = new.spark { sprite = explosion_sprite, anim = 0, palette = 0, steps = 27 }

function sparks.of_element(element: number): Spark
    return BY_ELEMENT[element] or BY_ELEMENT[0]
end

return sparks
