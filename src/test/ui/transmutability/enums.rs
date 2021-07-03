#![feature(transmutability)]
#![feature(marker_trait_attr)]
#![allow(dead_code)]
#![allow(incomplete_features)]

fn between_differently_signed_reprs() {
    #[repr(i8)]
    enum I8 {
        V = -2,
    }

    #[repr(u8)]
    enum U8 {
        V = 254,
    }

    assert_is_transmutable_all::<I8, U8>();
    assert_is_transmutable_all::<U8, I8>();
}

fn truncation() {
    #[repr(i16)]
    enum I16 {
        V = i16::from_ne_bytes([42, 0]),
    }

    #[repr(u8)]
    enum U8 {
        V = u8::from_ne_bytes([42]),
    }

    assert_is_transmutable_all::<I16, U8>();
    assert_is_transmutable_any::<U8, I16>(); //~ ERROR not satisfied
}

fn set_expansion() {
    #[repr(u8)]
    enum Src {
        A = 2,
        B = 8,
        C = 32
    }

    #[repr(u8)]
    enum Dst {
        A = 2,
        B = 4,
        C = 8,
        D = 16,
        E = 32,
    }

    assert_is_transmutable_all::<Src, Dst>();
    assert_is_transmutable_any_except_validity::<Dst, Src>(); //~ ERROR not satisfied
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

///////////////////////////////////////////////////////////////////////////////

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

///////////////////////////////////////////////////////////////////////////////

/// Assert that `Src` is transmutable to `Dst` for at least one combination of options.
/// Validity is not assumed.
fn assert_is_transmutable_any_except_validity<Src, Dst>()
where
    Dst:  BikeshedIntrinsicFromAnyExceptValidity<Src>
{}

#[marker]
trait BikeshedIntrinsicFromAnyExceptValidity<Src: ?Sized> {}

// Uncomment once visibility checking is implemented:
/*
impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true, false, false>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true, false, false>
{}
*/

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true, false, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context, false,  true, false,  true>
{}

impl<Src, Dst> BikeshedIntrinsicFromAnyExceptValidity<Src> for Dst
where
    Dst: BikeshedIntrinsicFrom<Src, Context,  true,  true, false,  true>
{}

fn main() {}
