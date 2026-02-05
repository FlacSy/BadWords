<div align="center">

  # 🚫 BadWords

  **High-performance profanity filter for Python with multilingual support and evasion detection.**

  [![Python Version](https://img.shields.io/badge/python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13-blue?style=flat-square)](https://www.python.org/)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
  [![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)](#)
  [![Downloads](https://img.shields.io/pypi/dm/badwords-py?style=flat-square&color=orange)](https://pypi.org/project/badwords-py/)
  
  [Installation](#-installation) • [Quick Start](#-quick-start) • [Supported Languages](#-supported-languages) • [Advanced Evasion Detection](#-advanced-evasion-detection)

</div>

---

## 📖 Description

`BadWords` is a sophisticated profanity filtering library designed to clean up user-generated content. Unlike simple keyword matching, it uses **similarity scoring**, **homoglyph detection**, and **transliteration** to catch even the most cleverly disguised insults.

## 📦 Installation

### Requirements
- **Recommended:** Python 3.13
- **Minimum:** Python 3.10+

### Install via GitHub
```bash
pip install git+[https://github.com/FlacSy/badwords.git](https://github.com/FlacSy/badwords.git)

```

### Install via PyPI
```bash
pip install badwords-py
```

---

## ⚡ Quick Start

### Basic Initialization

```python
from badwords import ProfanityFilter

# Initialize filter
p = ProfanityFilter()

# Load specific languages (e.g., English and Russian)
p.init(languages=["en", "ru"])

# Or load ALL 26+ supported languages
p.init()

```

### Checking and Filtering Text

```python
text = "Some very b4d text here"

# 1. Simple check (Returns Boolean)
is_bad = p.filter_text(text)
print(is_bad) # True

# 2. Censoring text (Returns String)
clean_text = p.filter_text(text, replace_character="*")
print(clean_text) # "Some very *** text here"

```

---

## 🛠 Methods & API

### `filter_text(text, match_threshold=0.8, replace_character=None)`

The core method of the library.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `text` | `str` | Required | Input text to check. |
| `match_threshold` | `float` | `0.8` | Similarity threshold (1.0 = exact match, 0.7 = aggressive). |
| `replace_character` | `str/None` | `None` | If provided, returns censored string. If None, returns bool. |

> [!WARNING]
> **Performance Tip:** Using `match_threshold < 1.0` enables fuzzy matching which is slower. Use `1.0` for high-traffic real-time filtering, or `0.95` for a good balance.

---

## 🧩 Advanced Evasion Detection

Standard filters are easy to bypass. `BadWords` is built to detect:

* **Homoglyphs:** Detects `hеllo` (using Cyrillic 'е') or `h4llo` (numbers).
* **Transliteration:** Automatically handles mapping between Cyrillic and Latin alphabets.
* **Normalization:** Strips diacritics, special characters, and decorative Unicode symbols.
* **Similarity Analysis:** Uses fuzzy matching to find words with deliberate typos.

### Examples of detected evasions:

```python
_filter.filter_text("hеllо")  # Mixed alphabets (Cyrillic + Latin) -> DETECTED
_filter.filter_text("h3ll0")  # Character substitution -> DETECTED
_filter.filter_text("h⍺llo")  # Mathematical/Greek symbols -> DETECTED
_filter.filter_text("привет") # Transliterated matches -> DETECTED

```

---

## 🌍 Supported Languages

`BadWords` currently supports **26 languages** out of the box:

| Code | Language | Code | Language | Code | Language |
| --- | --- | --- | --- | --- | --- |
| `en` | English | `ru` | Russian | `ua` | Ukrainian |
| `de` | German | `fr` | French | `it` | Italian |
| `sp` | Spanish | `pl` | Polish | `cz` | Czech |
| `ja` | Japanese | `ko` | Korean | `th` | Thai |
| ... | & 14 more |  |  |  |  |

*Use `p.get_all_languages()` to see the full list in your code.*

---

## 🚀 Full Integration Example

```python
from badwords import ProfanityFilter

def monitor_chat():
    # Setup for a global chat
    profanity_filter = ProfanityFilter()
    profanity_filter.init(["en", "ru", "de"])
    
    # Custom project-specific banned words
    profanity_filter.add_words(["spam_link_v1", "scam_bot_99"])

    user_input = "Hey! Check out this b.a.d.w.o.r.d"
    
    # Moderate with high accuracy
    is_offensive = profanity_filter.filter_text(user_input, match_threshold=0.95)
    
    if is_offensive:
        print("Message blocked: Contains restricted language.")
    else:
        # Proceed with processing
        pass

if __name__ == "__main__":
    monitor_chat()

```

---

## 🤝 Contributing

Contributions are what make the open-source community an amazing place to learn, inspire, and create.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

<div align="center">
<sub>Developed with ❤️ by <a href="https://github.com/FlacSy">FlacSy</a></sub>
</div>
