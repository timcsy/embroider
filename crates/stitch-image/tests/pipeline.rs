mod common;
use common::*;
use stitch_core::hoop::{builtin_hoops, Hoop};
use stitch_core::palette::threads;
use stitch_core::{validate_plan, StitchKind};
use stitch_image::{build_image_preview, ImageRequest};

fn hoop(id: &str) -> Hoop {
    builtin_hoops().into_iter().find(|h| h.id == id).unwrap()
}

fn request(img: &Img, k: usize, simplify: u32, size: f64, hoop_id: &str) -> ImageRequest {
    ImageRequest {
        rgba: img.rgba.clone(),
        width: img.w,
        height: img.h,
        color_count: k,
        simplify,
        size,
        hoop: hoop(hoop_id),
        settings: Default::default(),
    }
}

#[test]
fn 簡單圖案轉成三種線色且背景不繡() {
    let img = logo(96);
    let r = build_image_preview(&request(&img, 4, 30, 600.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 3, "{:?}", r.plan.colors);
    assert_eq!(r.plan.color_change_count(), 2);
    assert!(r.plan.stitch_count() > 500);
    validate_plan(&r.plan, &hoop("b70-medium")).unwrap();
    // 色名來自色票（中文）
    let palette = threads();
    for c in &r.plan.colors {
        assert!(palette.iter().any(|p| p.name == c.name), "{}", c.name);
    }
    assert!(r.adjustments.iter().all(|m| m.code != "photo_limited"));
}

#[test]
fn 紅色方塊繡在左上_藍色圓在右上_位置和圖一致() {
    let img = logo(96);
    let r = build_image_preview(&request(&img, 4, 30, 600.0, "b70-medium"));
    // 依序走訪：用顏色索引取出各色的針點範圍
    let mut color = 0usize;
    let mut bounds = vec![(i32::MAX, i32::MIN, i32::MAX, i32::MIN); r.plan.colors.len()];
    for s in &r.plan.stitches {
        if s.kind == StitchKind::ColorChange {
            color += 1;
        }
        if s.kind == StitchKind::Stitch {
            let b = &mut bounds[color];
            *b = (b.0.min(s.x), b.1.max(s.x), b.2.min(s.y), b.3.max(s.y));
        }
    }
    let idx = |name: &str| r.plan.colors.iter().position(|c| c.name == name).unwrap();
    let red = bounds[idx("正紅")];
    let blue = bounds[idx("寶藍")];
    // 紅方塊在左上：x、y 都偏負；藍圓在右上：x 偏正、y 偏負
    assert!(red.1 < blue.0 + 50, "紅在藍的左邊 {red:?} {blue:?}");
    assert!(red.3 < 0 && blue.3 < 0, "兩者都在上半部");
}

#[test]
fn 只有一種顏色的圖沒有東西可繡() {
    let img = Img::filled(32, 32, RED);
    let r = build_image_preview(&request(&img, 3, 30, 600.0, "b70-medium"));
    assert_eq!(r.blocked.unwrap().code, "single_color");
}

#[test]
fn 全透明的圖被阻止() {
    let mut img = Img::filled(8, 8, RED);
    for y in 0..8 {
        for x in 0..8 {
            img.set(x, y, RED, 0);
        }
    }
    let r = build_image_preview(&request(&img, 3, 30, 600.0, "b70-medium"));
    assert_eq!(r.blocked.unwrap().code, "empty_image");
}

#[test]
fn 透明背景的圖案直接繡圖案() {
    let mut img = Img::filled(64, 64, WHITE);
    for y in 0..64 {
        for x in 0..64 {
            img.set(x, y, WHITE, 0);
        }
    }
    img.disc(32.0, 32.0, 20.0, BLUE);
    let r = build_image_preview(&request(&img, 2, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 1);
}

#[test]
fn 顏色漸層很多的照片會提示效果有限但仍可繼續() {
    let mut img = Img::filled(64, 64, WHITE);
    for y in 0..64 {
        for x in 0..64 {
            img.set(
                x,
                y,
                [(x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8],
                255,
            );
        }
    }
    let r = build_image_preview(&request(&img, 6, 30, 600.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let m = r
        .adjustments
        .iter()
        .find(|m| m.code == "photo_limited")
        .expect("應提示效果有限");
    assert!(m.hint.is_some());
}

#[test]
fn 太細的線條被合併並說明() {
    let mut img = logo(96);
    // 幾條 1 像素寬的黑線橫過、縱過整張圖
    img.rect(0, 50, 96, 51, BLACK);
    img.rect(0, 56, 96, 57, BLACK);
    img.rect(20, 0, 21, 96, BLACK);
    let r = build_image_preview(&request(&img, 5, 30, 600.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert!(
        r.adjustments.iter().any(|m| m.code == "simplified"),
        "{:?}",
        r.adjustments
    );
}

#[test]
fn 簡化程度越高針越少() {
    // 兩種顏色隨機雜訊（固定亂數，結果可重現）
    let mut img = Img::filled(64, 64, WHITE);
    let mut seed = 12345u32;
    for y in 0..64 {
        for x in 0..64 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            if (seed >> 24).is_multiple_of(5) {
                img.set(x, y, BLUE, 255);
            }
        }
    }
    img.rect(10, 10, 40, 40, GREEN);
    let few = build_image_preview(&request(&img, 3, 90, 600.0, "b70-medium"));
    let many = build_image_preview(&request(&img, 3, 0, 600.0, "b70-medium"));
    assert!(few.blocked.is_none() && many.blocked.is_none());
    assert!(
        few.plan.stitches.len() < many.plan.stitches.len(),
        "簡化 90：{}，簡化 0：{}",
        few.plan.stitches.len(),
        many.plan.stitches.len()
    );
}

#[test]
fn 太大的圖會縮小放進小框並說明() {
    let img = logo(96);
    let r = build_image_preview(&request(&img, 4, 30, 2000.0, "b70-small"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert!(r.adjustments.iter().any(|m| m.code == "shrunk"));
    validate_plan(&r.plan, &hoop("b70-small")).unwrap();
}

#[test]
fn 顏色數不合法時回報白話錯誤() {
    let img = logo(32);
    let r = build_image_preview(&request(&img, 1, 30, 600.0, "b70-medium"));
    assert_eq!(r.blocked.unwrap().code, "bad_color_count");
}

#[test]
fn 簡化程度超過一百時當作一百() {
    let img = logo(64);
    let a = build_image_preview(&request(&img, 4, 100, 600.0, "b70-medium"));
    let b = build_image_preview(&request(&img, 4, 5000, 600.0, "b70-medium"));
    assert_eq!(a.plan, b.plan);
}

#[test]
fn 相同輸入得到相同輸出() {
    let img = logo(64);
    let a = build_image_preview(&request(&img, 4, 30, 600.0, "b70-medium"));
    let b = build_image_preview(&request(&img, 4, 30, 600.0, "b70-medium"));
    assert_eq!(a, b);
}

#[test]
fn 沒有被阻止時一律在安全範圍內() {
    let img = logo(80);
    for hoop_id in ["b70-small", "b70-medium", "b70-large"] {
        for size in [200.0, 600.0, 1800.0] {
            for k in [2usize, 4, 8] {
                let r = build_image_preview(&request(&img, k, 30, size, hoop_id));
                if r.blocked.is_none() {
                    validate_plan(&r.plan, &hoop(hoop_id))
                        .unwrap_or_else(|m| panic!("{hoop_id} {size} {k}: {m:?}"));
                }
            }
        }
    }
}

/// 卡通圖常見的「深色細輪廓線」：比最小線寬還細，也要留下來（加粗到剛好繡得出來），
/// 不能因為太細就整條消失，否則花瓣之間的分界會不見、全部糊成一團。
#[test]
fn 深色細輪廓線不會消失而是加粗() {
    let mut img = Img::filled(200, 200, WHITE);
    img.disc(100.0, 100.0, 80.0, BLACK);
    img.disc(100.0, 100.0, 78.0, RED); // 輪廓寬 2 像素；圖 50mm 寬 → 0.5mm，比最小線寬 0.8mm 細
    let r = build_image_preview(&request(&img, 3, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let dark = |c: &stitch_core::ThreadColor| (c.r as u32 + c.g as u32 + c.b as u32) < 200;
    assert!(
        r.plan.colors.iter().any(dark),
        "輪廓線消失了：{:?}",
        r.plan.colors.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// 圖案邊緣會有一圈「接近白色的過渡色」，它不該被當成另一種線色繡出來（像給圖案鑲一圈米白邊）。
#[test]
fn 接近背景的過渡色併入背景不繡() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(70, 40, 130, 160, [225, 240, 225]); // 過渡色的外框
    img.rect(75, 45, 125, 155, GREEN);
    let r = build_image_preview(&request(&img, 3, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(
        r.plan.colors.len(),
        1,
        "只該繡綠色：{:?}",
        r.plan.colors.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// 細的「淺色」過渡線只是邊緣的模糊，不是圖案的輪廓；只有深色細線才加粗保留。
#[test]
fn 淺色的細過渡線不會被加粗成一圈邊() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(70, 40, 130, 160, [170, 215, 170]); // 淺綠的 1 像素外框（夠淡，與綠色分成不同群）
    img.rect(71, 41, 129, 159, GREEN);
    let r = build_image_preview(&request(&img, 3, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(
        r.plan.colors.len(),
        1,
        "只該繡綠色：{:?}",
        r.plan.colors.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// 同一個深色裡有粗的區塊也有細線時，細線不能因為旁邊有粗的部分就被去掉。
#[test]
fn 深色同時有粗塊與細線時細線仍然保留() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(20, 20, 60, 60, BLACK); // 粗塊（左上）
    img.rect(100, 20, 102, 180, BLACK); // 細線：2 像素 = 0.5mm（圖 50mm 寬），在右側
    let r = build_image_preview(&request(&img, 2, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let (min_x, max_x) = r
        .plan
        .stitches
        .iter()
        .fold((i32::MAX, i32::MIN), |a, s| (a.0.min(s.x), a.1.max(s.x)));
    // 圖置中；細線在整張圖水平方向 50% 的位置，靠右半邊
    let mid = (min_x + max_x) / 2;
    assert!(
        r.plan.stitches.iter().any(|s| s.x > mid + 40),
        "細線不見了，針跡只剩左邊的粗塊：x 範圍 {min_x}..{max_x}"
    );
}

/// 顏色數抓得比實際多時，同一個色塊不會被切成好幾個幾乎一樣的線色。
#[test]
fn 顏色數過多時近似的顏色合併() {
    let img = logo(96); // 紅、藍、綠三個色塊加白底
    let r = build_image_preview(&request(&img, 12, 30, 600.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(
        r.plan.colors.len(),
        3,
        "{:?}",
        r.plan.colors.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// 色票只有 16 種時，中間亮度的棕色不能被配成灰色：配色要以「色相」為重，亮度為輕。
#[test]
fn 中間亮度的棕色配成咖啡色系而不是灰色() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(40, 40, 160, 160, [150, 100, 85]);
    let r = build_image_preview(&request(&img, 2, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 1);
    assert!(
        ["咖啡", "豆沙", "膚色", "杏色", "駝色"]
            .iter()
            .any(|n| r.plan.colors[0].name.contains(n)),
        "配成了「{}」",
        r.plan.colors[0].name
    );
}

/// 同一種線色不要分成好幾群各繡一次（換色、剪線都會變多）。
#[test]
fn 配到同一種線色的群合併成一次() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(30, 30, 100, 170, [60, 140, 50]); // 深一點的綠
    img.rect(100, 30, 170, 170, [95, 180, 75]); // 淺一點的綠，兩者配到同一種草綠
    let r = build_image_preview(&request(&img, 3, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(
        r.plan.colors.len(),
        1,
        "{:?}",
        r.plan.colors.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// 真實的照片（巧克力蛋糕，白底）：只有咖啡色系的深淺與奶油的白，
/// 配出來的線色不該有灰色或黑色大塊，也不該一堆顏色。
#[test]
fn 巧克力蛋糕照片配色是咖啡色系不出現灰色() {
    let rgba = std::fs::read("tests/fixtures/cake-160.rgba").unwrap();
    let img = Img {
        w: 160,
        h: 160,
        rgba,
    };
    let k = stitch_image::quantize::suggest_color_count(&img.rgba, img.w, img.h);
    let r = build_image_preview(&request(&img, k, 50, 1000.0, "b70-large"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let names: Vec<&str> = r.plan.colors.iter().map(|c| c.name.as_str()).collect();
    assert!(!names.contains(&"灰色"), "k={k} 出現灰色：{names:?}");
    assert!(names.len() <= 5, "顏色太多：{names:?}");
}

/// 相鄰的兩個色塊，下層要稍微延伸到上層底下，否則繡出來兩色之間會露出底布的縫。
#[test]
fn 相鄰色塊的針跡範圍互相重疊沒有縫() {
    let mut img = Img::filled(100, 100, WHITE);
    img.rect(10, 20, 50, 80, RED);
    img.rect(50, 20, 90, 80, BLUE);
    let r = build_image_preview(&request(&img, 3, 30, 400.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let mut color = 0usize;
    let mut bounds = vec![(i32::MAX, i32::MIN); r.plan.colors.len()];
    for s in &r.plan.stitches {
        if s.kind == StitchKind::ColorChange {
            color += 1;
        }
        if s.kind == StitchKind::Stitch {
            let b = &mut bounds[color];
            *b = (b.0.min(s.x), b.1.max(s.x));
        }
    }
    let idx = |name: &str| r.plan.colors.iter().position(|c| c.name == name).unwrap();
    let red = bounds[idx("正紅")];
    let blue = bounds[idx("寶藍")];
    // 單位 0.1mm；兩色的交界在 x=0，兩邊都要越過交界至少 0.2mm
    assert!(red.1 >= 2 && blue.0 <= -2, "紅 {:?} 藍 {:?}", red, blue);
}

/// 感恩節插畫：中間的「THANKS GIVING DAY!」是主角，字的筆畫細，不能被當成雜訊整個去掉。
/// 取中間字樣範圍內的針數：字若被去掉，只剩零星幾塊。
#[test]
fn 插畫中間的標題文字筆畫保留() {
    let rgba = std::fs::read("tests/fixtures/thanks-256.rgba").unwrap();
    let img = Img {
        w: 256,
        h: 171,
        rgba,
    };
    let r = build_image_preview(&request(&img, 6, 50, 1600.0, "b70-large"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    // 字樣約佔圖寬 27%～76%、圖高 25%～63%；圖以中心為原點，寬 1600（0.1mm）
    let (w, h) = (1600.0, 1600.0 * 171.0 / 256.0);
    let inside = r
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .filter(|s| {
            let (fx, fy) = (s.x as f64 / w + 0.5, s.y as f64 / h + 0.5);
            (0.27..0.76).contains(&fx) && (0.25..0.63).contains(&fy)
        })
        .count();
    eprintln!("字樣範圍內的針數 {inside}");
    assert!(
        inside > 3000,
        "字樣範圍內只有 {inside} 針，標題文字被吃掉了"
    );
}

/// 淡淡的薄荷綠（葉子）要配成綠色系，不是灰色。
#[test]
fn 淡薄荷綠的葉子配成綠色系而不是灰色() {
    let mut img = Img::filled(200, 200, WHITE);
    img.rect(40, 40, 160, 160, [190, 224, 203]);
    let r = build_image_preview(&request(&img, 2, 30, 500.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 1);
    assert!(
        r.plan.colors[0].name.contains('綠'),
        "配成了「{}」",
        r.plan.colors[0].name
    );
}
