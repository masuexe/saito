--!strict
-- EXE3's attachments: a sprite a navi holds during an attack (muzzle flash, cannon, sword, bomb, gun).

local slot = require("@exelib/slot")

local attachment = {}

type State = {
    slot: Slot,
    look: AttachmentLook,
    anim: number,
    while_dimmed: boolean,
}

function attachment.look(spec: AttachmentLook): AttachmentLook
    return new.record("attachment-look", spec)
end

local WORN = { "color_shader", "white", "alpha", "hflip", "vflip" }

local function init(me: Object)
    local s = me.state :: State
    local k = s.look
    local sprite = me.sprite
    sprite:load(k.sprite)
    sprite.shadow = "with_sprite"
    me.anim = s.anim
    me.anim_loaded = s.anim
    me.visible = true
    sprite:set_animation(s.anim)
    sprite.palette = k.palette
    me:set_coordinates_from_panel()
    me:set_lifecycle("update")
    local lift = (k.lift or 0) * 0x10000
    slot.follow(me, me.related1 :: Object, s.slot, {
        offset = Vec3.new(0, -lift, -lift),
        looks = WORN,
        visibility = true,
        while_dimmed = s.while_dimmed,
    })
end

attachment.kind = new.kind {
    id = "attachment",
    pool = "attack",
    state = { slot = slot.TYPE, look = "record:attachment-look", anim = "u8", while_dimmed = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        else
            me:free()
        end
    end,
}

function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, anim: number, while_dimmed: boolean?): Object?
    local o = battle.spawn(attachment.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.alliance = owner.alliance
        o.run_while_dimmed = true
        local s = o.state :: State
        s.slot = which
        s.look = look
        s.anim = anim
        s.while_dimmed = while_dimmed == true
    end
    slot.fill(owner, which, o)
    return o
end

return attachment
