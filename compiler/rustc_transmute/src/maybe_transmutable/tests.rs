use super::query_context::test::UltraMinimal;
use crate::{layout, Answer};

mod bool {
    use super::*;

    #[test]
    fn should_permit_identity_transmutation_tree() {
        println!("{:?}", layout::Tree::<!, !>::bool());
        let answer = crate::maybe_transmutable::MaybeTransmutableQuery::new(
            layout::Tree::<!, !>::bool(),
            layout::Tree::<!, !>::bool(),
            (),
            crate::Assume { alignment: false, lifetimes: false, validity: true, visibility: false },
            UltraMinimal,
        )
        .answer();
        assert_eq!(answer, Answer::Yes);
    }

    #[test]
    #[ignore]
    fn should_permit_identity_transmutation_dfa() {
        let answer = crate::maybe_transmutable::MaybeTransmutableQuery::new(
            layout::Dfa::<!>::bool(),
            layout::Dfa::<!>::bool(),
            (),
            crate::Assume { alignment: false, lifetimes: false, validity: true, visibility: false },
            UltraMinimal,
        )
        .answer();
        assert_eq!(answer, Answer::Yes);
    }
}
