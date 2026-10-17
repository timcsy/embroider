use std::time::Instant;
use stitch_core::hoop::builtin_hoops;
use stitch_core::ThreadColor;
use stitch_gen::fill::{fill, FillParams};
use stitch_gen::min_size::{min_scale_needed, MinSizeParams};
use stitch_gen::outline::outline;
use stitch_gen::underlay::underlay;
use stitch_text::{text_to_polygon, FontSet, Layout};

fn main() {
    let mut set = FontSet::default();
    set.add(
        "kai",
        "k",
        std::fs::read("fonts/LXGWWenKaiTC-subset.ttf").unwrap(),
    )
    .unwrap();
    let _ = (builtin_hoops(), ThreadColor::new(0, 0, 0, "x"));
    let text = "龍鳳呈祥龍鳳呈祥";
    let t = Instant::now();
    let l = text_to_polygon(&set, "kai", text, 250.0, Layout::SingleLine).unwrap();
    println!(
        "排版+字形: {:?}  rings={} pts={}",
        t.elapsed(),
        l.polygon.rings.len(),
        l.polygon.rings.iter().map(|r| r.len()).sum::<usize>()
    );
    let t = Instant::now();
    let n = l.polygon.normalized();
    println!("normalized: {:?}  rings={}", t.elapsed(), n.rings.len());
    let t = Instant::now();
    let s = min_scale_needed(&l.polygon, 250.0, &MinSizeParams::default());
    println!("min_scale_needed: {:?}  = {s:.2}", t.elapsed());
    let t = Instant::now();
    let off = l.polygon.offset(2.0);
    println!("offset(+2) 一次: {:?}", t.elapsed());
    let t = Instant::now();
    let parts = off.shapes();
    println!("shapes(): {:?} 形狀數={}", t.elapsed(), parts.len());
    let (mut tu, mut tf, mut to) = (0.0, 0.0, 0.0);
    for p in &parts {
        let t = Instant::now();
        let _ = underlay(p, 45.0);
        tu += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let _ = fill(p, &FillParams::default());
        tf += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let _ = outline(p, 25.0);
        to += t.elapsed().as_secs_f64();
    }
    println!(
        "底針合計 {:.0}ms, 面針合計 {:.0}ms, 輪廓合計 {:.0}ms",
        tu * 1e3,
        tf * 1e3,
        to * 1e3
    );
}
