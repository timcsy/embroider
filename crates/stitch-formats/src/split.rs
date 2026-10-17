/// 把一個位移拆成多段，使每段在兩軸上都不超過 `max_step`，且各段加總等於原位移。
/// 零位移回傳空。
pub fn split_move(dx: i32, dy: i32, max_step: i32) -> Vec<(i32, i32)> {
    assert!(max_step > 0);
    let biggest = dx.abs().max(dy.abs());
    if biggest == 0 {
        return Vec::new();
    }
    let n = (biggest + max_step - 1) / max_step;
    let cut = |total: i32, i: i32| -> i32 {
        // 以四捨五入的累積位置相減，確保總和精確
        let at = |k: i32| ((total as i64 * k as i64) as f64 / n as f64).round() as i32;
        at(i + 1) - at(i)
    };
    (0..n).map(|i| (cut(dx, i), cut(dy, i))).collect()
}
