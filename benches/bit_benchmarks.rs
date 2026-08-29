use bpc::bit::{BitReader, BitWriter};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_write_single_bits(c: &mut Criterion) {
    c.bench_function("write_bit x 8000", |b| {
        b.iter(|| {
            let mut writer = BitWriter::new();
            for _ in 0..8000 {
                writer.write_bit(black_box(true));
            }
            black_box(writer.into_bytes());
        });
    });
}

fn bench_write_bits_aligned(c: &mut Criterion) {
    c.bench_function("write_bits(8) x 1000", |b| {
        b.iter(|| {
            let mut writer = BitWriter::new();
            for _ in 0..1000 {
                writer.write_bits(black_box(0xAB), 8).unwrap();
            }
            black_box(writer.into_bytes());
        });
    });
}

fn bench_write_bits_unaligned(c: &mut Criterion) {
    c.bench_function("write_bits(7) x 1000", |b| {
        b.iter(|| {
            let mut writer = BitWriter::new();
            for _ in 0..1000 {
                writer.write_bits(black_box(0x55), 7).unwrap();
            }
            black_box(writer.into_bytes());
        });
    });
}

fn bench_read_single_bits(c: &mut Criterion) {
    let data = vec![0xAA; 1000];
    c.bench_function("read_bit x 8000", |b| {
        b.iter(|| {
            let mut reader = BitReader::from_bytes(black_box(&data));
            for _ in 0..8000 {
                black_box(reader.read_bit().unwrap());
            }
        });
    });
}

fn bench_read_bits_aligned(c: &mut Criterion) {
    let data = vec![0xAA; 1000];
    c.bench_function("read_bits(8) x 1000", |b| {
        b.iter(|| {
            let mut reader = BitReader::from_bytes(black_box(&data));
            for _ in 0..1000 {
                black_box(reader.read_bits(8).unwrap());
            }
        });
    });
}

fn bench_read_bits_unaligned(c: &mut Criterion) {
    let data = vec![0xAA; 1000];
    c.bench_function("read_bits(7) x 1000", |b| {
        b.iter(|| {
            let mut reader = BitReader::from_bytes(black_box(&data));
            while reader.remaining() >= 7 {
                black_box(reader.read_bits(7).unwrap());
            }
        });
    });
}

fn bench_round_trip_u32(c: &mut Criterion) {
    c.bench_function("round_trip u32 x 250", |b| {
        b.iter(|| {
            let mut writer = BitWriter::new();
            for i in 0u32..250 {
                writer.write_bits(black_box(u64::from(i)), 32).unwrap();
            }
            let bytes = writer.into_bytes();
            let mut reader = BitReader::from_bytes(&bytes);
            for _ in 0..250 {
                black_box(reader.read_bits(32).unwrap());
            }
        });
    });
}

fn bench_write_large_buffer(c: &mut Criterion) {
    c.bench_function("write_bits(32) x 8192 (1MB)", |b| {
        b.iter(|| {
            let mut writer = BitWriter::with_capacity(8192 * 32);
            for _ in 0..8192 {
                writer.write_bits(black_box(0xDEADBEEF), 32).unwrap();
            }
            black_box(writer.into_bytes());
        });
    });
}

criterion_group!(
    benches,
    bench_write_single_bits,
    bench_write_bits_aligned,
    bench_write_bits_unaligned,
    bench_read_single_bits,
    bench_read_bits_aligned,
    bench_read_bits_unaligned,
    bench_round_trip_u32,
    bench_write_large_buffer,
);
criterion_main!(benches);
