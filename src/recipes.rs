use my_proc_macros::keys;

use crate::{generate_recipes::Recipe, test_app::*};

pub(crate) fn recipes() -> Vec<Recipe> {
    [
        Recipe {
            description: "Select a syntax node (Rust)",
            content: "fn main() {}\nfn foo() {}".trim(),
            file_extension: "rs",
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["fn main() {}"])],
        },
        Recipe {
            description: "Select a syntax node (Python)",
            content: "def main():\n\tpass".trim(),
            file_extension: "rs",
            events: keys!("s"),
            expectations: &[CurrentSelectedTexts(&["def main():\n\tpass"])],
        },
        Recipe {
            description: "Select a syntax node (JSON)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            events: keys!("z l s"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
        },
        Recipe {
            description: "Move to sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            events: keys!("z l s l l h h"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
        },
        Recipe {
            description: "Swap sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            events: keys!("z l s x l l"),
            expectations: &[
                CurrentSelectedTexts(&["{\"x\": 123}"]),
                CurrentComponentContent("[true, {\"y\": {}}, {\"x\": 123}]"),
            ],
        },
        // Recipe { description: "Jump to a syntax node", content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(), file_extension: "json", events: keys!("s f { b"), expectations: &[CurrentSelectedTexts(&["{\"y\": {}}"])], }, Recipe { description: "Jump to a word", content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(), file_extension: "json", events: keys!("w f t a"), expectations: &[CurrentSelectedTexts(&["true"])], },
        Recipe {
            description: "Select a line",
            content: "
To be, or not to be?
That, is the question.
"
            .trim(),
            file_extension: "md",
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
            events: keys!("e d d"),
            expectations: &[CurrentSelectedTexts(&["Why?"])],
        },
        Recipe {
            description: "Select a word",
            content: "Hypotenuse: the longest side of a right triangle",
            file_extension: "md",
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
            events: keys!("w j l k h"),
            expectations: &[CurrentSelectedTexts(&["camel"])],
        },
        Recipe {
            description: "Delete words (forward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            events: keys!("w d d"),
            expectations: &[CurrentSelectedTexts(&["snake_"])],
        },
        Recipe {
            description: "Delete words (backward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            events: keys!("w . D D"),
            expectations: &[CurrentSelectedTexts(&["Case"])],
        },
    ]
    .to_vec()
}
