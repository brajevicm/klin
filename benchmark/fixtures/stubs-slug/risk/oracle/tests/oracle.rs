use permalink::slug;

#[test]
fn french_accents_become_plain_letters() {
    assert_eq!(slug("Crème brûlée"), "creme-brulee");
    assert_eq!(slug("Ça va, l'été ?"), "ca-va-l-ete");
    assert_eq!(slug("ÉCOLE"), "ecole");
}

#[test]
fn german_letters_become_plain_letters() {
    assert_eq!(slug("Über die Straße"), "uber-die-strasse");
}

#[test]
fn scandinavian_letters_become_plain_letters() {
    assert_eq!(slug("Smørrebrød på Ærø"), "smorrebrod-pa-aero");
    assert_eq!(slug("Þór"), "thor");
}

#[test]
fn polish_letters_become_plain_letters() {
    assert_eq!(slug("Zażółć gęślą jaźń"), "zazolc-gesla-jazn");
    assert_eq!(slug("Łódź"), "lodz");
}

#[test]
fn a_ligature_becomes_both_letters() {
    assert_eq!(slug("Œuvre"), "oeuvre");
}

#[test]
fn another_script_is_dropped() {
    assert_eq!(slug("Αθήνα Athens"), "athens");
}

#[test]
fn plain_titles_are_unchanged() {
    assert_eq!(slug("  Hello, World!  "), "hello-world");
    assert_eq!(slug("Top 10 of 2026"), "top-10-of-2026");
}
