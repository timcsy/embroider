use stitch_core::hoop::builtin_hoops;
use stitch_core::validate_plan;
use stitch_wasm::api;

fn logo_rgba(size: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..size {
        for x in 0..size {
            let c: [u8; 3] = if x < size / 2 && y < size / 2 {
                [200, 30, 40]
            } else if x >= size / 2 && y >= size / 2 {
                [40, 80, 180]
            } else {
                [255, 255, 255]
            };
            v.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    v
}

fn request(rgba: Vec<u8>, size: usize) -> api::ImageRequest {
    api::ImageRequest {
        rgba,
        width: size,
        height: size,
        color_count: 3,
        simplify: 30,
        size: 600.0,
        hoop: builtin_hoops().remove(1),
        settings: Default::default(),
    }
}

#[test]
fn 圖片預覽成功時計畫合法() {
    let r = api::build_image_preview(&request(logo_rgba(64), 64));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 2);
    validate_plan(&r.plan, &builtin_hoops()[1]).unwrap();
}

#[test]
fn 建議的顏色數在範圍內() {
    let k = api::suggest_color_count(&logo_rgba(64), 64, 64);
    assert!((2..=8).contains(&k));
}

#[test]
fn 色票有十六色且都有中文色名() {
    let t = api::list_thread_colors();
    assert_eq!(t.len(), 16);
    assert!(t.iter().all(|c| c
        .name
        .chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))));
}

#[test]
fn 請求可以從_json_還原_像素另外傳入() {
    let json = r#"{"width":4,"height":4,"color_count":3,"simplify":30,"size":600.0,
        "hoop":{"id":"h","name":"h","width":1200,"height":1800,"shape":"rect","margin":50,"builtin":false}}"#;
    let req: api::ImageRequest = serde_json::from_str(json).unwrap();
    assert!(req.rgba.is_empty());
    assert_eq!(req.width, 4);
}
