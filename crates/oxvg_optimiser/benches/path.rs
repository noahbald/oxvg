//! Benchmarks for path processing
use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxvg_ast::{
    arena::Allocator,
    element::Element,
    parse::roxmltree::parse,
    visitor::{Info, Visitor},
};
use oxvg_optimiser::ConvertPathData;
use roxmltree::ParsingOptions;

/// # Panics
/// Hopefully never, maybe if svg can't be parsed
pub fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! file {
        ($file:expr $(,)?) => {
            ($file, include_str!($file))
        };
    }
    let files = [
        file!("./archlinux-logo-dark-scalable.518881f04ca9.svg"),
        file!("./banner.svg"),
        file!("./blobs-d.svg"),
        file!("./Wikipedia-logo-v2.svg"),
        file!("./Inkscape_About_Screen_Isometric_madness_HdG4la4.svg"),
    ];
    for (filename, svg) in files {
        c.bench_with_input(
            BenchmarkId::new("optimise path", filename),
            &svg,
            |b, svg| {
                b.iter_custom(|iters| {
                    let mut result = Duration::default();
                    for _ in 0..iters {
                        let xml = roxmltree::Document::parse_with_options(
                            svg,
                            ParsingOptions {
                                allow_dtd: true,
                                ..ParsingOptions::default()
                            },
                        )
                        .unwrap();
                        let values = Allocator::new_values();
                        let mut arena = Allocator::new_arena();
                        let mut allocator = Allocator::new(&mut arena, &values);
                        let dom = parse(&xml, &mut allocator).unwrap();
                        let mut root = Element::from_parent(dom).unwrap();
                        let job = ConvertPathData::default();
                        let info = &Info::new(allocator);
                        let start = Instant::now();
                        let _ = black_box(job.start(&mut root, info, None));
                        result += start.elapsed();
                    }
                    result
                });
            },
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
