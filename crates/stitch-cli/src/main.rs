//! 開發用命令列工具：產生樣本檔，供獨立程式交叉驗證與除錯。不隨產品發佈。

use std::path::PathBuf;
use std::process::ExitCode;
use stitch_core::hoop::builtin_hoops;
use stitch_core::{validate_plan, Stitch, StitchPlan, ThreadColor};
use stitch_formats::Registry;
use stitch_wasm::api::{self, FontSet, LayoutSpec, TextRequest};

const USAGE: &str = "用法：stitch-cli sample (--shape square | --text <文字> | --image <png>) \
--hoop <框 id> --out <資料夾> [--fonts <字型資料夾>] [--font sans|serif|round|kai] [--height <0.1mm>] \
[--size <0.1mm>] [--colors <2-12>] [--simplify <0-100>]";

struct Args {
    shape: Option<String>,
    text: Option<String>,
    image: Option<PathBuf>,
    size: f64,
    colors: usize,
    simplify: u32,
    hoop: String,
    out: PathBuf,
    fonts: PathBuf,
    font: String,
    height: f64,
}

fn parse(args: &[String]) -> Result<Args, String> {
    if args.first().map(String::as_str) != Some("sample") {
        return Err(USAGE.to_string());
    }
    let mut a = Args {
        shape: None,
        text: None,
        image: None,
        size: 600.0,
        colors: 4,
        simplify: 30,
        hoop: "b70-small".into(),
        out: PathBuf::new(),
        fonts: PathBuf::from("fonts"),
        font: "sans".into(),
        height: 150.0,
    };
    let mut out = None;
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| format!("{flag} 後面缺少值。{USAGE}"))?;
        match flag.as_str() {
            "--shape" => a.shape = Some(value.clone()),
            "--text" => a.text = Some(value.clone()),
            "--image" => a.image = Some(PathBuf::from(value)),
            "--size" => {
                a.size = value
                    .parse()
                    .map_err(|_| format!("--size 要是數字：{value}"))?
            }
            "--colors" => {
                a.colors = value
                    .parse()
                    .map_err(|_| format!("--colors 要是整數：{value}"))?
            }
            "--simplify" => {
                a.simplify = value
                    .parse()
                    .map_err(|_| format!("--simplify 要是整數：{value}"))?
            }
            "--hoop" => a.hoop = value.clone(),
            "--out" => out = Some(PathBuf::from(value)),
            "--fonts" => a.fonts = PathBuf::from(value),
            "--font" => a.font = value.clone(),
            "--height" => {
                a.height = value
                    .parse()
                    .map_err(|_| format!("--height 要是數字：{value}"))?
            }
            other => return Err(format!("不認得的參數 {other}。{USAGE}")),
        }
    }
    a.out = out.ok_or_else(|| format!("缺少 --out。{USAGE}"))?;
    if a.shape.is_none() && a.text.is_none() && a.image.is_none() {
        a.shape = Some("square".into());
    }
    Ok(a)
}

/// 邊長 30mm 的方形外框（每邊超過單針上限，會被拆分），中間換色後畫一條對角線。
fn square_plan() -> StitchPlan {
    let h = 150;
    let mut s = vec![
        Stitch::jump(-h, -h),
        Stitch::stitch(h, -h),
        Stitch::stitch(h, h),
        Stitch::stitch(-h, h),
        Stitch::stitch(-h, -h),
        Stitch::color_change(-h, -h),
        // 終點刻意不對稱，才能偵測 Y 軸方向是否反了
        Stitch::stitch(h, 60),
    ];
    s.push(Stitch::end(h, 60));
    StitchPlan {
        stitches: s,
        colors: vec![
            ThreadColor::new(0, 0, 0, "黑"),
            ThreadColor::new(200, 30, 30, "正紅"),
        ],
    }
}

fn load_fonts(dir: &std::path::Path) -> Result<FontSet, String> {
    let mut set = FontSet::default();
    for (id, name, file) in [
        ("sans", "黑體", "NotoSansTC-subset.ttf"),
        ("serif", "宋體", "NotoSerifTC-subset.ttf"),
        ("round", "圓體", "jf-openhuninn-subset.ttf"),
        ("kai", "楷體", "LXGWWenKaiTC-subset.ttf"),
    ] {
        let path = dir.join(file);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("讀不到字型 {}：{e}", path.display()))?;
        set.add(id, name, bytes).map_err(|m| m.text)?;
    }
    Ok(set)
}

fn load_png(path: &std::path::Path) -> Result<(Vec<u8>, usize, usize), String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("讀不到圖片 {}：{e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("圖片打不開：{e}"))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("圖片打不開：{e}"))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()]
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => buf[..info.buffer_size()]
            .iter()
            .flat_map(|&g| [g, g, g, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..info.buffer_size()]
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        other => return Err(format!("不支援的 PNG 格式：{other:?}")),
    };
    Ok((rgba, w, h))
}

fn run(args: &[String]) -> Result<(), String> {
    let a = parse(args)?;
    let hoop = builtin_hoops()
        .into_iter()
        .find(|h| h.id == a.hoop)
        .ok_or_else(|| {
            format!(
                "找不到框 {}。可用：b70-large、b70-medium、b70-small。",
                a.hoop
            )
        })?;

    let (plan, name) = if let Some(path) = &a.image {
        let (rgba, width, height) = load_png(path)?;
        let req = api::ImageRequest {
            rgba,
            width,
            height,
            color_count: a.colors,
            simplify: a.simplify,
            size: a.size,
            hoop: hoop.clone(),
            settings: Default::default(),
        };
        let r = api::build_image_preview(&req);
        if let Some(m) = r.blocked {
            return Err(format!("{} {}", m.text, m.hint.unwrap_or_default()));
        }
        for m in &r.adjustments {
            eprintln!("提示：{}", m.text);
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        (r.plan, stem)
    } else if let Some(text) = &a.text {
        let fonts = load_fonts(&a.fonts)?;
        let req = TextRequest {
            text: text.clone(),
            font_id: a.font.clone(),
            height: a.height,
            color: ThreadColor::new(200, 30, 30, "正紅"),
            layout: LayoutSpec::SingleLine,
            hoop: hoop.clone(),
            settings: Default::default(),
        };
        let r = api::build_text_preview(&fonts, &req);
        if let Some(m) = r.blocked {
            return Err(format!("{} {}", m.text, m.hint.unwrap_or_default()));
        }
        for m in &r.adjustments {
            eprintln!("提示：{}", m.text);
        }
        (r.plan, text.clone())
    } else {
        match a.shape.as_deref() {
            Some("square") => (square_plan(), "SQUARE".to_string()),
            other => return Err(format!("不認得的形狀 {other:?}。")),
        }
    };

    validate_plan(&plan, &hoop).map_err(|m| m.text)?;
    std::fs::create_dir_all(&a.out).map_err(|e| e.to_string())?;
    // 方形樣本沿用固定檔名（SQUARE.DST）；文字樣本使用正式的檔名規則
    let stem;
    if a.text.is_some() || a.image.is_some() {
        let files = api::encode_all(&plan, &hoop, &[], &name, 1).map_err(|m| m.text)?;
        for f in &files {
            let path = a.out.join(&f.file_name);
            std::fs::write(&path, &f.bytes).map_err(|e| e.to_string())?;
            println!("{}", path.display());
        }
        stem = files[0].file_name.split('.').next().unwrap().to_string();
    } else {
        stem = name.to_uppercase();
        for f in Registry::default().list() {
            let bytes = f.encode(&plan, &stem).map_err(|e| e.to_string())?;
            let path = a.out.join(format!("{stem}.{}", f.extension()));
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            println!("{}", path.display());
        }
    }
    // 同時寫出原始計畫，供驗證腳本逐點比對
    let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
    std::fs::write(a.out.join(format!("{stem}.plan.json")), json).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("{m}");
            ExitCode::FAILURE
        }
    }
}
