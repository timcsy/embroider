use proptest::prelude::*;
use stitch_formats::split::split_move;

#[test]
fn 小位移不拆分() {
    assert_eq!(split_move(10, -5, 121), vec![(10, -5)]);
}

#[test]
fn 零位移回傳空() {
    assert!(split_move(0, 0, 121).is_empty());
}

#[test]
fn 剛好等於上限不拆分() {
    assert_eq!(split_move(121, 0, 121), vec![(121, 0)]);
    assert_eq!(split_move(0, -121, 121), vec![(0, -121)]);
}

#[test]
fn 超過上限一點就拆成兩段() {
    let parts = split_move(122, 0, 121);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts.iter().map(|p| p.0).sum::<i32>(), 122);
}

#[test]
fn 長位移拆分後每段不超過上限且總和不變() {
    let parts = split_move(1000, -333, 121);
    assert!(parts.iter().all(|&(x, y)| x.abs() <= 121 && y.abs() <= 121));
    assert_eq!(parts.iter().map(|p| p.0).sum::<i32>(), 1000);
    assert_eq!(parts.iter().map(|p| p.1).sum::<i32>(), -333);
    assert_eq!(parts.len(), 9); // ceil(1000 / 121)
}

proptest! {
    #[test]
    fn 任意位移拆分後每段不超過上限且總和等於原位移(
        dx in -5000i32..5000, dy in -5000i32..5000, max in prop::sample::select(vec![121i32, 127])
    ) {
        let parts = split_move(dx, dy, max);
        prop_assert_eq!(parts.iter().map(|p| p.0).sum::<i32>(), dx);
        prop_assert_eq!(parts.iter().map(|p| p.1).sum::<i32>(), dy);
        for (x, y) in parts {
            prop_assert!(x.abs() <= max && y.abs() <= max);
        }
    }
}
