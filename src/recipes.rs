use my_proc_macros::keys;

use crate::{generate_recipes::Recipe, test_app::*};

pub(crate) fn recipes() -> Vec<Recipe> {
    [
        Recipe {
            description: "Select a syntax node (Rust)",
            content: "fn main() {}\nfn foo() {}".trim(),
            file_extension: "rs",
            prepare_events: &[],
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["fn main() {}"])],
            terminal_height: None,
        },
        Recipe {
            description: "Select a syntax node (Python)",
            content: "def main():\n\tpass".trim(),
            file_extension: "rs",
            prepare_events: &[],
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["def main():\n\tpass"])],
            terminal_height: None,
        },
        Recipe {
            description: "Select a syntax node (JSON)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
            terminal_height: None,
        },
        Recipe {
            description: "Move to sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s l l h h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
            terminal_height: None,
        },
        Recipe {
            description: "Swap sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s x l l"),
            expectations: &[
                CurrentSelectedTexts(&["{\"x\": 123}"]),
                CurrentComponentContent("[true, {\"y\": {}}, {\"x\": 123}]"),
            ],
            terminal_height: None,
        },
        Recipe {
            description: "Swap sibling node",
            content: "<x><y>foo</y><div/></x>".trim(),
            file_extension: "xml",
            prepare_events: keys!("z l l l"),
            events: keys!("s j x l"),
            expectations: &[
                CurrentSelectedTexts(&["<y>foo</y>"]),
                CurrentComponentContent("<x><div/><y>foo</y></x>"),
            ],
            terminal_height: None,
        },
        Recipe {
            description: "Jump to a syntax node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("s f { b"),
            expectations: &[CurrentSelectedTexts(&["{\"y\": {}}"])],
            terminal_height: None,
        },
        Recipe {
            description: "Expand selection / Select Parent",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l l"),
            events: keys!("S k k k k"),
            expectations: &[CurrentSelectedTexts(&["[{\"x\": 123}, true, {\"y\": {}}]"])],
            terminal_height: None,
        },
        Recipe {
            description: "Shrink selection / Select First-Child",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("s"),
            events: keys!("s j j j j"),
            expectations: &[CurrentSelectedTexts(&["x"])],
            terminal_height: None,
        },
        Recipe {
            description: "Jump to a word",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("w f t"),
            expectations: &[CurrentSelectedTexts(&["true"])],
            terminal_height: None,
        },
        Recipe {
            description: "Select a line",
            content: "
To be, or not to be?
That, is the question.
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e"),
            expectations: &[CurrentSelectedTexts(&["To be, or not to be?"])],
            terminal_height: None,
        },
        Recipe {
            description: "Delete lines",
            content: "
To be, or not to be?
That, is the question.
Why?
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e d d"),
            expectations: &[CurrentSelectedTexts(&["Why?"])],
            terminal_height: None,
        },
        Recipe {
            description: "Word movement (skips symbols)",
            content: "hello-world camelCase snake_case",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w l l"),
            expectations: &[CurrentSelectedTexts(&["snake_case"])],
            terminal_height: None,
        },
        Recipe {
            description: "Sub words movement",
            content: "
camelCase
hello_world
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b j l k h"),
            expectations: &[CurrentSelectedTexts(&["camel"])],
            terminal_height: None,
        },
        Recipe {
            description: "Delete sub words (forward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b d d"),
            expectations: &[CurrentSelectedTexts(&["snake_"])],
            terminal_height: None,
        },
        Recipe {
            description: "Delete sub words (backward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b . D D"),
            expectations: &[CurrentSelectedTexts(&["Case"])],
            terminal_height: None,
        },
        Recipe {
            description: "Undo & Redo",
            content: "camelCase".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b d u U"),
            expectations: &[CurrentComponentContent("Case")],
            terminal_height: None,
        },
        Recipe {
            description: "Surround",
            content: "hello world".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e v s ("),
            expectations: &[CurrentComponentContent("(hello world)")],
            terminal_height: None,
        },
        Recipe {
            description: "Delete Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v d ("),
            expectations: &[CurrentComponentContent("hello world")],
            terminal_height: None,
        },
        Recipe {
            description: "Change Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v c ( {"),
            expectations: &[CurrentComponentContent("{hello world}")],
            terminal_height: None,
        },
        Recipe {
            description: "Select Inside Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v i ("),
            expectations: &[CurrentSelectedTexts(&["hello world"])],
            terminal_height: None,
        },
        Recipe {
            description: "Select Around Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v a ("),
            expectations: &[CurrentSelectedTexts(&["(hello world)"])],
            terminal_height: None,
        },
        Recipe {
            description: "Extend selection (Word)",
            content: "foo bar spam pi".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w v l l h"),
            expectations: &[CurrentSelectedTexts(&["foo bar"])],
            terminal_height: None,
        },
        Recipe {
            description: "Extend selection (Syntax Node)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s v l l h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}, true"])],
            terminal_height: None,
        },
        Recipe {
            description: "Extend selection (Switch Direction)",
            content: "foo bar spam baz tim".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w l l v l o h o l"),
            expectations: &[CurrentSelectedTexts(&["bar spam baz tim"])],
            terminal_height: None,
        },
        Recipe {
            description: "Default Search (literal, no escaping needed)",
            content: "foo bar (x) baz (x)".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ ( x ) enter l h"),
            expectations: &[CurrentSelectedTexts(&["(x)"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search (literal, match whole word)",
            content: "fobar fo spamfo fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter ' w l h"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search (literal, case-sensitive)",
            content: "fo Fo fo Fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ F o enter ' i l h"),
            expectations: &[CurrentSelectedTexts(&["Fo"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search current selection",
            content: "fo ba fo ba".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w n l"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search current selection (works for multiple lines too)",
            content: "
foo
  .bar()
  
spam()

foo
  .bar()
"
            .trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("s n l"),
            expectations: &[CurrentSelectedTexts(&["foo\n  .bar()"])],
            terminal_height: Some(14),
        },
        Recipe {
            description: "Search (regex)",
            content: "a (foo ba) bar".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ backslash ( . * backslash ) enter ' x"),
            expectations: &[CurrentSelectedTexts(&["(foo ba)"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search (case agnostic)",
            content: "foBa x fo_ba x fo ba x fo-ba".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f o space b a enter ' c l"),
            expectations: &[CurrentSelectedTexts(&["fo-ba"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search (AST Grep)",
            content: "f(1+1); f(x); f('f()')".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f ( $ X ) enter ' a"),
            expectations: &[CurrentSelectedTexts(&["f(1+1)"])],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search & Replace Multi-cursor",
            content: "fo x fo x fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter q l esc ' r b a enter ctrl+c ctrl+r"),
            expectations: &[CurrentComponentContent("ba x ba x fo")],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search & Replace All (regex)",
            content: "1 x 2".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ ( backslash d ) enter ' x ' r ( $ 1 ) enter R"),
            expectations: &[CurrentComponentContent("(1) x (2)")],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search & Replace All (case agnostic)",
            content: "foBa x fo_ba x fo ba x fo-ba".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f o space b a enter ' c ' r k a _ t o enter R"),
            expectations: &[CurrentComponentContent("kaTo x ka_to x ka to x ka-to")],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Search & Replace All (AST Grep)",
            content: "f(1+1); f(x); f('f()')".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f ( $ X ) enter ' a ' r ( $ X ) . z enter R"),
            expectations: &[CurrentComponentContent("(1+1).z; (x).z; ('f()').z")],
            terminal_height: Some(7),
        },
        Recipe {
            description: "Repeat last non-contigous selection mode",
            content: "fo world fo where".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter z d ;"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: None,
        },
        Recipe {
            description: "Multi-cursor: add using movement",
            content: "
foo bar spam
spam foo bar
bar spam foo
foo bar spam
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w n q l l esc a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foo bar spam",
            )],
            terminal_height: None,
        },
        Recipe {
            description: "Multi-cursor: Select all matches",
            content: "
foo bar spam
spam foo bar
bar spam foo
foo bar spam
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w n space a a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foox bar spam",
            )],
            terminal_height: None,
        },
        Recipe {
            description: "Move the first two elements to the last",
            content: "[{\"a\": b}, \"c\", [], {}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s v l y d . p"),
            expectations: &[CurrentComponentContent("[[], {}, {\"a\": b}, \"c\"]")],
            terminal_height: None,
        },
        Recipe {
            description: "Change the first two word",
            content: "This is am Ki".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w d c I esc"),
            expectations: &[CurrentComponentContent("I am Ki")],
            terminal_height: None,
        },
    ]
    .to_vec()
}
