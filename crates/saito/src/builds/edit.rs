//! A build's edits, each through the facts' own writer (`saito_match`): an edit a fact doesn't take changes
//! nothing. The folder's ([`folder`]: a chip put in an entry or taken out, the Regular chip, the tag chips),
//! and a list of definitions' ([`list`]: the patch cards', each a record of its card and the variant chosen for it,
//! [`variant`]).
//!
//! What a build's forms, its navi and its choices set is saito-match's (`build::Builds`), not an edit here.

use saito_battle::content::{ChipCode, Content, PlayerFact};
use saito_battle::custom::FolderChip;
use saito_battle::rules::Fact;
use saito_content_api::{ChipHandle, FieldType, Registry};
use saito_match::Side;
use saito_match::facts::{self, Stated, fact_of};

/// The type of a list's element, else the type itself.
fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// A list's room.
fn room(ty: &FieldType) -> usize {
    match ty {
        FieldType::List(_, n) => *n as usize,
        FieldType::Array(_, n) => *n as usize,
        _ => 0,
    }
}

/// An edit of a list of definitions.
#[derive(Clone, Copy, Debug)]
pub enum ListEdit {
    /// One added at its end.
    Add(u16),
    /// One added at a place, those from there on shifting one on (past the end: at the end).
    Insert(usize, u16),
    Remove(usize),
    /// An entry moved to another place, those between shifting one toward where it was.
    Move(usize, usize),
    /// Every entry taken out (the Patch cards tab's CLEAR).
    Clear,
}

/// Where the entry at `i` is after the one at `from` is moved to `to`, those between shifting one toward where it
/// was.
pub fn moved(i: usize, from: usize, to: usize) -> usize {
    match i {
        _ if i == from => to,
        _ if from < i && i <= to => i - 1,
        _ if to <= i && i < from => i + 1,
        _ => i,
    }
}

/// An element of a list of type `ty` that names definition `h`: the definition, or of a list of records the record
/// of its one field that names a definition, the rest none (a patch card of no variant yet).
fn naming(ty: &FieldType, h: u16) -> Option<Stated> {
    match element(ty) {
        FieldType::Ref(registry, _) => Some(Stated::Def(*registry, Some(h))),
        FieldType::Record(fields) => fields.fields().iter().find_map(|f| match &f.ty {
            FieldType::Ref(registry, _) => Some(Stated::Record(vec![(f.name.clone(), Stated::Def(*registry, Some(h)))])),
            _ => None,
        }),
        _ => None,
    }
}

/// Apply `edit` to the side's list `name` (of definitions, or of records each naming one; a definition is in it
/// once): whether it changed.
pub fn list(content: &Content, side: &mut Side, name: &str, edit: ListEdit) -> bool {
    let Some(field) = facts::field(content, name) else { return false };
    let Some(Stated::List(mut items)) = side.facts.get(content, name) else { return false };
    let full = items.len() >= room(field.ty);
    let held = |h: u16| items.iter().any(|x| x.defs().first() == Some(&h));
    match edit {
        ListEdit::Add(h) | ListEdit::Insert(_, h) if full || held(h) => return false,
        ListEdit::Add(h) => match naming(field.ty, h) {
            Some(x) => items.push(x),
            None => return false,
        },
        ListEdit::Insert(at, h) => match naming(field.ty, h) {
            Some(x) => items.insert(at.min(items.len()), x),
            None => return false,
        },
        ListEdit::Remove(i) if i < items.len() => {
            items.remove(i);
        }
        ListEdit::Move(from, to) if from != to && from.max(to) < items.len() => {
            let item = items.remove(from);
            items.insert(to, item);
        }
        ListEdit::Clear if !items.is_empty() => items.clear(),
        _ => return false,
    }
    write(content, side, name, &Stated::List(items))
}

/// Choose the variant of the record in place `i` of the side's list `name` (a patch card's, its `variant`: a
/// version's id; none, none): whether it changed.
pub fn variant(content: &Content, side: &mut Side, name: &str, i: usize, variant: Option<&str>) -> bool {
    let Some(Stated::List(mut items)) = side.facts.get(content, name) else { return false };
    let Some(Stated::Record(fields)) = items.get_mut(i) else { return false };
    let value = Stated::Variant(variant.map(str::to_string));
    match fields.iter_mut().find(|(n, _)| n == saito_match::ruleset::VARIANT) {
        Some((_, v)) if *v == value => return false,
        Some((_, v)) => *v = value,
        None => fields.push((saito_match::ruleset::VARIANT.to_string(), value)),
    }
    write(content, side, name, &Stated::List(items))
}

/// Write `value` as the side's fact `name`: whether it took it.
fn write(content: &Content, side: &mut Side, name: &str, value: &Stated) -> bool {
    let values = match fact_of(value) {
        Fact::List(items) => items,
        f => vec![f],
    };
    side.set_fact(content, name, &values).is_ok()
}

/// An edit of the folder, by entry (from 0).
#[derive(Clone, Copy, Debug)]
pub enum FolderEdit {
    Put(usize, ChipHandle, ChipCode),
    Clear(usize),
    /// The entry made the Regular chip, or no longer.
    Regular(usize),
    /// The entry made a tag chip (its pair the next one tagged), or no longer.
    Tag(usize),
    /// The entry moved to another, those between shifting one toward where it was: the Regular and tag marks go
    /// with their chips.
    Move(usize, usize),
}

/// Sort `folder`'s filled entries before empty ones using `cmp`, preserving relative order among equal
/// entries; returns the sorted folder and `new_pos[old_idx]` mapping if any entry moved. Regular and tag marks
/// move with their chips.
pub fn sort_folder(
    folder: &saito_match::Folder,
    mut cmp: impl FnMut(FolderChip, FolderChip) -> std::cmp::Ordering,
) -> Option<(saito_match::Folder, [usize; saito_battle::custom::folder::FOLDER_SIZE])> {
    let mut indexed: Vec<(usize, Option<FolderChip>)> = folder.chips.iter().copied().enumerate().collect();
    indexed.sort_by(|&(ai, a), &(bi, b)| match (a, b) {
        (Some(a), Some(b)) => cmp(a, b).then_with(|| ai.cmp(&bi)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => ai.cmp(&bi),
    });
    let mut after = *folder;
    let mut new_pos = [0usize; saito_battle::custom::folder::FOLDER_SIZE];
    for (to, &(from, chip)) in indexed.iter().enumerate() {
        after.chips[to] = chip;
        new_pos[from] = to;
    }
    after.regular = folder.regular.map(|i| new_pos[i as usize] as u8);
    after.tags = folder.tags.map(|(a, b)| {
        let (na, nb) = (new_pos[a as usize] as u8, new_pos[b as usize] as u8);
        (na.min(nb), na.max(nb))
    });
    (after != *folder).then_some((after, new_pos))
}

/// The folder `edit` makes of `f`; none: it changes nothing (an entry past the folder, a mark on an empty
/// entry, a tag in a game whose folders have none).
pub fn folder_after(content: &Content, f: &saito_match::Folder, edit: FolderEdit) -> Option<saito_match::Folder> {
    let mut f = *f;
    let before = f;
    match edit {
        FolderEdit::Put(i, chip, code) if i < f.chips.len() => f.chips[i] = Some(FolderChip::new(chip, code)),
        FolderEdit::Clear(i) if i < f.chips.len() => {
            f.chips[i] = None;
            let i = i as u8;
            if f.regular == Some(i) {
                f.regular = None;
            }
            if f.tags.is_some_and(|(a, b)| a == i || b == i) {
                f.tags = None;
            }
        }
        FolderEdit::Regular(i) if f.has(i as u8) => {
            let i = i as u8;
            f.regular = if f.regular == Some(i) { None } else { Some(i) };
        }
        FolderEdit::Tag(i) if f.has(i as u8) && content.defs.fact_field(PlayerFact::TagChips).is_some() => {
            let i = i as u8;
            f.tags = match f.tags {
                Some((a, b)) if a == i || b == i => None,
                // (A tag chip waiting for its pair is tagged with itself.)
                Some((a, b)) if a == b => Some((a.min(i), a.max(i))),
                _ => Some((i, i)),
            };
        }
        FolderEdit::Move(from, to) if from != to && from.max(to) < f.chips.len() => {
            let chip = f.chips[from];
            if from < to {
                f.chips.copy_within(from + 1..=to, from);
            } else {
                f.chips.copy_within(to..from, to + 1);
            }
            f.chips[to] = chip;
            let at = |i: u8| moved(i as usize, from, to) as u8;
            f.regular = f.regular.map(at);
            f.tags = f.tags.map(|(a, b)| (at(a).min(at(b)), at(a).max(at(b))));
        }
        _ => return None,
    }
    (f != before).then_some(f)
}

/// Apply `edit` to the side's folder: whether it changed.
pub fn folder(content: &Content, side: &mut Side, edit: FolderEdit) -> bool {
    folder_after(content, &side.folder(content), edit).is_some_and(|f| side.set_folder(content, &f).is_ok())
}

/// The definitions of `registry` (of collection `of`) in `game`.
pub fn of_game(content: &Content, game: &str, registry: Registry, of: Option<&str>) -> Vec<u16> {
    saito_match::ids::all_of(content, registry, of)
        .into_iter()
        .filter(|&h| saito_match::ids::key_of(content, registry, h).is_some_and(|k| saito_match::ids::in_game(content, game, k)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_match::testing::exe6_content;

    /// The folder's entries, its Regular and tag chips; a list's entries added once each, moved and taken out.
    #[test]
    fn folders_and_lists() {
        let six = exe6_content();
        let mut m = saito_match::Match::empty(&six, "exe6").unwrap();
        let side = &mut m.sides[0];
        let cannon = saito_match::ids::chip(&six, "exe6", "cannon").unwrap();
        let a = six.chip(cannon).codes[0];
        assert!(folder(&six, side, FolderEdit::Put(0, cannon, a)) && folder(&six, side, FolderEdit::Put(1, cannon, a)));
        assert!(!folder(&six, side, FolderEdit::Regular(2)), "an empty entry");
        assert!(folder(&six, side, FolderEdit::Regular(1)));
        assert!(folder(&six, side, FolderEdit::Tag(0)) && folder(&six, side, FolderEdit::Tag(1)));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (Some(1), Some((0, 1))));
        // (Moved, the marks go with their chips.)
        assert!(folder(&six, side, FolderEdit::Move(0, 3)));
        assert_eq!((side.folder(&six).chips[3].is_some(), side.folder(&six).chips[0].is_some()), (true, true));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (Some(0), Some((0, 3))));
        assert!(folder(&six, side, FolderEdit::Move(3, 0)) && !folder(&six, side, FolderEdit::Move(2, 2)));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (Some(1), Some((0, 1))));
        assert!(folder(&six, side, FolderEdit::Clear(1)));
        assert_eq!((side.folder(&six).regular, side.folder(&six).tags), (None, None));
        let cards = of_game(&six, "exe6", Registry::Entry, Some("patch_cards"));
        assert!(list(&six, side, "patch_cards", ListEdit::Add(cards[0])) && list(&six, side, "patch_cards", ListEdit::Add(cards[1])));
        assert!(!list(&six, side, "patch_cards", ListEdit::Add(cards[0])), "once");
        assert!(list(&six, side, "patch_cards", ListEdit::Move(1, 0)));
        assert_eq!(side.facts.get(&six, "patch_cards").unwrap().defs(), [cards[1], cards[0]]);
        assert!(!list(&six, side, "patch_cards", ListEdit::Move(0, 2)) && !list(&six, side, "patch_cards", ListEdit::Move(1, 1)));
        assert!(list(&six, side, "patch_cards", ListEdit::Remove(0)));
        assert_eq!(side.facts.get(&six, "patch_cards").unwrap().defs(), [cards[0]]);
        // (Put in at a place, those after it shifting on; past the end, at the end; once.)
        assert!(list(&six, side, "patch_cards", ListEdit::Insert(0, cards[2])) && list(&six, side, "patch_cards", ListEdit::Insert(9, cards[3])));
        assert!(list(&six, side, "patch_cards", ListEdit::Insert(1, cards[4])) && !list(&six, side, "patch_cards", ListEdit::Insert(0, cards[0])));
        assert_eq!(side.facts.get(&six, "patch_cards").unwrap().defs(), [cards[2], cards[4], cards[0], cards[3]]);
        // (Cleared, all at once; an empty list, no change.)
        assert!(list(&six, side, "patch_cards", ListEdit::Clear) && !list(&six, side, "patch_cards", ListEdit::Clear));
        assert!(side.facts.get(&six, "patch_cards").unwrap().defs().is_empty());
    }

    #[test]
    fn sort_folder_moves_marks_and_pushes_empty_slots_to_end() {
        let mut before = saito_match::Folder::EMPTY;
        before.chips[1] = Some(FolderChip::new(ChipHandle(20), ChipCode(0)));
        before.chips[3] = Some(FolderChip::new(ChipHandle(10), ChipCode(1)));
        before.chips[5] = Some(FolderChip::new(ChipHandle(30), ChipCode(0)));
        before.regular = Some(1);
        before.tags = Some((3, 5));

        let (after, new_pos) = sort_folder(&before, |a, b| a.id.0.cmp(&b.id.0)).expect("unsorted folder changes");
        assert_eq!(after.chips[0].map(|c| c.id), Some(ChipHandle(10)));
        assert_eq!(after.chips[1].map(|c| c.id), Some(ChipHandle(20)));
        assert_eq!(after.chips[2].map(|c| c.id), Some(ChipHandle(30)));
        assert!(after.chips[3..].iter().all(Option::is_none));
        assert_eq!(new_pos[3], 0);
        assert_eq!(new_pos[1], 1);
        assert_eq!(new_pos[5], 2);
        assert_eq!(after.regular, Some(1));
        assert_eq!(after.tags, Some((0, 2)));
        assert!(sort_folder(&after, |a, b| a.id.0.cmp(&b.id.0)).is_none());
    }
}
