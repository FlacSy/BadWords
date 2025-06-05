"""Module for advanced text processing and normalization."""

from __future__ import annotations

import json
import re
import unicodedata
from pathlib import Path
from typing import Dict, Set

class TextProcessor:
    """A class for advanced text processing and normalization."""

    def __init__(
            self,
            processing_normalize_text: bool = True,
            processing_aggressive_normalize: bool = True,
            processing_transliterate: bool = True,
            processing_replace_homoglyphs: bool = True,
    ) -> None:
        """Initialize the text processor."""
        self.processing_normalize_text = processing_normalize_text
        self.processing_aggressive_normalize = processing_aggressive_normalize
        self.processing_transliterate = processing_transliterate
        self.processing_replace_homoglyphs = processing_replace_homoglyphs

        self.resource_dir = Path(__file__).parent / 'resource'
        self.unicode_mappings = self._load_unicode_mappings()

        if self.processing_replace_homoglyphs == True:
            self.homoglyphs = self._load_homoglyphs()

        self.character_frequency = self._load_character_frequency()

        if self.processing_transliterate == True:
            self.cyrillic_to_latin = self._load_transliteration()
            self.latin_to_cyrillic = {v: k for k, v in self.cyrillic_to_latin.items()}

        if self.processing_replace_homoglyphs == True:
            self._build_homoglyph_map()

        self._build_frequency_map()

    def _load_unicode_mappings(self) -> Dict[str, str]:
        """Load Unicode mappings from JSON file."""
        with open(self.resource_dir / 'unicode_mappings.json', 'r', encoding='utf-8') as f:
            data = json.load(f)
            mappings = {}
            for category in data.values():
                mappings.update(category)
            return mappings

    def _load_homoglyphs(self) -> Dict[str, list[str]]:
        """Load homoglyph mappings from JSON file."""
        with open(self.resource_dir / 'homoglyphs.json', 'r', encoding='utf-8') as f:
            return json.load(f)

    def _load_character_frequency(self) -> Dict[str, list[str]]:
        """Load character frequency mappings from JSON file."""
        with open(self.resource_dir / 'character_frequency.json', 'r', encoding='utf-8') as f:
            return json.load(f)

    def _load_transliteration(self) -> Dict[str, str]:
        """Load transliteration mappings from JSON file."""
        with open(self.resource_dir / 'transliteration.json', 'r', encoding='utf-8') as f:
            data = json.load(f)
            return data['cyrillic_to_latin']

    def _build_homoglyph_map(self) -> None:
        """Build a comprehensive homoglyph map."""
        self.homoglyph_map: Dict[str, Set[str]] = {}
        for standard, variants in self.homoglyphs.items():
            self.homoglyph_map[standard] = set(variants)
            for variant in variants:
                if variant not in self.homoglyph_map:
                    self.homoglyph_map[variant] = set()
                self.homoglyph_map[variant].add(standard)

    def _build_frequency_map(self) -> None:
        """Build a comprehensive frequency-based substitution map."""
        self.frequency_map: Dict[str, Set[str]] = {}
        for standard, variants in self.character_frequency.items():
            self.frequency_map[standard] = set(variants)
            for variant in variants:
                if variant not in self.frequency_map:
                    self.frequency_map[variant] = set()
                self.frequency_map[variant].add(standard)

    def normalize_unicode(self, text: str) -> str:
        """Normalize Unicode characters to their basic form.
        
        Args:
            text: Input text to normalize.
            
        Returns:
            Normalized text.
        """
        text = unicodedata.normalize('NFKC', text)
        
        text = ''.join(c for c in text if not unicodedata.combining(c))
        
        text = text.lower()
        
        result = []
        for char in text:
            if char in self.unicode_mappings:
                result.append(self.unicode_mappings[char])
            else:
                result.append(char)
        
        return ''.join(result)

    def normalize_text(self, text: str) -> str:
        """Normalize text by converting to lowercase and removing diacritics.
        
        Args:
            text: Input text to normalize.
            
        Returns:
            Normalized text.
        """
        text = self.normalize_unicode(text)
        
        text = re.sub(r'[^\w\s]', '', text)
        
        return text

    def aggressive_normalize(self, text: str) -> str:
        """Perform aggressive text normalization.
        
        Args:
            text: Input text to normalize.
            
        Returns:
            Aggressively normalized text.
        """
        text = self.normalize_unicode(text)
        
        text = ''.join(c for c in text if c.isalnum() or c.isspace())
        
        text = ' '.join(text.split())
        
        return text

    def transliterate(self, text: str, to_latin: bool = True) -> str:
        """Transliterate text between Cyrillic and Latin.
        
        Args:
            text: Input text to transliterate.
            to_latin: If True, convert to Latin; if False, convert to Cyrillic.
            
        Returns:
            Transliterated text.
        """
        mapping = self.cyrillic_to_latin if to_latin else self.latin_to_cyrillic
        result = []
        
        for char in text:
            if char in mapping:
                result.append(mapping[char])
            else:
                result.append(char)
                
        return ''.join(result)

    def replace_homoglyphs(self, text: str) -> str:
        """Replace homoglyphs with their standard equivalents.
        
        Args:
            text: Input text to process.
            
        Returns:
            Text with homoglyphs replaced.
        """
        result = []
        for char in text:
            if char in self.homoglyph_map and self.homoglyph_map[char]:
                result.append(next(iter(self.homoglyph_map[char])))
            else:
                result.append(char)
        return ''.join(result)

    def process_text(self, text: str) -> str:
        """Apply all text processing steps in sequence.
        
        Args:
            text: Input text to process.
            
        Returns:
            Fully processed text.
        """

        txt = text
        if self.processing_normalize_text == True:
            txt = self.normalize_text(txt)

        if self.processing_aggressive_normalize == True:
            txt = self.aggressive_normalize(txt)

        if self.processing_transliterate == True:
            txt = self.transliterate(txt, to_latin=True)
            txt = self.transliterate(txt, to_latin=False)

        if self.processing_replace_homoglyphs == True:
            txt = self.replace_homoglyphs(txt)

        return txt