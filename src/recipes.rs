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
        },
        Recipe {
            description: "Select a syntax node (Python)",
            content: "def main():\n\tpass".trim(),
            file_extension: "rs",
            prepare_events: &[],
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["def main():\n\tpass"])],
        },
        Recipe {
            description: "Select a syntax node (JSON)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
        },
        Recipe {
            description: "Move to sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s l l h h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
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
        },
        Recipe {
            description: "Jump to a syntax node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("s f { b"),
            expectations: &[CurrentSelectedTexts(&["{\"y\": {}}"])],
        },
        Recipe {
            description: "Jump to a word",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("w f t"),
            expectations: &[CurrentSelectedTexts(&["true"])],
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
        },
        Recipe {
            description: "Select a word",
            content: "Hypotenuse: the longest side of a right triangle",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w"),
            expectations: &[CurrentSelectedTexts(&["Hypotenuse"])],
        },
        Recipe {
            description: "Moving words",
            content: "
camelCase
hello_world
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w j l k h"),
            expectations: &[CurrentSelectedTexts(&["camel"])],
        },
        Recipe {
            description: "Delete words (forward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w d d"),
            expectations: &[CurrentSelectedTexts(&["snake_"])],
        },
        Recipe {
            description: "Delete words (backward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w . D D"),
            expectations: &[CurrentSelectedTexts(&["Case"])],
        },
        Recipe {
            description: "Undo & Redo",
            content: "camelCase".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w d u U"),
            expectations: &[CurrentComponentContent("Case")],
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
            events: keys!("w ] c q l l esc a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foo bar spam",
            )],
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
            events: keys!("w ] c space a a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foox bar spam",
            )],
        },
        Recipe {
            description: "Surround",
            content: "hello world".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e v s ("),
            expectations: &[CurrentComponentContent("(hello world)")],
        },
        Recipe {
            description: "Delete Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v d ("),
            expectations: &[CurrentComponentContent("hello world")],
        },
        Recipe {
            description: "Change Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v c ( {"),
            expectations: &[CurrentComponentContent("{hello world}")],
        },
        Recipe {
            description: "Select Inside Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v i ("),
            expectations: &[CurrentSelectedTexts(&["hello world"])],
        },
        Recipe {
            description: "Select Around Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("z l"),
            events: keys!("v a ("),
            expectations: &[CurrentSelectedTexts(&["(hello world)"])],
        },
        Recipe {
            description: "Extend selection (Word)",
            content: "foo bar spam pi".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w v l l h"),
            expectations: &[CurrentSelectedTexts(&["foo bar"])],
        },
        Recipe {
            description: "Extend selection (Syntax Node)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("z l"),
            events: keys!("s v l l h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}, true"])],
        },
        Recipe {
            description: "Extend selection (Switch Direction)",
            content: "foo bar spam baz tim".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w l l v l o h o l"),
            expectations: &[CurrentSelectedTexts(&["bar spam baz tim"])],
        },
    ]
    .to_vec()
}
