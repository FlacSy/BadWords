# Word list sources

The lists in this directory are assembled from the project's own curation plus
two public collections. Both require attribution; see `/NOTICE` at the
repository root for the notices themselves.

| Source | License | Used for |
|--------|---------|----------|
| Project curation | MIT (this repository) | `en`, `es_419`, `pt_br`, and the bulk of `pl`, `ru`, `uk`, `tr`, `el` |
| [List of Dirty, Naughty, Obscene and Otherwise Bad Words][ldnoobw] | CC BY 4.0 | `cs`, `da`, `de`, `en`, `es`, `fi`, `fr`, `hu`, `it`, `ja`, `ko`, `nl`, `no`, `pl`, `pt`, `ru`, `sv`, `th`, `tr` |
| [washyourmouthoutwithsoap][wymows] | MIT | every language above, plus `el`, `id`, `ro`, `uk` |

[ldnoobw]: https://github.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words
[wymows]: https://github.com/thisandagain/washyourmouthoutwithsoap

## Format

One entry per line, sorted, deduplicated, lowercase, UTF-8 without a BOM.

Entries are matched **literally** after normalization, not as patterns. A line
containing regex metacharacters is not a pattern - it is a literal that can
never match, which is what `tests/test_wordlists.py` guards against.

Multi-word entries match consecutive words and are supported from 3.0.0
onwards. Before that they were inert.

## Adding entries

1. Edit the file here, not `python/badwords/resource/words` - that is a mirror.
2. `make sync-resources`
3. `make lang-packages` to regenerate the npm package
4. `make test`, and `make fp-report` if the entry is short or could occur
   inside ordinary words

Prefer entries that come from a citable list over ones invented on the spot,
and be wary of anything under five characters: it is what makes substring
matching expensive.
