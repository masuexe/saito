//! The pictures the builds' screens show of a game's definitions, from its graphics as a frame draws them
//! (`saito_render::pictures`): a chip's icon and its picture in the chip window, a chip family's element
//! icon, a form's face (a Cross's, a soul's), a navi's, and the face a round starts a navi with. Each is made
//! the first time it is asked for, and kept.

use crate::games::Ready;
use saito_battle::content::{ChipClass, ChipFamily};
use saito_content_api::{ChipHandle, FormHandle, NaviHandle};
use saito_match::build::Face;
use saito_render::graphics::GameGraphics;
use saito_render::pictures::{self, Image};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A game's pictures, made as they are asked for.
pub struct Pictures {
    ready: Rc<Ready>,
    graphics: GameGraphics,
    icons: RefCell<HashMap<ChipHandle, slint::Image>>,
    art: RefCell<HashMap<ChipHandle, slint::Image>>,
    hd_art: Cell<bool>,
    elements: RefCell<HashMap<ChipFamily, slint::Image>>,
    hd_elements: Cell<bool>,
    faces: RefCell<HashMap<FormHandle, slint::Image>>,
    navis: RefCell<HashMap<NaviHandle, slint::Image>>,
    shown: RefCell<HashMap<Face, slint::Image>>,
}

/// An RGBA picture as the window shows one (none: an empty image).
fn image(p: Option<Image>) -> slint::Image {
    let Some(p) = p else { return slint::Image::default() };
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&p.rgba, p.width, p.height);
    slint::Image::from_rgba8(buffer)
}

/// What `make` makes of `key`, made once.
fn kept<K: std::hash::Hash + Eq>(map: &RefCell<HashMap<K, slint::Image>>, key: K, make: impl FnOnce() -> Option<Image>) -> slint::Image {
    if let Some(i) = map.borrow().get(&key) {
        return i.clone();
    }
    let i = image(make());
    map.borrow_mut().insert(key, i.clone());
    i
}

// (The form's own face is the tabs' to come: the NaviCust's and the cards' lists show none yet.)
#[allow(dead_code)]
impl Pictures {
    pub fn new(ready: Rc<Ready>) -> Pictures {
        let graphics = ready.loaded.graphics.game_graphics();
        Pictures {
            ready,
            graphics,
            icons: Default::default(),
            art: Default::default(),
            hd_art: Cell::new(false),
            elements: Default::default(),
            hd_elements: Cell::new(false),
            faces: Default::default(),
            navis: Default::default(),
            shown: Default::default(),
        }
    }

    /// A chip's 16x16 icon.
    pub fn icon(&self, chip: ChipHandle) -> slint::Image {
        kept(&self.icons, chip, || pictures::chip_icon(&self.graphics.view(), self.ready.content(), chip))
    }

    /// A chip's picture (Legacy Collection HD when `hd` is true and available, else the 56x48 ROM art). A Program
    /// Advance has none (what its game's graphics hold in the place of one is no picture of it): an empty image.
    pub fn art(&self, chip: ChipHandle, hd: bool) -> slint::Image {
        if self.ready.content().chip(chip).class == ChipClass::ProgramAdvance {
            return slint::Image::default();
        }
        if self.hd_art.get() != hd {
            self.art.borrow_mut().clear();
            self.hd_art.set(hd);
        }
        kept(&self.art, chip, || pictures::chip_art_with_hd(&self.graphics.view(), self.ready.content(), chip, hd))
    }

    /// A chip's element icon: its family's, as the game's chip window shows it. A Program Advance has no element: an
    /// empty image.
    pub fn chip_element(&self, chip: ChipHandle, hd: bool) -> slint::Image {
        let record = self.ready.content().chip(chip);
        if record.class == ChipClass::ProgramAdvance {
            return slint::Image::default();
        }
        self.element(record.family, hd)
    }

    /// A chip family's element icon: Legacy Collection HD (108×108) when `hd` is true and available,
    /// else its frame smooth at any size drawn from the game's drawing (`pictures::element_icon_svg`),
    /// falling back to the 16×16 pixel picture.
    pub fn element(&self, family: ChipFamily, hd: bool) -> slint::Image {
        if self.hd_elements.get() != hd {
            self.elements.borrow_mut().clear();
            self.hd_elements.set(hd);
        }
        if let Some(i) = self.elements.borrow().get(&family) {
            return i.clone();
        }
        let graphics = self.graphics.view();
        let i = if let Some(hd_img) = pictures::hd_element_icon(&graphics, family, hd) {
            image(Some(hd_img))
        } else {
            let svg = pictures::element_icon_svg(&graphics, family).and_then(|s| slint::Image::load_from_svg_data(s.as_bytes()).ok());
            svg.unwrap_or_else(|| image(pictures::element_icon(&graphics, family)))
        };
        self.elements.borrow_mut().insert(family, i.clone());
        i
    }

    /// A form's 32x16 face.
    pub fn face(&self, form: FormHandle) -> slint::Image {
        kept(&self.faces, form, || pictures::form_face(&self.graphics.view(), self.ready.content(), form))
    }

    /// A navi's 32x16 face: its own, else its starting form's mugshot, else its
    /// starting form's battle sprite frame (for games without emotion-window mugshots).
    pub fn navi(&self, navi: NaviHandle) -> slint::Image {
        let content = self.ready.content();
        kept(&self.navis, navi, || {
            let graphics = self.graphics.view();
            let own = saito_render::lookups::navi_face(&graphics, content, navi, &mut Default::default());
            own.and_then(|(tiles, palettes, _)| pictures::image(tiles, 4, 2, palettes.first()?)).or_else(|| {
                let form_handle = saito_match::Side::fresh_stats(content, navi).starting_form;
                pictures::form_face(&graphics, content, form_handle).or_else(|| {
                    let form = content.form(form_handle);
                    pictures::sprite_frame(&graphics, content, form.sprite, form.idle_anim, form.palette)
                })
            })
        })
    }

    /// The face the emotion window shows as a round starts (`saito_match::build::Checked::face`): a form's
    /// for an emotion, a navi's own, or its battle sprite preview.
    pub fn shown(&self, face: &Face) -> slint::Image {
        match face {
            Face::Navi(n) => self.navi(*n),
            Face::Form(form, emotion, variant) => kept(&self.shown, face.clone(), || pictures::face(&self.graphics.view(), self.ready.content(), *form, emotion, *variant)),
            Face::Sprite(sprite, anim, palette) => kept(&self.shown, face.clone(), || pictures::sprite_frame(&self.graphics.view(), self.ready.content(), *sprite, *anim, *palette)),
        }
    }
}
