#[test]
fn should_translate_correctly() {
    // let b1 = crate::layout::Tree::Seq(vec![
    //     crate::layout::Tree::from_bits(0),
    //     crate::layout::Tree::from_bits(1),
    // ]);
    // let b2 = crate::layout::Tree::Seq(vec![
    //     crate::layout::Tree::from_bits(1),
    //     crate::layout::Tree::from_bits(1),
    // ]);
    // let b = crate::layout::Tree::<!, !>::Alt(vec![b1, b2]);
    let b = crate::layout::Tree::<!, !>::number(4);
    println!("tree: {:?}\n", b);
    let b = b.de_def();
    println!("tree: {:?}\n", b);
    let b = crate::layout::Nfa::from_tree(b).unwrap();
    println!("nfa: {:#?}\n", b);
    let start = std::time::Instant::now();
    let b = crate::layout::Dfa::from_nfa(b);
    println!("dfa: {:#?} {:?}\n", b, start.elapsed());
    assert!(true);
}
