--!strict
-- EXE3's engine roles: chips, statuses, sparks, collision types, regions, actions, kinds, sounds, effects, sprites,
-- music, and banners.

local barriers = require("../lib/barriers")
local charge_glow = require("./charge_glow")
local collision = require("./collision")
local deletion = require("./deletion")
local invalid = require("../chips/invalid").invalid
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("./status")
local zeroed = require("../chips/zeroed").zeroed
local blindness_sprite = asset.sprite("blindness")
local bubble_sprite = asset.sprite("bubble")
local burst_sprite = asset.sprite("burst")
local charge_glow_sprite = asset.sprite("charge-glow")
local charge_glow_a_sprite = charge_glow_sprite
local confusion_sprite = asset.sprite("confusion")
local deletion_sprite = asset.sprite("deletion")
local heal_sprite = asset.sprite("heal")
local hit_marker_sprite = asset.sprite("hit-marker")
local hit_sparks_sprite = asset.sprite("hit-sparks")
local ice_sprite = asset.sprite("ice")
local immobilized_sprite = asset.sprite("immobilized")
local target_marker_sprite = hit_sparks_sprite

local roles: RolesSpec = {
    chips = {
        zeroed = zeroed,
        invalid = invalid,
    },
    statuses = {
        damage_word_paralysis = statuses.paralyze,
        damage_word_confusion = statuses.confuse,
        damage_word_blindness = statuses.blind,
        counter_paralysis = statuses.paralyze,
    },
    sparks = {
        plain = sparks.plain,
        guard = sparks.guard,
        thrown_obstacle = sparks.plain,
    },
    collision = {
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
        thrown_obstacle = collision.breaking,
        thrown_obstacle_target = collision.hits_navis,
    },
    regions = {
        anchor = regions.single,
    },
    actions = {
        deletion = deletion.action,
    },
    kinds = {
        charge_glow = charge_glow.kind,
    },
    sounds = {
        appear = asset.sound("appear"),
        arrive = asset.sound("arrive"),
        auto_battle_hit = asset.sound("own-hit"),
        bubble = asset.sound("bubble"),
        bubble_pop = asset.sound("bubble-pop"),
        burst = asset.sound("burst"),
        buster_charge = asset.sound("buster-charge"),
        buster_charged = asset.sound("buster-charged"),
        confusion = asset.sound("confusion"),
        damage = asset.sound("damage"),
        deleted = asset.sound("deleted"),
        fade = asset.sound("fade"),
        freeze = asset.sound("freeze"),
        gauge_full = asset.sound("gauge-full"),
        invisible = asset.sound("invisible"),
        low_hp = asset.sound("low-hp"),
        counter_hit = asset.sound("counter-hit"),
        damage_bonus = asset.sound("damage-bonus"),
        panel_crack = asset.sound("panel-crack"),
        refused = asset.sound("sfx-error"),
        custom_open = asset.sound("sfx-custom-open"),
        custom_cursor = asset.sound("custom-cursor"),
        custom_pick = asset.sound("custom-pick"),
        custom_ok = asset.sound("custom-ok"),
        custom_back = asset.sound("custom-back"),
        custom_cancel = asset.sound("custom-back"),
        custom_description = asset.sound("description-open"),
        program_advance_part = asset.sound("program-advance-part"),
        program_advance = asset.sound("program-advance"),
        encounter = asset.sound("encounter"),
        hit = asset.sound("hit"),
        own_hit = asset.sound("own-hit"),
        guard = asset.sound("guard"),
        recovery = asset.sound("sfx-recover"),
        trap_mark = asset.sound("trap-mark"),
        obstacle_lift = asset.sound("obstacle-lift"),
        obstacle_throw = asset.sound("obstacle-throw"),
        pause = asset.sound("pause"),
    },
    effects = {
        recovery = new.effect { sprite = heal_sprite, anim = 0, palette = 0, steps = 16 },
        cut_in_flash = new.effect { sprite = hit_marker_sprite, anim = 0, palette = 0, steps = 18 },
        trap_mark = new.effect { sprite = hit_sparks_sprite, anim = 7, palette = 0, steps = 24 },
        deletion = new.effect { sprite = deletion_sprite, anim = 0, palette = 0, steps = 32 },
        burst = new.effect { sprite = burst_sprite, anim = 0, palette = 0, steps = 16 },
    },
    hooks = {
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
    },
    sprites = {
        charge_glow = charge_glow_sprite,
        charge_glow_a = charge_glow_a_sprite,
        confusion = confusion_sprite,
        blindness = blindness_sprite,
        immobilized = immobilized_sprite,
        ice = ice_sprite,
        bubble = bubble_sprite,
        hit_marker = hit_marker_sprite,
        target_marker = target_marker_sprite,
    },
    music = {
        link_battle = asset.sound("battle-boss"),
        link_battle_single = asset.sound("battle-net"),
        winner = asset.sound("battle-win"),
        winner_special = asset.sound("battle-win"),
        loser = asset.sound("battle-lose"),
        title_screen = asset.sound("title-screen"),
    },
    banners = {
        round_start = asset.banner("round-start"),
        turn_start = asset.banner("turn-start"),
        final_turn = asset.banner("final-turn"),
        draw = asset.banner("draw"),
        win = asset.banner("enemy-deleted"),
        win_judged = asset.banner("win"),
        lose_judged = asset.banner("lose"),
        judge = asset.banner("hit-damage"),
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance"),
    },
}

return roles
