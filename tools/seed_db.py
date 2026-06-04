#!/usr/bin/env python3
"""Seed the EN->CN mini dictionary fixture used by storage / dictionary tests.

Schema (unchanged across iter-011): one row per English headword, with
`definitions` carrying the Chinese senses as a JSON array of strings.
Re-running this script must produce a byte-identical SQLite file when the
WORDS list below is unchanged, so test fixtures are reproducible:

    python tools/seed_db.py [out_path]

Default out_path: tests/fixtures/mini_dict.sqlite (the canonical fixture).
"""
from __future__ import annotations

import json
import sqlite3
import sys
from pathlib import Path

# Hand-curated 80-entry subset. Each (headword, phonetic, [zh definitions...]).
# Keep the list deterministic — golden tests downstream depend on it.
WORDS: list[tuple[str, str, list[str]]] = [
    ("apple",    "/ˈæp.əl/",    ["苹果", "苹果树"]),
    ("apply",    "/əˈplaɪ/",    ["申请", "应用，运用"]),
    ("ample",    "/ˈæm.pəl/",   ["充足的，丰富的"]),
    ("banana",   "/bəˈnɑː.nə/", ["香蕉"]),
    ("band",     "/bænd/",      ["乐队", "带子，条带"]),
    ("book",     "/bʊk/",       ["书，书籍", "预订，预约"]),
    ("boot",     "/buːt/",      ["靴子", "启动（计算机）"]),
    ("cat",      "/kæt/",       ["猫"]),
    ("car",      "/kɑːr/",      ["汽车，轿车"]),
    ("code",     "/koʊd/",      ["代码，编码", "密码"]),
    ("dog",      "/dɒɡ/",       ["狗，犬"]),
    ("door",     "/dɔːr/",      ["门"]),
    ("easy",     "/ˈiː.zi/",    ["容易的，轻松的"]),
    ("east",     "/iːst/",      ["东方", "向东的"]),
    ("english",  "/ˈɪŋ.ɡlɪʃ/",  ["英语", "英国的"]),
    ("fast",     "/fæst/",      ["快的，迅速的", "斋戒"]),
    ("file",     "/faɪl/",      ["文件", "文件夹", "锉刀"]),
    ("flash",    "/flæʃ/",      ["闪光", "闪现"]),
    ("good",     "/ɡʊd/",       ["好的，良好的", "善行"]),
    ("great",    "/ɡreɪt/",     ["伟大的", "极好的"]),
    ("hello",    "/həˈloʊ/",    ["你好", "（电话用语）喂"]),
    ("house",    "/haʊs/",      ["房子，住宅"]),
    ("ice",      "/aɪs/",       ["冰", "冰镇"]),
    ("idea",     "/aɪˈdɪə/",    ["想法，主意", "概念"]),
    ("join",     "/dʒɔɪn/",     ["加入，参加", "连接"]),
    ("jump",     "/dʒʌmp/",     ["跳跃", "猛涨"]),
    ("key",      "/kiː/",       ["钥匙", "关键的", "按键"]),
    ("kind",     "/kaɪnd/",     ["友好的，仁慈的", "种类"]),
    ("learn",    "/lɜːrn/",     ["学习", "获悉"]),
    ("light",    "/laɪt/",      ["光，光线", "轻的"]),
    ("map",      "/mæp/",       ["地图", "映射"]),
    ("music",    "/ˈmjuːzɪk/",  ["音乐"]),
    ("night",    "/naɪt/",      ["夜晚，夜间"]),
    ("note",     "/noʊt/",      ["笔记，便条", "音符", "注意"]),
    ("open",     "/ˈoʊpən/",    ["打开", "开放的"]),
    ("orange",   "/ˈɒrɪndʒ/",   ["橙子", "橙色"]),
    ("paper",    "/ˈpeɪpər/",   ["纸", "论文", "报纸"]),
    ("phone",    "/foʊn/",      ["电话", "打电话"]),
    ("query",    "/ˈkwɪər.i/",  ["疑问，询问", "查询"]),
    ("quick",    "/kwɪk/",      ["快的，迅速的"]),
    ("read",     "/riːd/",      ["阅读，读"]),
    ("right",    "/raɪt/",      ["正确的", "右边的", "权利"]),
    ("see",      "/siː/",       ["看见", "明白"]),
    ("sun",      "/sʌn/",       ["太阳", "晒太阳"]),
    ("teach",    "/tiːtʃ/",     ["教，教授"]),
    ("time",     "/taɪm/",      ["时间", "次，倍"]),
    ("under",    "/ˈʌndər/",    ["在……下面", "在……之下"]),
    ("use",      "/juːz/",      ["使用", "用途"]),
    ("very",     "/ˈvɛri/",     ["非常，很", "确实的"]),
    ("water",    "/ˈwɔːtər/",   ["水", "给……浇水"]),
    ("word",     "/wɜːrd/",     ["单词，词语", "诺言"]),
    ("write",    "/raɪt/",      ["写，书写", "写作"]),
    ("year",     "/jɪər/",      ["年，年份"]),
    ("zero",     "/ˈzɪroʊ/",    ["零", "零点"]),
    ("hello",    "/həˈloʊ/",    ["你好", "（电话用语）喂"]),  # dup -> dedup later
    ("table",    "/ˈteɪ.bəl/",  ["桌子，餐桌", "表格"]),
    ("chair",    "/tʃɛər/",     ["椅子", "主席"]),
    ("computer", "/kəmˈpjuː.tər/", ["计算机，电脑"]),
    ("software", "/ˈsɔːft.wɛr/",   ["软件"]),
    ("hardware", "/ˈhɑːrd.wɛr/",   ["硬件", "五金器具"]),
    ("language", "/ˈlæŋ.ɡwɪdʒ/",   ["语言"]),
    ("friend",   "/frɛnd/",        ["朋友"]),
    ("family",   "/ˈfæm.ə.li/",    ["家庭", "家人"]),
    ("school",   "/skuːl/",        ["学校"]),
    ("student",  "/ˈstuː.dənt/",   ["学生"]),
    ("teacher",  "/ˈtiː.tʃər/",    ["老师，教师"]),
    ("travel",   "/ˈtræv.əl/",     ["旅行", "行进"]),
    ("country",  "/ˈkʌn.tri/",     ["国家", "乡村"]),
    ("city",     "/ˈsɪt.i/",       ["城市"]),
    ("street",   "/striːt/",       ["街道"]),
    ("river",    "/ˈrɪv.ər/",      ["河，河流"]),
    ("mountain", "/ˈmaʊn.tɪn/",    ["山，山脉"]),
    ("ocean",    "/ˈoʊ.ʃən/",      ["海洋"]),
    ("morning",  "/ˈmɔːr.nɪŋ/",    ["早晨，上午"]),
    ("evening",  "/ˈiːv.nɪŋ/",     ["傍晚，晚上"]),
    ("today",    "/təˈdeɪ/",       ["今天，今日"]),
    ("tomorrow", "/təˈmɒr.oʊ/",    ["明天", "未来"]),
    ("yesterday","/ˈjɛs.tər.deɪ/", ["昨天"]),
    ("happy",    "/ˈhæp.i/",       ["快乐的，幸福的"]),
    ("sad",      "/sæd/",          ["悲伤的，难过的"]),
    ("love",     "/lʌv/",          ["爱，喜爱", "爱情"]),
    ("hate",     "/heɪt/",         ["憎恨，讨厌"]),
    ("speak",    "/spiːk/",        ["说，讲", "演讲"]),
    ("listen",   "/ˈlɪs.ən/",      ["听，倾听"]),
    ("question", "/ˈkwɛs.tʃən/",   ["问题", "怀疑"]),
    ("answer",   "/ˈæn.sər/",      ["回答", "答案"]),
]

SCHEMA = """\
PRAGMA user_version = 2;
CREATE TABLE IF NOT EXISTS entries (
    headword    TEXT PRIMARY KEY COLLATE NOCASE,
    phonetic    TEXT NOT NULL DEFAULT '',
    definitions TEXT NOT NULL  -- JSON array of Chinese gloss strings
);
CREATE INDEX IF NOT EXISTS idx_entries_headword_nocase
    ON entries(headword COLLATE NOCASE);
"""


def main(argv: list[str]) -> int:
    out = Path(argv[1]) if len(argv) > 1 else Path("tests/fixtures/mini_dict.sqlite")
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()

    # De-duplicate by headword (case-insensitive); last definition wins.
    by_word: dict[str, tuple[str, str, list[str]]] = {}
    for w, p, d in WORDS:
        by_word[w.lower()] = (w, p, d)
    rows = list(by_word.values())

    conn = sqlite3.connect(out)
    try:
        conn.executescript(SCHEMA)
        conn.executemany(
            "INSERT INTO entries(headword, phonetic, definitions) VALUES(?, ?, ?);",
            [(w, p, json.dumps(d, ensure_ascii=False)) for w, p, d in rows],
        )
        conn.commit()
    finally:
        conn.close()

    print(f"Wrote {len(rows)} EN->CN entries to {out} ({out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
