use bumpalo::Bump;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::io::{Read, Write};

struct MockReadWrite;

impl Read for MockReadWrite {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        buf.fill(b'A');
        Ok(buf.len())
    }
}

impl Write for MockReadWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_bf(bf: &str) {
    let bump = Bump::new();
    let parsed = brainfuck::parse::parse(&bump, bf.bytes().enumerate()).unwrap();
    let optimized = brainfuck::opts::optimize(&bump, &parsed);
    brainfuck::ir_interpreter::run(&optimized, MockReadWrite, MockReadWrite);
}

fn optimized(c: &mut Criterion) {
    let fizzbuzz = include_str!("fizzbuzz.bf");
    let bench = include_str!("bench.bf");
    let loopremove = include_str!("loopremove.bf");
    let twinkle = include_str!("twinkle.bf");
    let bottles = include_str!("bottles.bf");

    c.bench_function("fizzbuzz", |b| b.iter(|| run_bf(black_box(fizzbuzz))));
    c.bench_function("bench", |b| b.iter(|| run_bf(black_box(bench))));
    c.bench_function("loopremove", |b| b.iter(|| run_bf(black_box(loopremove))));
    c.bench_function("twinkle", |b| b.iter(|| run_bf(black_box(twinkle))));
    c.bench_function("bottles", |b| b.iter(|| run_bf(black_box(bottles))));
}

criterion_group!(benches, optimized);
criterion_main!(benches);
