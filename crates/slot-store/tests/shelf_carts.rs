mod common;

use common::tmp_root;
use slot_store::{read_shelf_carts, write_shelf_carts, Platform, Recent, SHELF_CARTS_FILE};

fn kept(key: &str, platform: Platform, stem: &str) -> (String, Recent) {
    (
        key.into(),
        Recent {
            platform,
            stem: stem.into(),
        },
    )
}

#[test]
fn each_shelf_keeps_its_cart_across_a_write_and_a_read() {
    let d = tmp_root();
    let carts = vec![
        kept("gba", Platform::Gba, "Drill Dozer"),
        kept("recents", Platform::Gb, "Tetris"),
        kept("favorites", Platform::Gbc, "Zelda = DX/Rev 1"),
    ];
    write_shelf_carts(d.path(), &carts).unwrap();
    let text = std::fs::read_to_string(d.path().join("Config").join(SHELF_CARTS_FILE)).unwrap();
    assert_eq!(
        text,
        "gba=gba/Drill Dozer\nrecents=gb/Tetris\nfavorites=gbc/Zelda = DX/Rev 1\n"
    );
    assert_eq!(read_shelf_carts(d.path()), carts);
}

#[test]
fn a_card_with_no_shelf_carts_file_remembers_nothing() {
    let d = tmp_root();
    assert!(read_shelf_carts(d.path()).is_empty());
    write_shelf_carts(d.path(), &[]).unwrap();
    assert!(!d.path().join("Config").join(SHELF_CARTS_FILE).exists());
}

#[test]
fn damaged_and_repeated_lines_are_skipped() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config").join(SHELF_CARTS_FILE),
        "gba=gba/Fusion\ngba=gba/Emerald\ngb=xyz/Tetris\nno equals sign\n=gba/Emerald\ngb=gb/Tetris\n",
    )
    .unwrap();
    assert_eq!(
        read_shelf_carts(d.path()),
        [
            kept("gba", Platform::Gba, "Fusion"),
            kept("gb", Platform::Gb, "Tetris")
        ]
    );
}
