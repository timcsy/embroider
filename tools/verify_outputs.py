"""以獨立實作（pyembroidery）讀回輸出檔，印出摘要 JSON，並與核心的原始計畫逐點比對。

用法：
    uv run --no-project --with pyembroidery python -I tools/verify_outputs.py <資料夾>

資料夾內若有 `<名稱>.plan.json`（由 stitch-cli 寫出），會檢查同名的每個 .DST／.EXP：
計畫中的每個「一般針／跳針」終點，必須依序出現在讀回的針跡中。
只證明「檔案格式正確、內容與核心一致」，不證明繡機能讀（見 contracts/output-files.md）。
"""
import json
import os
import sys

import pyembroidery


def read_pattern(path):
    pattern = pyembroidery.read(path)
    if pattern is None:
        raise ValueError(f"無法讀取：{path}")
    return pattern


def summarize(path, pattern):
    stitches = pattern.stitches
    xs = [s[0] for s in stitches]
    ys = [s[1] for s in stitches]
    cmds = [s[2] & pyembroidery.COMMAND_MASK for s in stitches]
    return {
        "file": os.path.basename(path),
        "stitch_count": sum(1 for c in cmds if c == pyembroidery.STITCH),
        "jump_count": sum(1 for c in cmds if c == pyembroidery.JUMP),
        "trim_count": sum(1 for c in cmds if c == pyembroidery.TRIM),
        "color_changes": sum(1 for c in cmds if c == pyembroidery.COLOR_CHANGE),
        "min_x": min(xs) if xs else 0,
        "max_x": max(xs) if xs else 0,
        "min_y": min(ys) if ys else 0,
        "max_y": max(ys) if ys else 0,
    }


def compare_with_plan(pattern, plan):
    """計畫的每個 Stitch／Jump 終點，必須依序出現在讀回的針跡中。回傳錯誤訊息或 None。"""
    decoded = [
        (s[0], s[1])
        for s in pattern.stitches
        if (s[2] & pyembroidery.COMMAND_MASK) in (pyembroidery.STITCH, pyembroidery.JUMP)
    ]
    expected = [(s["x"], s["y"]) for s in plan["stitches"] if s["kind"] in ("Stitch", "Jump")]
    i = 0
    for point in decoded:
        if i < len(expected) and point == expected[i]:
            i += 1
    if i != len(expected):
        return f"第 {i + 1} 個終點 {expected[i]} 沒有出現在讀回的針跡中"
    changes = sum(1 for s in plan["stitches"] if s["kind"] == "ColorChange")
    got = sum(
        1 for s in pattern.stitches if (s[2] & pyembroidery.COMMAND_MASK) == pyembroidery.COLOR_CHANGE
    )
    if changes != got:
        return f"換色次數不一致：計畫 {changes}，讀回 {got}"
    return None


def main():
    if len(sys.argv) != 2:
        print("用法：verify_outputs.py <資料夾>", file=sys.stderr)
        return 2
    folder = sys.argv[1]
    results, errors = [], []
    for name in sorted(os.listdir(folder)):
        if not name.lower().endswith((".dst", ".exp")):
            continue
        path = os.path.join(folder, name)
        pattern = read_pattern(path)
        results.append(summarize(path, pattern))
        plan_path = os.path.join(folder, os.path.splitext(name)[0] + ".plan.json")
        if os.path.exists(plan_path):
            with open(plan_path, encoding="utf-8") as f:
                problem = compare_with_plan(pattern, json.load(f))
            if problem:
                errors.append(f"{name}：{problem}")
    print(json.dumps(results, ensure_ascii=False, indent=2))
    for e in errors:
        print("驗證失敗：" + e, file=sys.stderr)
    return 0 if results and not errors else 1


if __name__ == "__main__":
    sys.exit(main())
