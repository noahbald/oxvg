//! Benchmarks for path processing
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxvg_ast::{
    element::Element,
    parse::roxmltree::{ParsingOptions, parse_with_options},
    visitor::{Info, Visitor},
};
use oxvg_optimiser::ConvertPathData;

/// # Panics
/// Hopefully never, maybe if svg can't be parsed
pub fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! file {
        ($file:expr $(,)?) => {
            ($file, include_str!($file))
        };
    }
    let files = [
        file!("./fixtures/gnome-blobs.svg"),
        file!("./fixtures/inkscape-isometric-madness.svg"),
        file!("./fixtures/tldr-banner.svg"),
        file!("./fixtures/trajans-column.svg"),
    ];
    for (filename, svg) in files {
        c.bench_with_input(
            BenchmarkId::new("optimise path", filename),
            &svg,
            |b, svg| {
                b.iter_custom(|iters| {
                    let mut result = Duration::default();
                    for _ in 0..iters {
                        parse_with_options(
                            svg,
                            ParsingOptions {
                                allow_dtd: true,
                                ..ParsingOptions::default()
                            },
                            |dom, allocator| {
                                let root = Element::from_parent(dom).unwrap();
                                let job = ConvertPathData::default();
                                let info = &Info::new(allocator);
                                let start = Instant::now();
                                let _ = black_box(job.start_with_info(root, info, None));
                                result += start.elapsed();
                            },
                        )
                        .unwrap();
                    }
                    result
                });
            },
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
