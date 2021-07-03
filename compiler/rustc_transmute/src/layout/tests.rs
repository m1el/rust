#[test]
fn should_translate_correctly() {
    let b = crate::layout::Tree::<!, !>::bool();
    println!("tree: {:?}\n", b);
    let b = b.de_def();
    println!("tree: {:?}\n", b);
    let b = crate::layout::Nfa::from_tree(b).unwrap();
    println!("nfa: {:#?}\n", b);
    let b = crate::layout::Dfa::from_nfa(b);
    println!("dfa: {:#?}\n", b);
    assert!(true);
}
