"""把來源字型子集化成可內嵌的小檔，輸出到 fonts/。

用法：
    uv run --no-project --with fonttools python -I tools/subset_fonts.py <來源字型資料夾>

來源資料夾需含：NotoSansTC.ttf、NotoSerifTC.ttf（可變字型）、jf-openhuninn.ttf、LXGWWenKaiTC.ttf
字元範圍：Big5 第一級常用漢字（5401 字）、ASCII、全形與中文標點。
授權：四個字型皆為 SIL OFL 1.1，允許嵌入、子集化與散布（見 fonts/LICENSES/）。
"""
import os
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "fonts")

# (來源檔, 輸出檔, 可變字型要固定的字重)
JOBS = [
    ("NotoSansTC.ttf", "NotoSansTC-subset.ttf", 500),
    ("NotoSerifTC.ttf", "NotoSerifTC-subset.ttf", 500),
    ("jf-openhuninn.ttf", "jf-openhuninn-subset.ttf", None),
    ("LXGWWenKaiTC.ttf", "LXGWWenKaiTC-subset.ttf", None),
]

EXTRA = (
    "".join(chr(c) for c in range(0x20, 0x7F))
    + "，。、：；！？「」『』（）—…～·《》〈〉【】﹁﹂％＆＠＃＋－＝／０１２３４５６７８９"
    + "ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ"
)


def big5_hanzi():
    """Big5 第一級常用漢字（A440–C67E，約 5401 字）。"""
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


def build(src_dir):
    text = "".join(sorted(big5_hanzi())) + EXTRA
    for src, dst, weight in JOBS:
        font = TTFont(os.path.join(src_dir, src))
        if "fvar" in font:
            font = instancer.instantiateVariableFont(font, {"wght": weight})
        opts = subset.Options()
        opts.layout_features = []
        opts.hinting = False
        opts.glyph_names = False
        opts.notdef_outline = True
        opts.name_IDs = ["*"]
        opts.drop_tables += ["DSIG", "GSUB", "GPOS", "GDEF", "kern", "STAT", "vhea", "vmtx"]
        sub = subset.Subsetter(opts)
        sub.populate(text=text)
        sub.subset(font)
        out = os.path.join(OUT_DIR, dst)
        font.save(out)
        print(f"{dst}: {os.path.getsize(out) / 1024 / 1024:.2f} MB")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    build(sys.argv[1])
