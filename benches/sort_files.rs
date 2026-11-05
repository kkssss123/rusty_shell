use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use rusty_shell::commands::rn::{SortBy, TargetFile as ActualTargetFile, sort_files};
use std::hint::black_box;
use std::time::SystemTime;

struct MockTargetFile<'a> {
    _name: &'a str,
    lower_name: String,
}

fn sort_current(file_names: &mut [String]) {
    file_names.sort_by(|a, b| {
        let a = a.to_lowercase();
        let b = b.to_lowercase();
        alphanumeric_sort::compare_str(a, b)
    })
}

fn sort_cached(file_names: &[String]) -> Vec<MockTargetFile<'_>> {
    let mut cached_file_names: Vec<MockTargetFile> = vec![];
    for f in file_names {
        cached_file_names.push(MockTargetFile {
            _name: f.as_str(),
            lower_name: f.to_lowercase(),
        })
    }
    cached_file_names.sort_by(|a, b| alphanumeric_sort::compare_str(&a.lower_name, &b.lower_name));

    cached_file_names
}

fn bench_sort_files_mock(c: &mut Criterion) {
    let mut group = c.benchmark_group("mock_sort_files");

    for size in [100, 1000, 10000] {
        let file_names: Vec<String> = (0..size)
            .map(|i| {
                let prefixes = ["Video", "image", "MOVIE", "sample", "TEST", "clip"];
                let extensions = [".mp4", ".JPEG", ".jpg", ".PNG", ".webp"];

                format!(
                    "{}_{:04}{}",
                    prefixes[i % prefixes.len()],
                    i * 7 % 1000,
                    extensions[i % extensions.len()],
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("current", size),
            &file_names,
            |b, files| {
                b.iter_batched(
                    || files.clone(),
                    |mut files| {
                        sort_current(&mut files);
                        black_box(files);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(BenchmarkId::new("cached", size), &file_names, |b, files| {
            b.iter_batched(
                || files.clone(),
                |files| {
                    black_box(sort_cached(&files));
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_sort_files_actual(c: &mut Criterion) {
    let mut group = c.benchmark_group("actual_sort_files");
    let sort_by = SortBy::FileName { desc: false };

    for size in [100, 1000, 10000] {
        let file_names: Vec<ActualTargetFile> = (0..size)
            .map(|i| {
                let prefixes = ["Video", "image", "MOVIE", "sample", "TEST", "clip"];
                let extensions = [".mp4", ".JPEG", ".jpg", ".PNG", ".webp"];

                let name = format!(
                    "{}_{:04}{}",
                    prefixes[i % prefixes.len()],
                    i * 7 % 1000,
                    extensions[i % extensions.len()],
                );
                let name_lower = name.to_lowercase();
                ActualTargetFile {
                    name,
                    name_lower,
                    mtime: SystemTime::now(),
                }
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("sort", size), &file_names, |b, files| {
            b.iter_batched(
                || files.clone(),
                |mut files| {
                    sort_files(&mut files, &sort_by);
                    black_box(files);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sort_files_mock, bench_sort_files_actual);
criterion_main!(benches);
