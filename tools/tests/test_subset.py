"""字型子集化的驗收測試：
- 涵蓋 Big5 常用字（第一級 5401 字）加英數與常見標點
- 每個檔案大小在上限內
執行：uv run --no-project --with fonttools python -I tools/tests/test_subset.py
"""
import os
import unittest

from fontTools.ttLib import TTFont

FONT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "fonts")
FONTS = [
    "NotoSansTC-subset.ttf",
    "NotoSerifTC-subset.ttf",
    "jf-openhuninn-subset.ttf",
    "LXGWWenKaiTC-subset.ttf",
]
MAX_BYTES = 4 * 1024 * 1024


def big5_level1():
    chars = set()
    for hi in range(0xA4, 0xC7):
        for lo in list(range(0x40, 0x7F)) + list(range(0xA1, 0xFF)):
            try:
                c = bytes([hi, lo]).decode("big5")
            except UnicodeDecodeError:
                continue
            if len(c) == 1 and "一" <= c <= "鿿":
                chars.add(c)
    return chars


class SubsetTest(unittest.TestCase):
    def path(self, name):
        return os.path.join(FONT_DIR, name)

    def test_files_exist_and_within_size_limit(self):
        for name in FONTS:
            p = self.path(name)
            self.assertTrue(os.path.exists(p), f"缺少 {name}")
            self.assertLessEqual(os.path.getsize(p), MAX_BYTES, f"{name} 太大")

    def test_covers_common_traditional_chars(self):
        wanted = big5_level1()
        self.assertGreater(len(wanted), 5000)
        for name in FONTS:
            cmap = TTFont(self.path(name), lazy=True).getBestCmap()
            missing = [c for c in wanted if ord(c) not in cmap]
            # 字型本身可能缺少少數字；容許 1% 以內
            self.assertLessEqual(len(missing), len(wanted) * 0.01, f"{name} 缺 {len(missing)} 字：{missing[:10]}")

    def test_covers_ascii_and_punctuation(self):
        sample = "ABCxyz0123456789 .,:;!?-_()[]/&@#%+=，。、：；！？「」『』（）—…～"
        for name in FONTS:
            cmap = TTFont(self.path(name), lazy=True).getBestCmap()
            missing = [c for c in sample if ord(c) not in cmap]
            self.assertEqual(missing, [], f"{name} 缺 {missing}")

    def test_is_static_truetype(self):
        for name in FONTS:
            f = TTFont(self.path(name), lazy=True)
            self.assertIn("glyf", f, f"{name} 不是 TrueType 輪廓")
            self.assertNotIn("fvar", f, f"{name} 仍是可變字型")


if __name__ == "__main__":
    unittest.main()
