use super::packer::Packer;

#[test]
fn places_at_lowest_height_and_breaks_ties_left_to_right() {
    let mut packer = Packer::create([8, 8]);
    assert_eq!(packer.pack([4, 4]), Some([0, 0]));
    assert_eq!(packer.pack([4, 2]), Some([4, 0]));
    assert_eq!(packer.pack([4, 2]), Some([4, 2]));
    assert_eq!(packer.pack([8, 1]), Some([0, 4]));
}

#[test]
fn spanning_rectangles_clear_the_highest_column_underneath() {
    let mut packer = Packer::create([8, 8]);
    assert_eq!(packer.pack([3, 3]), Some([0, 0]));
    assert_eq!(packer.pack([2, 1]), Some([3, 0]));
    assert_eq!(packer.pack([4, 2]), Some([3, 1]));
    assert_eq!(packer.pack([8, 1]), Some([0, 3]));
}

#[test]
fn exact_fit_exhausts_the_atlas() {
    let mut packer = Packer::create([4, 3]);
    assert_eq!(packer.pack([4, 3]), Some([0, 0]));
    assert_eq!(packer.pack([1, 1]), None);
}

#[test]
fn unsuccessful_packs_leave_space_for_later_rectangles() {
    let mut packer = Packer::create([4, 4]);
    assert_eq!(packer.pack([2, 3]), Some([0, 0]));
    for dimensions in [[0, 1], [1, 0], [5, 1], [1, 5], [3, 2]] {
        assert_eq!(packer.pack(dimensions), None);
    }
    assert_eq!(packer.pack([2, 4]), Some([2, 0]));
    assert_eq!(packer.pack([2, 1]), Some([0, 3]));
}

#[test]
fn empty_atlases_cannot_fit_nonempty_rectangles() {
    for dimensions in [[0, 0], [0, 4], [4, 0]] {
        assert_eq!(Packer::create(dimensions).pack([1, 1]), None);
    }
}

#[test]
fn height_checks_do_not_overflow() {
    let mut packer = Packer::create([1, usize::MAX]);
    assert_eq!(packer.pack([1, usize::MAX - 1]), Some([0, 0]));
    assert_eq!(packer.pack([1, 2]), None);
    assert_eq!(packer.pack([1, 1]), Some([0, usize::MAX - 1]));
}

#[test]
fn successful_rectangles_stay_in_bounds_and_never_overlap() {
    let mut packer = Packer::create([16, 12]);
    let mut rectangles: Vec<([usize; 2], [usize; 2])> = Vec::new();
    for width in (1..=8).rev() {
        for height in (1..=6).rev() {
            if let Some([x, y]) = packer.pack([width, height]) {
                assert!(x + width <= 16 && y + height <= 12);
                for &([other_x, other_y], [other_width, other_height]) in &rectangles {
                    assert!(
                        x + width <= other_x
                            || other_x + other_width <= x
                            || y + height <= other_y
                            || other_y + other_height <= y
                    );
                }
                rectangles.push(([x, y], [width, height]));
            }
        }
    }
    assert!(!rectangles.is_empty());
}
