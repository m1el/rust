#![feature(transmutability)]
#![feature(marker_trait_attr)]
#![allow(dead_code)]
#![allow(incomplete_features)]

fn test_assume_validity() {
    // if the compiler can assume that the programmer is performing additional validity
    // checks, a transmute from bool to u8 can be sound
    assert_is_transmutable_assume_validity::<bool, u8>();

    // ...but even with validity assumed, the compiler will still reject transmutations
    // that couldn't *possibly* be valid. e.g.: an uninit byte cannot become an initialized byte.
    #[repr(C, align(2))] pub struct BoolThenPadding(pub bool);
    assert_is_transmutable_any::<BoolThenPadding, u16>(); //~ ERROR not satisfied
}

use std::mem::BikeshedIntrinsicFrom;

struct Context;

/// Assert that `Src` is transmutable to `Dst` under all combinations of options
/// where validity is assumed.
fn assert_is_transmutable_assume_validity<Src, Dst>()
where
    Dst:
        // Uncomment once visibility checking is implemented:
        /*
        + BikeshedIntrinsicFrom<Src, Context, false, false,  true, false>
        + BikeshedIntrinsicFrom<Src, Context,  true, false,  true, false>
        + BikeshedIntrinsicFrom<Src, Context, false,  true,  true, false>
        + BikeshedIntrinsicFrom<Src, Context,  true,  true,  true, false>
        + */ BikeshedIntrinsicFrom<Src, Context, false, false,  true,  true>
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

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false,  true, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false,  true, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true,  true, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true,  true, false>
{}

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

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false,  true,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false,  true,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true,  true,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAny<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true,  true,  true>
{}

fn main() {}
