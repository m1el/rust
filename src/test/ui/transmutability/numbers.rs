#![feature(transmutability)]
#![feature(marker_trait_attr)]
#![allow(dead_code)]
#![allow(incomplete_features)]

fn test_identity() {
    assert_is_transmutable_all::< bool,  bool>();
    assert_is_transmutable_all::<   i8,    i8>();
    assert_is_transmutable_all::<   u8,    u8>();
    assert_is_transmutable_all::<  i16,   i16>();
    assert_is_transmutable_all::<  u16,   u16>();
    assert_is_transmutable_all::<  i32,   i32>();
    assert_is_transmutable_all::<  f32,   f32>();
    assert_is_transmutable_all::<  u32,   u32>();
    assert_is_transmutable_all::<  i64,   i64>();
    assert_is_transmutable_all::<  f64,   f64>();
    assert_is_transmutable_all::<  u64,   u64>();
    assert_is_transmutable_all::< i128,  i128>();
    assert_is_transmutable_all::< u128,  u128>();
    assert_is_transmutable_all::<isize, isize>();
    assert_is_transmutable_all::<usize, usize>();
}

fn test_same_size() {
    assert_is_transmutable_all::< bool,    i8>();
    assert_is_transmutable_all::< bool,    u8>();
    assert_is_transmutable_any::<   i8,  bool>(); //~ ERROR not satisfied
    assert_is_transmutable_all::<   i8,    u8>();
    assert_is_transmutable_any::<   u8,  bool>(); //~ ERROR not satisfied
    assert_is_transmutable_all::<   u8,    i8>();

    assert_is_transmutable_all::<  i16,   u16>();
    assert_is_transmutable_all::<  u16,   i16>();

    assert_is_transmutable_all::<  i32,   f32>();
    assert_is_transmutable_all::<  i32,   u32>();
    assert_is_transmutable_all::<  f32,   i32>();
    assert_is_transmutable_all::<  f32,   u32>();
    assert_is_transmutable_all::<  u32,   i32>();
    assert_is_transmutable_all::<  u32,   f32>();

    assert_is_transmutable_all::<  u64,   i64>();
    assert_is_transmutable_all::<  u64,   f64>();
    assert_is_transmutable_all::<  i64,   u64>();
    assert_is_transmutable_all::<  i64,   f64>();
    assert_is_transmutable_all::<  f64,   u64>();
    assert_is_transmutable_all::<  f64,   i64>();

    assert_is_transmutable_all::< u128,  i128>();
    assert_is_transmutable_all::< i128,  u128>();

    assert_is_transmutable_all::<isize, usize>();
    assert_is_transmutable_all::<usize, isize>();
}

fn test_extension() {
    assert_is_transmutable_any::< bool,   i16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   u16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   i32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   f32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   u32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::< bool,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<   i8,   i16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   u16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   i32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   f32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   u32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   i8,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<   u8,   i16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   u16>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   i32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   f32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   u32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<   u8,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  i16,   i32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,   f32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,   u32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i16,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  u16,   i32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,   f32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,   u32>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u16,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  i32,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i32,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i32,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i32,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i32,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  f32,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  f32,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  f32,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  f32,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  f32,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  u32,   u64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u32,   i64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u32,   f64>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u32,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u32,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  u64,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  u64,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  i64,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  i64,  i128>(); //~ ERROR not satisfied

    assert_is_transmutable_any::<  f64,  u128>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<  f64,  i128>(); //~ ERROR not satisfied
}


use std::mem::BikeshedIntrinsicFrom;

struct Context;

/// Assert that `Src` is transmutable to `Dst` under all combinations of options.
fn assert_is_transmutable_all<Src, Dst>()
where
    Dst:
        // Uncomment once visibility checking is implemented:
        /* BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        + BikeshedIntrinsicFrom<Src, Context,  true, false, false, false>
        + BikeshedIntrinsicFrom<Src, Context, false,  true, false, false>
        + BikeshedIntrinsicFrom<Src, Context,  true,  true, false, false>
        + BikeshedIntrinsicFrom<Src, Context, false, false,  true, false>
        + BikeshedIntrinsicFrom<Src, Context,  true, false,  true, false>
        + BikeshedIntrinsicFrom<Src, Context, false,  true,  true, false>
        + BikeshedIntrinsicFrom<Src, Context,  true,  true,  true, false>
        + */ BikeshedIntrinsicFrom<Src, Context, false, false, false,  true>
        + BikeshedIntrinsicFrom<Src, Context,  true, false, false,  true>
        + BikeshedIntrinsicFrom<Src, Context, false,  true, false,  true>
        + BikeshedIntrinsicFrom<Src, Context,  true,  true, false,  true>
        + BikeshedIntrinsicFrom<Src, Context, false, false,  true,  true>
        + BikeshedIntrinsicFrom<Src, Context,  true, false,  true,  true>
        + BikeshedIntrinsicFrom<Src, Context, false,  true,  true,  true>
        + BikeshedIntrinsicFrom<Src, Context,  true,  true,  true,  true>
{}

/// Assert that `Src` is transmutable to `Dst` for at least one combination of options,
/// except those where validity is assumed.
fn assert_is_transmutable_any<Src, Dst>()
where
    Dst:  BikeshedIntrinsicFromAny<Src>
{}

#[marker]
trait BikeshedIntrinsicFromAny<Src: ?Sized> {}

// Uncomment once visibility checking is implemented:
/*
impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true, false, false>
{}
*/

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true, false,  true>
{}

fn main() {}
