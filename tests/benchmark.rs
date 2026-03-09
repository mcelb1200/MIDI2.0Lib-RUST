use am_midi2::utils::scale_down;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mut sum: u32 = 0;

    // warm up
    for _i in 0..10_000_000 {
        sum = sum.wrapping_add(scale_down(
            black_box(0xFFFF_FFFF),
            black_box(32),
            black_box(7),
        ));
        sum = sum.wrapping_add(scale_down(
            black_box(0x4000_0000),
            black_box(32),
            black_box(7),
        ));
        sum = sum.wrapping_add(scale_down(black_box(0x100), black_box(16), black_box(7)));
        sum = sum.wrapping_add(scale_down(black_box(0xFFFF), black_box(16), black_box(8)));
    }

    let start = Instant::now();
    for _i in 0..10_000_000 {
        sum = sum.wrapping_add(scale_down(
            black_box(0xFFFF_FFFF),
            black_box(32),
            black_box(7),
        ));
        sum = sum.wrapping_add(scale_down(
            black_box(0x4000_0000),
            black_box(32),
            black_box(7),
        ));
        sum = sum.wrapping_add(scale_down(black_box(0x100), black_box(16), black_box(7)));
        sum = sum.wrapping_add(scale_down(black_box(0xFFFF), black_box(16), black_box(8)));
    }
    println!("Time: {:?}", start.elapsed());
    println!("Sum: {}", sum);
}
