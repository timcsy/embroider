use std::path::PathBuf;
use std::process::Command;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/tests/e2e/fixtures/logo.png")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_stitch-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn 圖片樣本產生所有格式與計畫檔() {
    let dir = std::env::temp_dir().join(format!("stitch-cli-image-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = run(&[
        "sample",
        "--image",
        fixture().to_str().unwrap(),
        "--hoop",
        "b70-medium",
        "--size",
        "600",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for name in ["LOGO0001.DST", "LOGO0001.EXP", "LOGO0001.plan.json"] {
        assert!(dir.join(name).exists(), "缺少 {name}");
    }
    let plan: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("LOGO0001.plan.json")).unwrap())
            .unwrap();
    assert_eq!(plan["colors"].as_array().unwrap().len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 讀不到的圖片回報錯誤() {
    let out = run(&["sample", "--image", "/nonexistent.png", "--out", "/tmp/x"]);
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());
}
