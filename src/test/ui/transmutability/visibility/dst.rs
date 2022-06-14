//! The destination type, its fields, and its fields' types (and so on) must be visible from
//! `Context`.

#![feature(transmutability)]
#![allow(dead_code)]

// the field of `Dst` is private
mod dst_field_private {
    mod assert {
        use std::mem::BikeshedIntrinsicFrom;

        pub fn is_transmutable<Src, Dst, Context>()
        where
            Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        {}
    }

    mod src {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Src {
            pub(in super) field: Zst,
        }
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Dst {
            pub(self) field: Zst, // <- private field
        }
    }

    const _: () = {|| {
        struct Context;
        assert::is_transmutable::<src::Src, dst::Dst, Context>(); //~ ERROR not satisfied
    };};
}

// the type of `Dst` is private
mod dst_type_private {
    mod src {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Src {
            pub(in super) field: Zst,
        }
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(self) struct Dst { // <- private type
            pub(in super) field: Zst,
        }

        use std::mem::BikeshedIntrinsicFrom;

        pub trait IsTransmutable<Src, Context> {}

        impl<Src, Context> IsTransmutable<Src, Context> for Dst
        where
            Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        {}

        pub fn is_transmutable<Src, Context>()
        where
            Dst: IsTransmutable<Src, Context>
        {}
    }

    const _: () = {|| {
        pub(self) struct Context;
        dst::is_transmutable::<src::Src, Context>(); //~ ERROR not satisfied
    };};
}

// the type of `Dst`'s field is private
mod dst_field_type_private {
    mod src {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Src {
            pub(in super) field: Zst,
        }
    }

    mod dst {
        #[repr(C)] pub(self) struct Zst; // <- private type

        #[repr(C)] pub(in super) struct Dst {
            pub(in super) field: Zst,
        }

        use std::mem::BikeshedIntrinsicFrom;

        pub trait IsTransmutable<Src, Context> {}

        impl<Src, Context> IsTransmutable<Src, Context> for Dst
        where
            Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        {}

        pub fn is_transmutable<Src, Context>()
        where
            Dst: IsTransmutable<Src, Context>
        {}
    }

    const _: () = {|| {
        pub(self) struct Context;
        dst::is_transmutable::<src::Src, Context>(); //~ ERROR not satisfied
    };};
}

fn main() {}
