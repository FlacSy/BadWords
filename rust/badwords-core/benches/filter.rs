//! Benchmarks for badwords-core filter.

use badwords_core::{default_resource_dir, ProfanityFilter};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn make_filter() -> ProfanityFilter {
    let resource_dir = default_resource_dir();
    let mut filter = ProfanityFilter::new(&resource_dir, true, true, true, true);
    filter.init(Some(&["en".to_string(), "ru".to_string()])).unwrap();
    filter
}

fn bench_filter_clean(c: &mut Criterion) {
    let filter = make_filter();
    c.bench_function("filter_text_clean", |b| {
        b.iter(|| {
            let (found, _) = filter.filter_text(
                black_box("Hello, this is a clean message for testing."),
                1.0,
                None,
            );
            found
        })
    });
}

fn bench_filter_bad(c: &mut Criterion) {
    let filter = make_filter();
    c.bench_function("filter_text_bad", |b| {
        b.iter(|| {
            let (found, _) = filter.filter_text(
                black_box("Some bad words here"),
                1.0,
                None,
            );
            found
        })
    });
}

fn bench_filter_censor(c: &mut Criterion) {
    let filter = make_filter();
    c.bench_function("filter_text_censor", |b| {
        b.iter(|| {
            let (found, result) = filter.filter_text(
                black_box("Replace bad word"),
                1.0,
                Some('*'),
            );
            (found, result)
        })
    });
}

fn bench_filter_many(c: &mut Criterion) {
    let filter = make_filter();
    let texts: Vec<&str> = vec![
        "Hello world",
        "Clean message",
        "Some text with potential",
        "Another clean one",
        "Final test string",
    ];
    c.bench_function("filter_text_many", |b| {
        b.iter(|| {
            for text in &texts {
                let (found, _) = filter.filter_text(black_box(*text), 1.0, None);
                black_box(found);
            }
        })
    });
}

criterion_group!(
    benches,
    bench_filter_clean,
    bench_filter_bad,
    bench_filter_censor,
    bench_filter_many
);
criterion_main!(benches);
