use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_stitch-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn 方形樣本會產生_dst_與_exp() {
    let dir = std::env::temp_dir().join(format!("stitch-cli-sample-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = run(&[
        "sample",
        "--shape",
        "square",
        "--hoop",
        "b70-small",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dst = std::fs::read(dir.join("SQUARE.DST")).unwrap();
    let exp = std::fs::read(dir.join("SQUARE.EXP")).unwrap();
    assert!(dst.len() > 512);
    assert!(!exp.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 未知的框會失敗並說明() {
    let out = run(&[
        "sample", "--shape", "square", "--hoop", "nope", "--out", "/tmp/x",
    ]);
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());
}

#[test]
fn 沒有子命令時顯示用法() {
    let out = run(&[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("用法"));
}
