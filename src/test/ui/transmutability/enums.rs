#![feature(transmutability)]
#![feature(marker_trait_attr)]
#![allow(dead_code)]
#![allow(incomplete_features)]
#![allow(conflicting_repr_hints)]

fn test_well_defined() {
    #[repr(C, u8)]
    pub enum WellDefined { A }
    pub enum Unspecified { A }

    assert_is_transmutable_assume_nothing::<WellDefined, WellDefined>();
    assert_is_transmutable_assume_nothing::<WellDefined, Unspecified>(); //~ ERROR not satisfied
    assert_is_transmutable_assume_nothing::<Unspecified, WellDefined>(); //~ ERROR not satisfied
    assert_is_transmutable_assume_nothing::<Unspecified, Unspecified>(); //~ ERROR not satisfied
}

fn test_read_tag() {
    #[repr(C, u8)]
    pub enum OnlyTag { A }

    assert_is_transmutable_assume_nothing::<OnlyTag, u8>();
    // cannot write to tag without verifying
    assert_is_transmutable_assume_nothing::<u8, OnlyTag>(); //~ ERROR not satisfied
    // can write to tag if validity is checked
    assert_is_transmutable_assume_validity::<u8, OnlyTag>();
}

fn test_with_data() {
    #[repr(C, u8)]
    pub enum TwoU8 { A(u8), B(u8) }
    #[repr(C)]
    pub struct EquivalentStruct { tag: bool, value: u8 }
    assert_is_transmutable_assume_nothing::<TwoU8, EquivalentStruct>();
    assert_is_transmutable_assume_nothing::<EquivalentStruct, TwoU8>();
}

fn test_fail_with_data() {
    #[repr(C, u8)]
    pub enum TwoU8 { A(u8), B(bool) }
    #[repr(C)]
    pub struct Same { tag: bool, value: u8 }
    assert_is_transmutable_assume_nothing::<TwoU8, Same>();
    assert_is_transmutable_assume_nothing::<Same, TwoU8>(); //~ ERROR not satisfied
}

fn test_cursed_enum() {
    #[repr(C, u8)] enum EnumA { A(bool), B(u8), }
    #[repr(C)] struct StructA { a: bool, b: EnumA }

    #[repr(C, u8)] enum EnumB { A(bool), B(bool) }
    #[repr(C)] struct StructB { a: EnumB, b: bool }
    assert_is_transmutable_assume_nothing::<StructA, StructB>(); //~ ERROR not satisfied
    assert_is_transmutable_assume_nothing::<StructB, StructA>();
}

use std::mem::BikeshedIntrinsicFrom;

struct Context;

fn assert_is_transmutable_assume_nothing<Src, Dst>()
where
    Dst:
        BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
{}

fn assert_is_transmutable_assume_validity<Src, Dst>()
where
    Dst:
        BikeshedIntrinsicFrom<Src, Context, false, false, true, false>
{}

fn main() {}
