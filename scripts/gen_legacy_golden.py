"""Снимает эталон поведения filter_text с текущего (2.3.1) дерева.

Словарь фиксированный и синтетический: эталон должен фиксировать поведение
ДВИЖКА, а не содержимое словарей, иначе он развалится при их пополнении.
Дёргается _native напрямую, минуя коэрцию аргументов в python/badwords.
"""

import json
from pathlib import Path

from badwords._native import PyProfanityFilter

RESOURCE_DIR = str(Path("python/badwords/resource").resolve())

DICT = [
    # английский: база, словоформы, слитные
    "fuck", "fucking", "fucked", "fucker", "shit", "shitty", "bitch", "bitches",
    "asshole", "dick", "dickhead", "cock", "cunt", "pussy", "bastard", "damn",
    "sonofabitch", "motherfucker", "a55", "5hit", "f_uc_k",
    # кириллица
    "хуй", "бля", "сука", "пизда", "ебать", "мудак",
    # фразы (сейчас мертвы, но должны остаться мертвы для legacy)
    "kill your self", "son of a bitch",
    # прочее
    "spam_link", "scam_bot", "custombad", "badword", "verylongprofanityword",
    "ass", "as", "x",
]

CLEAN = [
    "hello", "world", "hello world", "the quick brown fox", "classic", "assassin",
    "grass", "analysis", "cocktail", "password", "assignment", "bass", "class",
    "document", "function", "competition", "scunthorpe", "disk", "sock", "sum",
    "summer", "soon", "slit", "fuss", "pisces", "привет", "доброе утро",
    "программирование", "", "   ", "\t", "\n", "  hello  ", "a", "ab",
]

PUNCT = [
    "fuck", "fuck.", "fuck,", "fuck!", "fuck!!!", "(fuck)", "[fuck]", "\"fuck\"",
    "'fuck'", "...fuck...", "fuck?", "fuck;", "fuck:", "-fuck-", "_fuck_",
    "f.u.c.k", "f-u-c-k", "f_u_c_k", "f u c k", "you.fuck", "fuck-you", "fuck.you",
    "fuck/you", "hey fuck, ok", "well, fuck.", "a fuck b", "fuck fuck", "fuckfuck",
    "don't", "it's fuck", "co-op", "e.g.", "U.S.A.", "1,000", "3.14",
]

UNICODE = [
    "fuсk", "fυck", "𝔣𝔲𝔠𝔨", "ｆｕｃｋ", "ⓕⓤⓒⓚ", "🅵🅄🅲🅺", "fцск", "diсk", "shіtty",
    "FUCK", "FuCk", "fUcK", "ＦＵＣＫ", "ｈｅｌｌｏ", "🙂", "fuck🙂", "🙂fuck🙂",
    "café", "naïve", "ﬁle", "ＡＢＣ", "١٢٣", "مرحبا", "こんにちは", "中文",
    "é", "́", "﻿", "﻿fuck", "a​b", "nbsp space",
]

LEET = [
    "sh1t", "5hit", "f4ck", "@ss", "a55", "b1tch", "p0rn", "1337", "404", "100k",
    "1st", "2nd", "mp3", "h2o", "ps5", "win7", "no1", "u2", "0", "123", "12345",
]

SENTENCES = [
    "you are such a dickhead honestly",
    "what a fucking mess this is",
    "Check out this spam_link right now",
    "Hello, how are you doing today?",
    "ты полный мудак",
    "это badword и ещё custombad",
    "mixed привет and fuck together",
    "a b c d e f g h i j k l m n o p",
    "verylongprofanityword " * 2,
    "clean " * 10,
    "fuck " * 5,
    "line one\nline two fuck\nline three",
    "tab\tseparated\tfuck",
]

REPEATS = ["fuuuck", "ffuck", "fucck", "fuckk", "ffffuck", "fuuuuuuck", "shiiit",
           "assess", "book", "boot", "cook", "cassette", "bookkeeper"]

CORPUS = list(dict.fromkeys(CLEAN + PUNCT + UNICODE + LEET + SENTENCES + REPEATS + DICT))

# (normalize_text, aggressive_normalize, transliterate, replace_homoglyphs)
CONFIGS = [
    (True, True, True, True),      # дефолт
    (True, False, True, True),     # ветка allow_underscore
    (True, True, False, False),    # как в tests/test_integration.py
    (False, False, False, False),  # нормализация выключена целиком
]

THRESHOLDS = [1.0, 0.9, 0.85, 0.0]
REPLACE = [None, "*", "#"]

out = []
cases = 0
for cfg in CONFIGS:
    f = PyProfanityFilter(RESOURCE_DIR, *cfg)
    f.init([])
    f.add_words(DICT)
    for text in CORPUS:
        results = []
        for th in THRESHOLDS:
            for rc in REPLACE:
                r = f.filter_text(text, th, rc)
                if rc is None:
                    found, output = bool(r), None
                elif r is False:
                    found, output = False, None
                else:
                    found, output = True, r
                results.append([th, rc, found, output])
                cases += 1
        out.append({"cfg": list(cfg), "text": text, "results": results})

dest = Path("rust/badwords-core/tests/fixtures/legacy_golden.jsonl")
with dest.open("w", encoding="utf-8") as fh:
    fh.write(json.dumps({"dict": DICT}, ensure_ascii=False) + "\n")
    for rec in out:
        fh.write(json.dumps(rec, ensure_ascii=False, sort_keys=True) + "\n")

print(f"строк: {len(out)}, случаев: {cases}  ({len(CORPUS)} входов x {len(CONFIGS)} конфигов "
      f"x {len(THRESHOLDS)} порогов x {len(REPLACE)} режимов замены)")
print(f"найдено мата: {sum(1 for r in out for c in r['results'] if c[2])}")
print(f"файл: {dest}  {dest.stat().st_size / 1024:.0f} КБ")
