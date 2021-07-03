#![feature(transmutability)]
#![feature(marker_trait_attr)]
#![allow(dead_code)]
#![allow(incomplete_features)]

fn test_well_defined() {
    #[repr(C)] struct WellDefined;
    pub struct Unspecified;

    assert_is_transmutable_all::<WellDefined, WellDefined>();
    assert_is_transmutable_any::<WellDefined, Unspecified>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<Unspecified, WellDefined>(); //~ ERROR not satisfied
    assert_is_transmutable_any::<Unspecified, Unspecified>(); //~ ERROR not satisfied
}

fn truncation() {
    #[repr(C)] pub struct Zst;

    assert_is_transmutable_all::<u8, Zst>();
}

fn extension() {
    #[repr(C, align(8))] pub struct U8ThenPadding(pub u8);

    // a `u8` is extensible to `U8ThenPadding`
    assert_is_transmutable_all::<u8, U8ThenPadding>();
    // a `U8ThenPadding` is truncatable to a `u8`
    assert_is_transmutable_all::<U8ThenPadding, u8>();
    // a `U8ThenPadding` is NOT extensible to a `u16`.
    assert_is_transmutable_any::<U8ThenPadding, u16>(); //~ ERROR not satisfied
}

fn same_size() {
    #[repr(C)] pub struct Padded(pub u16, pub u8);
    #[repr(C)] pub struct Unpadded(pub u16, pub u16);

    assert_is_transmutable_all::<Unpadded, Padded>();
    assert_is_transmutable_any::<Padded, Unpadded>(); //~ ERROR not satisfied
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

/// Assert that `Src` is transmutable to `Dst` for at least one combination of options.
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
