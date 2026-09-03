## What this changes

<!-- One or two sentences. Link the issue if there is one. -->

## Checklist

- [ ] `make test` passes
- [ ] `make lint` and `make format` pass
- [ ] Word lists edited in `rust/badwords-core/resources/words`, followed by
      `make sync-resources` and `make lang-packages`
- [ ] `make fp-report` run if this could add false positives (new short
      entries, or a change to matching)
- [ ] `CHANGELOG.md` updated for anything user-visible

## Detection changes

<!-- Delete if this does not touch matching.

Which inputs change behaviour, and what the false-positive measurement says.
New detectors default to off; see CONTRIBUTING.md. -->
