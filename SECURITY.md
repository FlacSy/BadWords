# Security policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 3.x     | yes       |
| 2.x     | security fixes only, until 2027-01-01 |
| < 2.0   | no        |

## Reporting a vulnerability

Please report privately through
[GitHub Security Advisories](https://github.com/FlacSy/badwords/security/advisories/new),
or by email to flacsy.x@gmail.com. Please do not open a public issue first.

Expect an acknowledgement within a few days and an assessment within two weeks.

## What counts

This is a text-processing library, so the interesting surface is small but real:

- **Untrusted input to the matcher.** Any panic, unbounded allocation or
  quadratic blowup reachable from `is_profane`, `find` or `censor` on
  attacker-controlled text is a vulnerability, not a bug report.
- **Untrusted word lists or resource directories.** Loading a language must
  never read outside the resource directory. Caller-supplied language codes are
  resolved through the registry and never used as path components; a way around
  that is a vulnerability.
- **The ML model download.** Archive extraction rejects entries that would
  write outside the target directory. Report anything that gets past it.

## What does not count

- The filter missing profanity, or flagging something ordinary. That is a
  word-list or tuning issue - please open a regular issue.
- Denial of service from options you enabled yourself. Substring matching and
  a low fuzzy threshold are documented as more expensive.
