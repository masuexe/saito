--!strict
-- EXE3's barriers and auras, using data-driven `status.intake.barrier.types` IDs:
--   ID 1: Barrier (10 HP, plain)
--   ID 2: Barr100 (100 HP, plain)
--   ID 3: Barr200 (200 HP, plain)
--   ID 6: Aura (100 threshold, 1800 ticks)
--   ID 7: LifeAura (200 threshold, 1800 ticks)
--   ID 8: FirstBarrier (EXE3's BattleMegaManInit 0x080B048C: plain 100-HP barrier, NOT BubbleWrap!)
--   ID 9: BubbleWrap (distinct EXE3 ID with elec pop + 240-tick regrow)

local barrier_visual = require("../objects/barrier_visual")
local common_controller = require("@exelib/barriers/controller")
local common_visual = require("@exelib/barriers/visual")
local element = require("@exelib/element")
local aura_sprite = asset.sprite("aura")
local barrier_sprite = aura_sprite

local barriers = {}

local SPRITE, AURA = barrier_sprite, aura_sprite
local SOUND = asset.sound("barrier")
local NO_TIMER = 0xFFFF
local PART_7 = 0x1000000

local BARRIER_IDS: { [string]: number } = {
    ["barrier/10"] = 1,
    ["barrier/100"] = 2,
    ["barrier/200"] = 3,
    ["barrier/aura"] = 6,
    ["barrier/life-aura"] = 7,
    ["barrier/first"] = 8,
    ["barrier/bubble-wrap"] = 9,
}

local function barrier(b: Barrier): Barrier
    return new.record("barrier", b)
end

-- ID 1: Barrier (10 HP).
barriers.barrier_10 = barrier {
    id = "barrier/10",
    behavior = "plain",
    hp = 10,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 4, palette = 0, attach_point = 0, hidden_parts = 0, up_steps = 16, down_steps = 21 },
    sound = SOUND,
}

-- ID 2: Barr100 (100 HP).
barriers.barrier_100 = barrier {
    id = "barrier/100",
    behavior = "plain",
    hp = 100,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 4, palette = 1, attach_point = 0, hidden_parts = 0, up_steps = 16, down_steps = 21 },
    sound = SOUND,
}

-- ID 3: Barr200 (200 HP).
barriers.barrier_200 = barrier {
    id = "barrier/200",
    behavior = "plain",
    hp = 200,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 4, palette = 2, attach_point = 0, hidden_parts = 0, up_steps = 16, down_steps = 21 },
    sound = SOUND,
}

-- ID 6: Aura (blocks < 100 damage for 1800 ticks).
barriers.aura = barrier {
    id = "barrier/aura",
    behavior = "plain",
    hp = 1,
    threshold = 100,
    timer = 1800,
    look = { sprite = AURA, anim = 0, palette = 0, attach_point = 0, hidden_parts = PART_7, up_steps = 9, down_steps = 19 },
    sound = SOUND,
}

-- ID 7: LifeAura (blocks < 200 damage for 1800 ticks).
barriers.life_aura = barrier {
    id = "barrier/life-aura",
    behavior = "plain",
    hp = 1,
    threshold = 200,
    timer = 1800,
    look = { sprite = AURA, anim = 0, palette = 1, attach_point = 0, hidden_parts = PART_7, up_steps = 9, down_steps = 19 },
    sound = SOUND,
}

-- ID 8: FirstBarrier (EXE3's Shield Style / NaviCust FirstBarrier: plain 100-HP barrier, ID 8).
barriers.first_barrier = barrier {
    id = "barrier/first",
    behavior = "plain",
    hp = 100,
    threshold = 0,
    timer = NO_TIMER,
    look = { sprite = SPRITE, anim = 4, palette = 1, attach_point = 0, hidden_parts = 0, up_steps = 16, down_steps = 21 },
    sound = SOUND,
}

-- ID 9: BubbleWrap (1 HP, regrows after 240 ticks, popped by elec).
barriers.bubble_wrap = barrier {
    id = "barrier/bubble-wrap",
    behavior = "bubble",
    hp = 1,
    threshold = 0,
    timer = NO_TIMER,
    weak = "elec",
    look = { sprite = SPRITE, anim = 4, palette = 0, attach_point = 0, hidden_parts = 0, up_steps = 16, down_steps = 21 },
    sound = SOUND,
}

function barriers.raise(navi: Object, b: Barrier, keep_old: boolean?)
    navi:raise_barrier {
        id = if b.id then BARRIER_IDS[b.id] else nil,
        behavior = b.behavior,
        hp = b.hp,
        threshold = b.threshold,
        timer = b.timer,
        weak_element = if b.weak then element.byte(b.weak) else 0,
    }
    local old = navi.barrier_visual
    if old and not keep_old then
        common_visual.finish(old)
    end
    barrier_visual.spawn(navi, b)
end

barriers.controller = common_controller.make {
    id = "barriers/controller",
    raise = function(navi: Object, b: Barrier)
        barriers.raise(navi, b)
    end,
}

return barriers
