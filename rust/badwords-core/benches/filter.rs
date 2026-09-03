use badwords_core::{MatchMode, Options, ProfanityFilter, Scratch};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn make_filter() -> ProfanityFilter {
    ProfanityFilter::builder()
        .embedded()
        .languages(["en", "ru"])
        .build()
        .expect("embedded resources")
}

const CLEAN: &str = "This is a perfectly normal sentence about programming";
const BAD: &str = "Some bad words here";
const MANY: [&str; 5] = [
    "Hello world",
    "This is fine",
    "Some bad words here",
    "Another clean message",
    "Yet another one",
];

fn bench_is_profane_clean(c: &mut Criterion) {
    let filter = make_filter();
    let opts = Options::new();
    c.bench_function("is_profane_clean", |b| {
        b.iter(|| filter.is_profane(black_box(CLEAN), opts));
    });
}

fn bench_is_profane_bad(c: &mut Criterion) {
    let mut filter = make_filter();
    filter.add_words(&["bad"]);
    let opts = Options::new();
    c.bench_function("is_profane_bad", |b| {
        b.iter(|| filter.is_profane(black_box(BAD), opts));
    });
}

fn bench_censor(c: &mut Criterion) {
    let mut filter = make_filter();
    filter.add_words(&["bad"]);
    let opts = Options::new();
    c.bench_function("censor", |b| {
        b.iter(|| filter.censor(black_box("Replace bad word"), '*', opts));
    });
}

fn bench_many(c: &mut Criterion) {
    let mut filter = make_filter();
    filter.add_words(&["bad"]);
    let opts = Options::new();
    c.bench_function("is_profane_many_5", |b| {
        b.iter(|| filter.is_profane_many(black_box(&MANY), opts));
    });
}

/// The reason the fuzzy index exists: this was a linear scan over every entry,
/// with an allocation per comparison.
fn bench_fuzzy(c: &mut Criterion) {
    let filter = make_filter();
    let opts = Options::new().threshold(0.9);
    c.bench_function("find_fuzzy_090", |b| {
        b.iter(|| filter.find(black_box(CLEAN), opts));
    });
}

fn bench_scratch_reuse(c: &mut Criterion) {
    let filter = make_filter();
    let opts = Options::new();
    let mut scratch = Scratch::new();
    let mut out = Vec::new();
    c.bench_function("find_into_reused_scratch", |b| {
        b.iter(|| filter.find_into(black_box(CLEAN), opts, &mut scratch, &mut out));
    });
}

fn bench_substring(c: &mut Criterion) {
    let mut filter = make_filter();
    filter.add_words(&["badword"]);
    let opts = Options::new().match_mode(MatchMode::Substring);
    c.bench_function("find_substring", |b| {
        b.iter(|| filter.find(black_box("prefixbadwordsuffix here"), opts));
    });
}

criterion_group!(
    benches,
    bench_is_profane_clean,
    bench_is_profane_bad,
    bench_censor,
    bench_many,
    bench_fuzzy,
    bench_scratch_reuse,
    bench_substring,
);
criterion_main!(benches);
