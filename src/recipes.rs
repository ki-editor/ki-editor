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
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Select a syntax node (Python)",
            content: "def main():\n\tpass".trim(),
            file_extension: "rs",
            prepare_events: &[],
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["def main():\n\tpass"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Select a syntax node (JSON)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Move to sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s l l h h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
            terminal_height: None,
            similar_vim_combos: &[],
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
            similar_vim_combos: &[],
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
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Jump to a syntax node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("s f { b"),
            expectations: &[CurrentSelectedTexts(&["{\"y\": {}}"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Expand selection / Select Parent",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l l"),
            events: keys!("S k k k k"),
            expectations: &[CurrentSelectedTexts(&["[{\"x\": 123}, true, {\"y\": {}}]"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Shrink selection / Select First-Child",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("s"),
            events: keys!("s j j j j"),
            expectations: &[CurrentSelectedTexts(&["x"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Jump to a word",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("w f t"),
            expectations: &[CurrentSelectedTexts(&["true"])],
            terminal_height: None,
            similar_vim_combos: &[],
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
            similar_vim_combos: &["V"],
        },
        Recipe {
            description: "Duplicate current line",
            content: "
To be, or not to be?
That, is the question.
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e y p"),
            expectations: &[CurrentComponentContent(
                "To be, or not to be?
To be, or not to be?
That, is the question.",
            )],
            terminal_height: None,
            similar_vim_combos: &["y y p"],
        },
        Recipe {
            description: "Go to first and last line",
            content: "
To be, or not to be, that is the question:
Whether 'tis nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles
And by opposing end them. To die—to sleep,
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e . ,"),
            expectations: &[CurrentSelectedTexts(&[
                "To be, or not to be, that is the question:",
            ])],
            terminal_height: None,
            similar_vim_combos: &["g g", "G"],
        },
        Recipe {
            description: "Select every line",
            content: "
To be, or not to be, that is the question:
Whether 'tis nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles
And by opposing end them. To die—to sleep,
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e V"),
            expectations: &[CurrentSelectedTexts(&[
                "To be, or not to be, that is the question:
Whether 'tis nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles
And by opposing end them. To die—to sleep,",
            ])],
            terminal_height: None,
            similar_vim_combos: &["g g V G"],
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
            similar_vim_combos: &["d d", "d j"],
        },
        Recipe {
            description: "Insert at the beginning of line",
            content: "  hat is that?",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e i W"),
            expectations: &[CurrentComponentContent("  What is that?")],
            terminal_height: None,
            similar_vim_combos: &["I"],
        },
        Recipe {
            description: "Insert at the end of line",
            content: "  What is that",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e a ?"),
            expectations: &[CurrentComponentContent("  What is that?")],
            terminal_height: None,
            similar_vim_combos: &["A"],
        },
        Recipe {
            description: "Word movement",
            content: "hello-world camelCase snake_case",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w l l"),
            expectations: &[CurrentSelectedTexts(&["snake_case"])],
            terminal_height: None,
            similar_vim_combos: &["w", "e", "b"],
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
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Delete sub words (forward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b d d"),
            expectations: &[CurrentSelectedTexts(&["snake_"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Delete sub words (backward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b . D D"),
            expectations: &[CurrentSelectedTexts(&["Case"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Undo & Redo",
            content: "camelCase".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("b d u U"),
            expectations: &[CurrentComponentContent("Case")],
            terminal_height: None,
            similar_vim_combos: &["u", "ctrl+r"],
        },
        Recipe {
            description: "Surround",
            content: "hello world".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e v s ("),
            expectations: &[CurrentComponentContent("(hello world)")],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Delete Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v d ("),
            expectations: &[CurrentComponentContent("hello world")],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Change Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v c ( {"),
            expectations: &[CurrentComponentContent("{hello world}")],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Select Inside Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v i ("),
            expectations: &[CurrentSelectedTexts(&["hello world"])],
            terminal_height: None,
            similar_vim_combos: &["v i ("],
        },
        Recipe {
            description: "Select Around Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v a ("),
            expectations: &[CurrentSelectedTexts(&["(hello world)"])],
            terminal_height: None,
            similar_vim_combos: &["v a ("],
        },
        Recipe {
            description: "Extend selection (Word)",
            content: "foo bar spam pi".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w v l l h"),
            expectations: &[CurrentSelectedTexts(&["foo bar"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Extend selection (Syntax Node)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s v l l h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}, true"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Extend selection (Switch Direction)",
            content: "foo bar spam baz tim".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w l l v l o h o l"),
            expectations: &[CurrentSelectedTexts(&["bar spam baz tim"])],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Default Search (literal, no escaping needed)",
            content: "foo bar (x) baz (x)".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ ( x ) enter l h"),
            expectations: &[CurrentSelectedTexts(&["(x)"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search (literal, match whole word)",
            content: "fobar fo spamfo fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter ' w l h"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search (literal, case-sensitive)",
            content: "fo Fo fo Fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ F o enter ' i l h"),
            expectations: &[CurrentSelectedTexts(&["Fo"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search current selection",
            content: "fo ba fo ba".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w n l"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
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
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search (regex)",
            content: "a (foo ba) bar".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ backslash ( . * backslash ) enter ' x"),
            expectations: &[CurrentSelectedTexts(&["(foo ba)"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search (case agnostic)",
            content: "foBa x fo_ba x fo ba x fo-ba".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f o space b a enter ' c l"),
            expectations: &[CurrentSelectedTexts(&["fo-ba"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search (AST Grep)",
            content: "f(1+1); f(x); f('f()')".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f ( $ X ) enter ' a"),
            expectations: &[CurrentSelectedTexts(&["f(1+1)"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search & Replace Multi-cursor",
            content: "fo x fo x fo".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter q l esc ' r b a enter ctrl+c ctrl+r"),
            expectations: &[CurrentComponentContent("ba x ba x fo")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search & Replace All (regex)",
            content: "1 x 2".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ ( backslash d ) enter ' x ' r ( $ 1 ) enter R"),
            expectations: &[CurrentComponentContent("(1) x (2)")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search & Replace All (case agnostic)",
            content: "foBa x fo_ba x fo ba x fo-ba".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f o space b a enter ' c ' r k a _ t o enter R"),
            expectations: &[CurrentComponentContent("kaTo x ka_to x ka to x ka-to")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Search & Replace All (AST Grep)",
            content: "f(1+1); f(x); f('f()')".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f ( $ X ) enter ' a ' r ( $ X ) . z enter R"),
            expectations: &[CurrentComponentContent("(1+1).z; (x).z; ('f()').z")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Repeat last non-contigous selection mode",
            content: "fo world fo where".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ f o enter z d ;"),
            expectations: &[CurrentSelectedTexts(&["fo"])],
            terminal_height: None,
            similar_vim_combos: &[],
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
            similar_vim_combos: &[],
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
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Move the first two elements to the last",
            content: "[{\"a\": b}, \"c\", [], {}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s v l y d . p"),
            expectations: &[CurrentComponentContent("[[], {}, {\"a\": b}, \"c\"]")],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Change the first two word",
            content: "This is am Ki".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w d c I esc"),
            expectations: &[CurrentComponentContent("I am Ki")],
            terminal_height: None,
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Swap distant expressions using jump",
            content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
            file_extension: "js",
            prepare_events: keys!("/ x enter"),
            events: keys!("s x f ' a"),
            expectations: &[CurrentComponentContent(
                "if(condition) { 'hello world' } else { x(bar(baz)) }",
            )],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Replace parent node with current node",
            content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
            file_extension: "js",
            prepare_events: keys!("/ x enter"),
            events: keys!("s y k k r"),
            expectations: &[CurrentComponentContent("x(bar(baz))")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Remove all sibling nodes except the current node",
            content: "[foo(), {xar: 'spam'}, baz + baz]".trim(),
            file_extension: "js",
            prepare_events: keys!("/ { enter"),
            events: keys!("s y V r"),
            expectations: &[CurrentComponentContent("[{xar: 'spam'}]")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
        },
        Recipe {
            description: "Save",
            content: "hello world".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("enter"),
            expectations: &[],
            terminal_height: None,
            similar_vim_combos: &[": w enter"],
        },
        Recipe {
            description: "Switch view alignment",
            content: "
Who lives in a pineapple under the sea?


Absorbent and yellow and porous is he?


If nautical nonsense be something you wish?


And drop on the deck and flop like a fish? 
"
            .trim(),
            file_extension: "md",
            prepare_events: keys!("/ i f enter e"),
            events: keys!("ctrl+l ctrl+l ctrl+l"),
            expectations: &[CurrentSelectedTexts(&[
                "If nautical nonsense be something you wish?",
            ])],
            terminal_height: Some(8),
            similar_vim_combos: &["z t", "z z", "z b"],
        },
        Recipe {
            description: "Replace cut",
            content: "
foo(bar, 1 + 1, spam)
3 * 10
"
            .trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("s y l R j l j l r"),
            expectations: &[CurrentComponentContent(
                "foo(bar, 1 + 1, spam)
foo(bar, 3 * 10, spam)",
            )],
            terminal_height: None,
            similar_vim_combos: &["p"],
        },
    ]
    .to_vec()
}
