use criterion::{Criterion, criterion_group, criterion_main};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use oxvg_jsx::{
    config::{TemplateAST, TemplateString},
    transform,
};

fn criterion_benchmark(c: &mut Criterion) {
    swc_core::common::GLOBALS.set(&swc_core::common::Globals::new(), || {
        c.bench_function("transform (swc)", |b| {
            b.iter_custom(|iters| {
                let mut result = Duration::default();
                for _ in 0..iters {
                    oxvg_ast::parse::roxmltree::parse(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <rect x="25" y="36" width="48" height="1" aria-label="Test" class="rect"></rect>
</svg>"#,
                        |root, allocator| {
                            let mut buf = Vec::new();
                            let start = Instant::now();
                            let _ = black_box(transform::<_, TemplateAST<_>, TemplateString<_>>(
                                black_box(root),
                                allocator,
                                None,
                                None,
                                None,
                                black_box(&mut buf),
                            ));
                            result += start.elapsed();
                        },
                    );
                }
                result
            });
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
