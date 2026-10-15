mod common;

use std::path::Path;

use common::{tmp_root_with_carts, write_gb_cart};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{
    read_shelf_carts, read_slot_state, write_slot_state, Platform, Recent, SlotState,
    SHELF_CARTS_FILE,
};
use tempfile::TempDir;

fn card() -> TempDir {
    let d = tmp_root_with_carts(&["Drill Dozer", "Emerald", "Fusion"]);
    write_gb_cart(&d, "Kirby", "KIRBY");
    write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn boot_with(root: &Path, state: SlotState) -> App {
    write_slot_state(
        root,
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    App::boot(root)
}

fn settle(a: &mut App) {
    for _ in 0..30 {
        a.update(1.0 / 60.0);
    }
    a.flush_state();
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
    settle(a);
}

fn recent(platform: Platform, stem: &str) -> Recent {
    Recent {
        platform,
        stem: stem.into(),
    }
}

fn remembered(root: &Path) -> Vec<(String, Recent)> {
    read_shelf_carts(root)
}

#[test]
fn switching_shelves_comes_back_to_each_shelfs_cart_after_a_crash() {
    let d = card();
    let mut a = boot_with(d.path(), SlotState::default());
    press(&mut a, Btn::Right);
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    drop(a);

    let mut a = App::boot(d.path());
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
    assert_eq!(a.selected_stem(), Some("Tetris"));
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some("Game Boy Advance"));
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

#[test]
fn a_cart_at_rest_is_written_without_a_power_off() {
    let d = card();
    let mut a = boot_with(d.path(), SlotState::default());
    press(&mut a, Btn::Right);
    assert_eq!(
        remembered(d.path()),
        [("gba".to_string(), recent(Platform::Gba, "Emerald"))]
    );
    let state = read_slot_state(d.path());
    assert_eq!(state.shelf.as_deref(), Some("gba"));
    assert_eq!(state.shelf_cart, Some(recent(Platform::Gba, "Emerald")));
}

#[test]
fn a_held_direction_writes_once_it_is_let_go() {
    let d = card();
    let mut a = boot_with(d.path(), SlotState::default());
    settle(&mut a);
    a.apply(Action::GbaDown(Btn::Right));
    settle(&mut a);
    assert_eq!(
        remembered(d.path()),
        [("gba".to_string(), recent(Platform::Gba, "Drill Dozer"))],
        "the cart under a held direction was written"
    );
    a.apply(Action::GbaUp(Btn::Right));
    settle(&mut a);
    let stem = a.selected_stem().map(str::to_string).expect("a cart");
    assert_ne!(stem, "Drill Dozer");
    assert_eq!(
        remembered(d.path()),
        [("gba".to_string(), recent(Platform::Gba, &stem))]
    );
}

#[test]
fn recents_and_favorites_keep_their_own_cart() {
    let d = card();
    let mut a = boot_with(
        d.path(),
        SlotState {
            show_recents: true,
            recents: vec![
                recent(Platform::Gba, "Emerald"),
                recent(Platform::Gb, "Tetris"),
            ],
            favorites: vec![
                recent(Platform::Gba, "Drill Dozer"),
                recent(Platform::Gb, "Kirby"),
            ],
            ..SlotState::default()
        },
    );
    assert_eq!(a.shelf_platform_name(), Some("Recents"));
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some("Favorites"));
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Kirby"));
    press(&mut a, Btn::R1);
    drop(a);

    let mut a = App::boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some("Game Boy Advance"));
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some("Favorites"));
    assert_eq!(a.selected_stem(), Some("Kirby"));
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some("Recents"));
    assert_eq!(a.selected_stem(), Some("Tetris"));
}

#[test]
fn a_remembered_cart_the_card_no_longer_holds_starts_the_shelf_at_its_first_cart() {
    let d = card();
    std::fs::write(
        d.path().join("Config").join(SHELF_CARTS_FILE),
        "gb=gb/Deleted\ngba=gba/Fusion\n",
    )
    .unwrap();
    let mut a = boot_with(d.path(), SlotState::default());
    assert_eq!(a.selected_stem(), Some("Fusion"));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
    assert_eq!(a.selected_stem(), Some("Kirby"));
}

#[test]
fn a_seated_cart_still_resumes_over_the_shelf_memory() {
    let d = card();
    std::fs::write(
        d.path().join("Config").join(SHELF_CARTS_FILE),
        "gba=gba/Drill Dozer\ngb=gb/Tetris\n",
    )
    .unwrap();
    let a = boot_with(
        d.path(),
        SlotState {
            cart: Some("Fusion".into()),
            cart_platform: Some(Platform::Gba),
            shelf: Some("gb".into()),
            shelf_cart: Some(recent(Platform::Gb, "Tetris")),
            ..SlotState::default()
        },
    );
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "{:?}",
        a.phase()
    );
    assert_eq!(a.selected_stem(), Some("Fusion"));
}
