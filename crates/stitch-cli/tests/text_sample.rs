use std::path::PathBuf;
use std::process::Command;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fonts")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_stitch-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn 文字樣本產生所有格式與計畫檔() {
    let dir = std::env::temp_dir().join(format!("stitch-cli-text-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = run(&[
        "sample",
        "--text",
        "Amy 2026",
        "--hoop",
        "b70-medium",
        "--fonts",
        fonts_dir().to_str().unwrap(),
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for name in ["AMY20001.DST", "AMY20001.EXP", "AMY20001.plan.json"] {
        assert!(dir.join(name).exists(), "缺少 {name}");
    }
    assert!(std::fs::metadata(dir.join("AMY20001.DST")).unwrap().len() > 2000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 文字放不進框時失敗並說明() {
    let out = run(&[
        "sample",
        "--text",
        "王小明的刺繡作品展覽很長的一段文字",
        "--hoop",
        "b70-small",
        "--fonts",
        fonts_dir().to_str().unwrap(),
        "--out",
        "/tmp/should-not-exist",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("放不進"));
}
