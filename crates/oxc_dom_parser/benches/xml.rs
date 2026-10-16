//! Benchmark XML against fixtures.
//!
//! - Fixtures are copied from roxmltree repo.
//! - Benches run both for roxmltree and oxc_dom_parser under similar conditions.
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxc_dom_parser::{Allocator, MimeType, Parser};
use std::{
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::Duration,
};

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("self");
    group.measurement_time(Duration::from_secs(8));

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = workspace_root.join("benches/fixtures");
    let inputs = collect_bench_inputs(&fixture_dir);
    assert!(
        !inputs.is_empty(),
        "no benchmark inputs found in {}",
        fixture_dir.display(),
    );

    for path in inputs {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let name_roxmltree = format!("{} (roxmltree)", name.as_str());
        let code = black_box(fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read benchmark input {}: {error}", path.display())
        }));

        group.bench_with_input(
            BenchmarkId::from_parameter(name.clone()),
            &code,
            |b, code| {
                b.iter(|| {
                    let allocator = Allocator::default();
                    let parser = Parser::new(&allocator, code, MimeType::ApplicationXML);
                    black_box(parser.parse().map_err(|e| (e, name.clone())).unwrap());
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::from_parameter(name_roxmltree),
            &code,
            |b, code| {
                b.iter(|| {
                    let options = roxmltree::ParsingOptions {
                        allow_dtd: true,
                        ..Default::default()
                    };
                    black_box(roxmltree::Document::parse_with_options(code, options).unwrap());
                });
            },
        );
    }
    group.finish();
}

fn collect_bench_inputs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut inputs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    inputs.sort();
    inputs
}

criterion_group!(benches, bench_parser);
criterion_main!(benches);
