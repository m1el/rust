//! The visibilities of the `Src` type, its fields, and its fields' types (and so on) generally do
//! not matter.

#![feature(transmutability)]
#![allow(dead_code)]

mod assert {
    use std::mem::BikeshedIntrinsicFrom;

    pub fn is_transmutable<Src, Dst, Context>()
    where
        Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
    {}
}

// all involved types and fields are public
mod all_visible {
    mod src {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Src {
            pub(in super) field: Zst,
        }
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Dst {
            pub(in super) field: Zst,
        }
    }

    const _: () = {|| {
        struct Context;
        crate::assert::is_transmutable::<src::Src, dst::Dst, Context>();
    };};
}

// the field of `Src` is private
mod src_field_private {
    mod src {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Src {
            pub(self) field: Zst, // <- private field
        }
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Dst {
            pub(in super) field: Zst,
        }
    }

    const _: () = {|| {
        struct Context;
        crate::assert::is_transmutable::<src::Src, dst::Dst, Context>();
    };};
}

// the type of `Src` is private
mod src_type_private {
    mod src {
        use std::mem::BikeshedIntrinsicFrom;

        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(self) struct Src { // <- private type
            pub(in super) field: Zst,
        }

        pub trait IsTransmutable<Context> {}

        impl<Dst, Context> IsTransmutable<Context> for Dst
        where
            Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        {}

        pub fn is_transmutable<Dst, Context>()
        where
            Dst: IsTransmutable<Context>
        {}
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Dst {
            pub(in super) field: Zst,
        }
    }

    const _: () = {|| {
        pub(self) struct Context;
        src::is_transmutable::<dst::Dst, Context>();
    };};
}

// the type of `Src`'s field is private
mod src_field_type_private {
    mod src {
        #[repr(C)] pub(self) struct Zst; // <- private type

        #[repr(C)] pub(in super) struct Src {
            pub(in super) field: Zst,  //~ ERROR private type `src_field_type_private::src::Zst` in public interface
        }

        use std::mem::BikeshedIntrinsicFrom;

        pub trait IsTransmutable<Context> {}

        impl<Dst, Context> IsTransmutable<Context> for Dst
        where
            Dst: BikeshedIntrinsicFrom<Src, Context, false, false, false, false>
        {}

        pub fn is_transmutable<Dst, Context>()
        where
            Dst: IsTransmutable<Context>
        {}
    }

    mod dst {
        #[repr(C)] pub(in super) struct Zst;

        #[repr(C)] pub(in super) struct Dst {
            pub(in super) field: Zst,
        }
    }

    const _: () = {|| {
        pub(self) struct Context;
        src::is_transmutable::<dst::Dst, Context>();
    };};
}

fn main() {}
