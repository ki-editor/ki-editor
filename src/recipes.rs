use my_proc_macros::keys;

use crate::{
    components::editor::Mode,
    generate_recipes::{Recipe, RecipeGroup},
    position::Position,
    test_app::*,
};

pub(crate) fn recipe_groups() -> Vec<RecipeGroup> {
    [
        swap_cursors(),
        swap_current_selection_using_a_different_selection_mode(),
        reveal_selections(),
        reveal_cursors(),
        reveal_marks(),
        showcase(),
        syntax_node(),
        multicursors(),
        RecipeGroup {
            filename: "align-view",
            recipes: [
                Recipe {
                    description: "Align view",
                    content: "
fn main() {
// padding top 1
// padding top 2
// padding top 3
	foo {
        bar: spam
    }
// padding bottom 4
// padding bottom 5
// padding bottom 6
}".trim(),
                    file_extension: "rs",
                    prepare_events: keys!("q f o o enter d"),
                    events: keys!("alt+; alt+; alt+;"),
                    expectations: Box::new([CurrentSelectedTexts(&["foo {\n        bar: spam\n    }"])]),
                    terminal_height: Some(9),
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "jump",
            recipes: [
                Recipe {
                    description: "Jump to a subword",
                    content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                    file_extension: "json",
                    prepare_events: &[],
                    events: keys!("w m t"),
                    expectations: Box::new([CurrentSelectedTexts(&["true"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Jump to a syntax node",
                    content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                    file_extension: "json",
                    prepare_events: &[],
                    events: keys!("d m { k"),
                    expectations: Box::new([CurrentSelectedTexts(&["{\"y\": {}}"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "line",
            recipes: [
            Recipe {
                description: "Select a line",
                content: "
To be, or not to be?
That, is the question.
"
                .trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("a"),
                expectations: Box::new([CurrentSelectedTexts(&["To be, or not to be?"])]),
                terminal_height: None,
                similar_vim_combos: &["V"],
                only: false,
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
                events: keys!("a p y"),
                expectations: Box::new([CurrentSelectedTexts(&[
                    "To be, or not to be, that is the question:",
                ])]),
                terminal_height: None,
                similar_vim_combos: &["g g", "G"],
                only: false,
            },
            Recipe {
                description: "Go to empty lines",
                content: "
foo


bar


spam


baz"
                .trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("a i i i k k"),
                expectations: Box::new([CurrentSelectedTexts(&[
                    "",
                ])]),
                terminal_height: None,
                similar_vim_combos: &["{", "}"],
                only: false,
            },
            Recipe {
                description: "Insert at the beginning of line",
                content: "  hat is that?",
                file_extension: "md",
                prepare_events: &[],
                events: keys!("a l h W"),
                expectations: Box::new([CurrentComponentContent("  What is that?")]),
                terminal_height: None,
                similar_vim_combos: &["I"],
                only: false,
            },
            Recipe {
                description: "Insert at the end of line",
                content: "  What is that",
                file_extension: "md",
                prepare_events: &[],
                events: keys!("a l ; ?"),
                expectations: Box::new([CurrentComponentContent("  What is that?")]),
                terminal_height: None,
                similar_vim_combos: &["A"],
                only: false,
            }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "delete",
            recipes: [
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
                    events: keys!("a v l v l"),
                    expectations: Box::new([CurrentSelectedTexts(&["Why?"]), CurrentComponentContent("Why?")]),
                    terminal_height: None,
                    similar_vim_combos: &["d d", "d j"],
                    only: false,
                },
                Recipe {
                    description: "Delete subword (forward)",
                    content: "snake_case kebab-case".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w v l v l"),
                    expectations: Box::new([CurrentSelectedTexts(&["kebab"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete word (backward)",
                    content: "camelCase snake_case PascalCase".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l l v j v j"),
                    expectations: Box::new([CurrentSelectedTexts(&["camelCase"]), CurrentComponentContent("camelCase")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete forward: auto backward at the end",
                    content: "foo bar spam".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l l v l v l"),
                    expectations: Box::new([CurrentSelectedTexts(&["foo"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete backward: auto backward at the beginning",
                    content: "foo bar spam".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s v j v j"),
                    expectations: Box::new([CurrentSelectedTexts(&["spam"]), CurrentComponentContent("spam")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete sibling nodes",
                    content: "[{foo: bar}, spam, 1 + 1]".trim(),
                    file_extension: "js",
                    prepare_events: keys!("w o"),
                    events: keys!("d v l v l"),
                    expectations: Box::new([CurrentSelectedTexts(&["1 + 1"]), CurrentComponentContent("[1 + 1]")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete multiple selections with Extend",
                    content: "[{foo: bar}, spam, 1 + 1]".trim(),
                    file_extension: "js",
                    prepare_events: keys!("w o"),
                    events: keys!("d g l v l"),
                    expectations: Box::new([CurrentSelectedTexts(&["1 + 1"]), CurrentComponentContent("[1 + 1]")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "align-selections",
            recipes: [
                Recipe {
                    description: "Align Left",
                    content: "
====
  1)
 20)
300)
"
                        .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a l r l l esc shift+Y"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["1)", "20)", "300)"]),
                        CurrentComponentContent("
====
  1)
  20)
  300)
"
                            .trim()
                        )
                    ]),
                    terminal_height: Some(10),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Align Right",
                    content: "
====
1)
20)
300)
"
                        .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a l r l l esc shift+P"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["1)", "20)", "300)"]),
                        CurrentComponentContent("
====
  1)
 20)
300)
"
                            .trim()
                        )
                    ]),
                    terminal_height: Some(10),
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "delete-finer-movement",
            recipes: [
                Recipe {
                    description: "Delete word",
                    content: "hello  world"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s v o"),
                    expectations: Box::new([CurrentSelectedTexts(&["world"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete subword",
                    content: "kebab-case".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w v o v o"),
                    expectations: Box::new([CurrentSelectedTexts(&["case"]), CurrentComponentContent("case")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete sibling nodes",
                    content: "[{foo: bar}, spam, 1 + 1]".trim(),
                    file_extension: "js",
                    prepare_events: keys!("w o"),
                    events: keys!("d v o v o v o"),
                    expectations: Box::new([CurrentSelectedTexts(&[","]), CurrentComponentContent("[, 1 + 1]")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "delete-submode",
            recipes: [
                Recipe {
                    description: "Delete multiple words",
                    content: "foo bar spam baz"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s v g l l space"),
                    expectations: Box::new([CurrentSelectedTexts(&["spam"]), CurrentComponentContent("spam baz")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete multiple lines",
                    content: "foo\nbar\nspam\nbaz"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a v g l l space"),
                    expectations: Box::new([CurrentSelectedTexts(&["spam"]), CurrentComponentContent("spam\nbaz")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "delete-one",
            recipes: [
                Recipe {
                    description: "Delete One",
                    content: "hello world"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s v v"),
                    expectations: Box::new([CurrentSelectedTexts(&[" "])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "extend",
            recipes: [
                Recipe {
                    description: "Extend selection (Subword)",
                    content: "foo bar spam pi".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w g l l j"),
                    expectations: Box::new([CurrentSelectedTexts(&["foo bar"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Extend selection (Syntax Node)",
                    content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                    file_extension: "json",
                    prepare_events: keys!("w o"),
                    events: keys!("d g l l j"),
                    expectations: Box::new([CurrentSelectedTexts(&["{\"x\": 123}, true"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Extend selection (Switch Direction)",
                    content: "foo bar spam baz tim".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w l l g l ? j ? l"),
                    expectations: Box::new([CurrentSelectedTexts(&["bar spam baz tim"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Extend selection (Change selection mode)",
                    content: "fooBar helloWorldSpamSpam tada".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s g l w l"),
                    expectations: Box::new([CurrentSelectedTexts(&["fooBar helloWorld"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "surround",
            recipes: [
                Recipe {
                    description: "Surround",
                    content: "hello world".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a g , j"),
                    expectations: Box::new([CurrentComponentContent("(hello world)")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Surround with XML tag",
                    content: "hello world".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s g , p x y enter"),
                    expectations: Box::new([
                        CurrentComponentContent("<xy>hello</xy> world"),
                        CurrentSelectedTexts(&["<xy>hello</xy>"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Delete Surround",
                    content: "(hello world)".trim(),
                    file_extension: "md",
                    prepare_events: keys!("w o"),
                    events: keys!("g v j"),
                    expectations: Box::new([CurrentComponentContent("hello world")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Change Surround",
                    content: "(hello world)".trim(),
                    file_extension: "md",
                    prepare_events: keys!("w o"),
                    events: keys!("g f j l"),
                    expectations: Box::new([CurrentComponentContent("{hello world}")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Select Inside Enclosures",
                    content: "(hello world)".trim(),
                    file_extension: "md",
                    prepare_events: keys!("w o"),
                    events: keys!("g h j"),
                    expectations: Box::new([CurrentSelectedTexts(&["hello world"])]),
                    terminal_height: None,
                    similar_vim_combos: &["v i ("],
                    only: false,
                },
                Recipe {
                    description: "Select Around Enclosures",
                    content: "(hello world)".trim(),
                    file_extension: "md",
                    prepare_events: keys!("w o"),
                    events: keys!("g ; j"),
                    expectations: Box::new([CurrentSelectedTexts(&["(hello world)"])]),
                    terminal_height: None,
                    similar_vim_combos: &["v a ("],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "replace-cut",
            recipes: [
                Recipe {
                    description: "Replace cut",
                    content: "
foo(bar, 1 + 1, spam)
3 * 10
"
                    .trim(),
                    file_extension: "js",
                    prepare_events: &[],
                    events: keys!("d c l C k l k l x"),
                    expectations: Box::new([CurrentComponentContent( "foo(bar, 1 + 1, spam)\nfoo(bar, 3 * 10, spam)", )]),
                    terminal_height: None,
                    similar_vim_combos: &["p"],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "paste",
            recipes: [
                Recipe {
                    description: "Paste forward",
                    content: "foo bar spam"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l c j b"),
                    expectations: Box::new([CurrentComponentContent("foo bar bar spam")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Paste backward",
                    content: "foo bar spam"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l c j / b"),
                    expectations: Box::new([CurrentComponentContent("bar foo bar spam")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Paste with automatic gap insertion (Line)",
                    content: "
foo bar
spam baz
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a c b"),
                    expectations: Box::new([CurrentComponentContent("foo bar\nfoo bar\nspam baz")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Paste with automatic gap insertion (Syntax Node)",
                    content: "function foo(bar: Bar, spam: Spam) {}",
                    file_extension: "ts",
                    prepare_events: keys!("q b a r enter"),
                    events: keys!("d c b"),
                    expectations: Box::new([CurrentComponentContent("function foo(bar: Bar, bar: Bar, spam: Spam) {}")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Paste without automatic gap insertion",
                    content: "foo bar".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a c B"),
                    expectations: Box::new([CurrentComponentContent("foo barfoo bar")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ].to_vec(),
        },
        RecipeGroup {
            filename: "join",
            recipes: [Recipe {
                description: "Example",
                content: "foo\n    bar spam"
                .trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("s l l shift+I"),
                expectations: Box::new([CurrentSelectedTexts(&["spam"]), CurrentComponentContent("foobar spam")]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "break",
            recipes: [Recipe {
                description: "Example",
                content: "
def foo():
    bar = 1; spam = 2;"
                .trim(),
                file_extension: "md",
                prepare_events: keys!("q s p a m enter"),
                events: keys!("K K"),
                expectations: Box::new([CurrentSelectedTexts(&["spam"]), CurrentComponentContent("def foo():
    bar = 1;
    
    spam = 2;")]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "swap",
            recipes: [
                Recipe {
                    description: "Swap sibling node (JSON)",
                    content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                    file_extension: "json",
                    prepare_events: keys!("w o"),
                    events: keys!("d t l l"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["{\"x\": 123}"]),
                        CurrentComponentContent("[true, {\"y\": {}}, {\"x\": 123}]"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap sibling node (XML)",
                    content: "<x><y>foo</y><div/></x>".trim(),
                    file_extension: "xml",
                    prepare_events: keys!("w o o o"),
                    events: keys!("d k t l"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["<y>foo</y>"]),
                        CurrentComponentContent("<x><div/><y>foo</y></x>"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap till the first",
                    content: "fn main(foo: F, bar: B, spam: S, zap: Z) {}".trim(),
                    file_extension: "rs",
                    prepare_events: keys!("q s p a m enter"),
                    events: keys!("d t y"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["spam: S"]),
                        CurrentComponentContent("fn main(spam: S, foo: F, bar: B, zap: Z) {}"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap till the last",
                    content: "fn main(foo: F, bar: B, spam: S, zap: Z) {}".trim(),
                    file_extension: "rs",
                    prepare_events: keys!("q b a r enter"),
                    events: keys!("d t p"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["bar: B"]),
                        CurrentComponentContent("fn main(foo: F, spam: S, zap: Z, bar: B) {}"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap distant expressions using jump",
                    content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
                    file_extension: "js",
                    prepare_events: keys!("q x enter"),
                    events: keys!("d t m ' d"),
                    expectations: Box::new([CurrentComponentContent(
                        "if(condition) { 'hello world' } else { x(bar(baz)) }",
                    )]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap body of if-else",
                    content: r#"
impl<C> Iterator for PostorderTraverse<C>
    if c.goto_next_sibling() {
        // If we successfully go to a sibling of this node, we want to go back down
        // the tree on the next iteration
        self.retracing = false;
    } else {
        // If we weren't already retracing, we are now; travel upwards until we can
        // go to the next sibling or reach the root again
        self.retracing = true;
        if !c.goto_parent() {
            // We've reached the root again, and our iteration is done
            self.cursor = None;
        }
    }

    Some(node)
}
"#
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!(
                        "q { enter d t m { k"
                    ),
                    expectations: Box::new([]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap extended syntax node selections",
                    content: "[{\"x\": 123}, true, {\"y\": {}}, false]".trim(),
                    file_extension: "json",
                    prepare_events: keys!("w o"),
                    events: keys!("d g l t l l j"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["{\"x\": 123}, true"]),
                        CurrentComponentContent("[{\"y\": {}}, {\"x\": 123}, true, false]"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap extended line selections",
                    content: "foo\nbar\nspam\nbaz".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a g l t l l j"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["foo\nbar"]),
                        CurrentComponentContent("spam\nfoo\nbar\nbaz"),
                    ]),
                    terminal_height: Some(10),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swap extended word selections",
                    content: "foo bar spam baz".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w g l t l l j"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["foo bar"]),
                        CurrentComponentContent("spam foo bar baz"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
           ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "open",
            recipes: [
                Recipe {
                    description: "Open: syntax node selection mode (parameter)",
                    content: "def foo(bar: Bar, spam: Spam): pass",
                    file_extension: "py",
                    prepare_events: keys!("q s p a m enter"),
                    events: keys!("d , x esc / , y"),
                    expectations: Box::new([CurrentComponentContent("def foo(bar: Bar, spam: Spam, y, x): pass")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Open: syntax node selection mode (statements)",
                    content: "
function foo() {
  let x = hello();
  let y = hey()
     .bar();
}
".trim(),
                    file_extension: "js",
                    prepare_events: keys!("q l e t space y enter"),
                    events: keys!("d , l e t space z"),
                    expectations: Box::new([CurrentComponentContent("function foo() {
  let x = hello();
  let y = hey()
     .bar();
  let z
}")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Open: Line selection mode",
                    content: "
fn foo() {
    bar();
}".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a , x esc / , y"),
                    expectations: Box::new([CurrentComponentContent("fn foo() {
    y
    x
    bar();
}")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Open: Word selection mode",
                    content: "foo bar spam".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w , h i"),
                    expectations: Box::new([CurrentComponentContent("foo hi bar spam")]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                }
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "subword",
            recipes: [
                Recipe {
                    description: "Subword (skip symbols)",
                    content: "
HTTPNetwork 88 kebab-case 
snake_case 99 PascalCase
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w l l l l l k j j j j j i"),
                    expectations: Box::new([CurrentSelectedTexts(&["HTTP"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Subword (next/previous)",
                    content: "snake_case kebab-case"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w o o o u u u"),
                    expectations: Box::new([CurrentSelectedTexts(&["snake"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Subword: First/Last movement",
                    content: "hello HTTPNetworkRequestMiddleware world"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w l p y"),
                    expectations: Box::new([CurrentSelectedTexts(&["HTTP"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "word",
            recipes: [
                Recipe {
                    description: "Word: Left/Right skip symbols & spaces",
                    content: "
camelCase , kebab-case : snake_case 
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l l j j"),
                    expectations: Box::new([CurrentSelectedTexts(&["camelCase"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Word: Prev/Next no skip symbols & spaces",
                    content: "camelCase ,   kebab-case\nsnake_case".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s o o o u u"),
                    expectations: Box::new([CurrentSelectedTexts(&[","])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "char",
            recipes: [
                Recipe {
                    description: "Char: up/down/left/right movement",
                    content: "
camel 
snake
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("W l l k j j i"),
                    expectations: Box::new([CurrentSelectedTexts(&["c"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Char: first/last movement (first/last char of current subword)",
                    content: "campHelloDun".trim(),
                    file_extension: "md",
                    prepare_events: keys!("q h e l l o enter"),
                    events: keys!("W p y"),
                    expectations: Box::new([CurrentSelectedTexts(&["H"])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "raise",
            recipes: [
                Recipe {
                    description: "Raise: Conditionals",
                    content: "count > 0 ? x + 2 : y / z".trim(),
                    file_extension: "js",
                    prepare_events: keys!("q x enter"),
                    events: keys!("d T"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["x + 2"]),
                        CurrentComponentContent("x + 2"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Raise: XML/JSX",
                    content: "
<GParent>
    <Parent>
        <Child x={y}/>
        <Brother/>
    </Parent>
</GParent>"
                        .trim(),
                    file_extension: "js",
                    prepare_events: keys!("q < c h i l d enter"),
                    events: keys!("d T"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["<Child x={y}/>"]),
                        CurrentComponentContent("<GParent>\n    <Child x={y}/>\n</GParent>"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Raise: lambdas",
                    content: "
app.post('/admin', () => { 
    return router.route(foo, bar) 
})"
                    .trim(),
                    file_extension: "js",
                    prepare_events: keys!("q r o u t e r enter"),
                    events: keys!("d T T"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["router.route(foo, bar)"]),
                        CurrentComponentContent("app.post('/admin', () => router.route(foo, bar))"),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Raise: JSON",
                    content: r#"{"hello": {"world": [123], "foo": null}}"#.trim(),
                    file_extension: "js",
                    prepare_events: keys!("q 1 2 3 enter"),
                    events: keys!("d T T"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["123"]),
                        CurrentComponentContent(r#"{"hello": 123}"#),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "split-selections",
            recipes: [
                Recipe {
                    description: "Split selections by search",
                    content: "
fooz bar fooy
bar foox foow
foov foou bar
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a g l r q f o o enter v v"),
                    expectations: Box::new([
                        CurrentComponentContent(
                            "z bar y
bar x w
foov foou bar",
                        ),
                        CurrentSelectedTexts(&["z", "y", "x", "w"]),
                    ]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Split selections by last search",
                    content: "
fn main(foo: str) {
    bar(foo);

    foo = x + foo;
    return foo;
}"
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!("s l l e a d r n r"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["foo", "foo", "foo", "foo", "foo",]),
                    ]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Split selections by marks",
                    content: "foo bar spam"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s z l l z a r n z"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["foo", "spam"]),
                    ]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Split selections by line",
                    content: "
fn foo() {
   bar();
   spam();
   baz();
}"
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!("a l g h l r a"),
                    expectations: Box::new([CurrentSelectedTexts(&["bar();", "spam();", "baz();"])]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Nested spliting",
                    content: "
foo-da bar spam
bar foo-baz foo-yo
foo ha"
                        .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!("a g l r q f o o enter s r w r ; - enter"),
                    expectations: Box::new([CurrentSelectedTexts(&[
                        "foo", "da", "foo", "baz", "foo", "yo",
                    ])]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "add-cursor-with-movement",
            recipes: [
                Recipe {
                    description: "Add cursor to the next selections",
                    content: "foo bar spam baz"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w r l l"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["foo", "bar", "spam"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Add cursor to the previous selections",
                    content: "foo bar spam baz"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("a / s r j j"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["bar", "spam", "baz"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Add cursor to any places (with Jump)",
                    content: "alpha beta gamma omega"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w r m g"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["alpha", "gamma"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Add cursor till the last selection",
                    content: "alpha beta gamma omega zeta"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("s l r p"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["beta", "gamma", "omega", "zeta"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Add cursor till the first selection",
                    content: "alpha beta gamma omega zeta"
                    .trim(),
                    file_extension: "md",
                    prepare_events: keys!("q z enter"),
                    events: keys!("s p j r y"),
                    expectations: Box::new([
                        CurrentSelectedTexts(&["alpha","beta", "gamma", "omega"]),
                    ]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "literal-search",
            recipes: [Recipe {
                description: "Example",
                content: "foo bar (xo) baz (XO)".trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("q ( x o ) enter l j"),
                expectations: Box::new([CurrentSelectedTexts(&["(xo)"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "match-whole-word",
            recipes: [Recipe {
                description: "Example",
                content: "fobar fo spamfo fo".trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("q w space f o enter l j"),
                expectations: Box::new([CurrentSelectedTexts(&["fo"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "case-sensitive",
            recipes: [Recipe {
                description: "Example",
                content: "fo Fo fo Fo".trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("q c space F o enter l j"),
                expectations: Box::new([CurrentSelectedTexts(&["Fo"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "regex",
            recipes: [Recipe {
                description: "Example",
                content: "a (foo ba) bar".trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("q r space backslash ( . * backslash ) enter"),
                expectations: Box::new([CurrentSelectedTexts(&["(foo ba)"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "naming-convention-agnostic",
            recipes: [Recipe {
                description: "Example",
                content: "foBa x fo_ba x fo ba x fo-ba".trim(),
                file_extension: "js",
                prepare_events: &[],
                events: keys!("q n / f o space b a enter r r"),
                expectations: Box::new([CurrentSelectedTexts(&["foBa", "fo_ba", "fo ba", "fo-ba"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "ast-grep",
            recipes: [Recipe {
                description: "Example",
                content: "f(1+1); f(x); f('f()')".trim(),
                file_extension: "js",
                prepare_events: &[],
                events: keys!("q a space f ( $ X ) enter"),
                expectations: Box::new([CurrentSelectedTexts(&["f(1+1)"])]),
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "search-current-selection",
            recipes: [
                Recipe {
                    description: "Example 1",
                    content: "fo ba fo ba".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w e l"),
                    expectations: Box::new([CurrentSelectedTexts(&["fo"])]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Example 2: works for multiple lines too",
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
                    events: keys!("d e l"),
                    expectations: Box::new([CurrentSelectedTexts(&["foo\n  .bar()"])]),
                    terminal_height: Some(14),
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "replace-with-pattern",
            recipes: [
                Recipe {
                    description: "Naming convention-agnostic search and replace",
                    content: r#"
pub(crate) fn select(
    &mut self,
    selection_mode: SelectionMode,
    movement: Movement,
) -> anyhow::Result<Dispatches> {
    // There are a few selection modes where Current make sense.
    let direction = if self.selection_set.mode != selection_mode {
        Movement::Current
    } else {
        movement
    };

    if let Some(selection_set) = self.get_selection_set(&selection_mode, direction)? {
        Ok(self.update_selection_set(selection_set, true))
    } else {
        Ok(Default::default())
    }
}

fn jump_characters() -> Vec<char> {
    ('a'..='z').chain('A'..='Z').chain('0'..='9').collect_vec()
}

pub(crate) fn get_selection_mode_trait_object(
    &self,
    selection: &Selection,
) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
    self.selection_set.mode.to_selection_mode_trait_object(
        &self.buffer(),
        selection,
        &self.cursor_direction,
        &self.selection_set.filters,
    )
}
"#,
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!("q n / s e l e c t i o n space m o d e / f o o space b a r enter r r X r m"),
                    expectations: Box::new([]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Search & Replace Multi-cursor",
                    content: "fo x fo x fo".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("q l space f o space b a enter r l X"),
                    expectations: Box::new([CurrentComponentContent("ba x ba x fo")]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "replace-all",
            recipes: [
                Recipe {
                    description: "Example 1: Regex",
                    content: "1 x 2".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("q r space ( backslash d ) space ( $ 1 ) enter r r X"),
                    expectations: Box::new([CurrentComponentContent("(1) x (2)")]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Example 2: Naming convention-Agnostic",
                    content: "foBa x fo_ba x fo ba x fo-ba".trim(),
                    file_extension: "js",
                    prepare_events: &[],
                    events: keys!("q n / f o space b a / k a _ t o enter r r X"),
                    expectations: Box::new([CurrentComponentContent("kaTo x ka_to x ka to x ka-to")]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Example 3: AST Grep",
                    content: "f(1+1); f(x); f('f()')".trim(),
                    file_extension: "js",
                    prepare_events: &[],
                    events: keys!("q a space f ( $ X ) space ( $ X ) . z enter r r X"),
                    expectations: Box::new([CurrentComponentContent("(1+1).z; (x).z; ('f()').z")]),
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "filter-matching-selections",
            recipes: [
            Recipe {
                description: "Maintain matching selections",
                content: "
    enum Foo {
       Bar(baz),
       /// Spam is good
       Spam { what: String }
       /// Fifa means filifala
       Fifa
    }
    "
                .trim(),
                file_extension: "rs",
                prepare_events: keys!("q b enter"),
                events: keys!("d r r r h / / enter"),
                expectations: Box::new([
                    CurrentSelectedTexts(&[
                        "/// Spam is good\n",
                        "/// Fifa means filifala\n",
                    ]),
                    CurrentMode(Mode::Normal)
                ]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                    description: "Remove matching selections",
                    content: "
        enum Foo {
           Bar(baz),
           /// Spam is good
           Spam { what: String }
           /// Fifa means filifala
           Fifa
        }
        "
                    .trim(),
                    file_extension: "rs",
                    prepare_events: keys!("q b enter"),
                    events: keys!("d r r r ; / / enter"),
                    expectations: Box::new([CurrentSelectedTexts(&[
                        "Bar(baz)",
                        "Spam { what: String }",
                        "Fifa",
                    ])]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
            }].to_vec(),
        },
        RecipeGroup {
            filename: "add-cursor-to-all-matching-selections",
            recipes: [
            Recipe {
            description: "Example",
            content: "
foo bar spam
spam foo bar
bar spam foo
foo bar spam
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w e r r"),
            expectations: Box::new([
                CurrentSelectedTexts(&["foo", "foo", "foo", "foo"]),
                CurrentMode(Mode::Normal)
            ]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        }].to_vec(),
        },
        RecipeGroup {
            filename: "keep-primary-cursor-only",
            recipes: [
            Recipe {
            description: "Example",
            content: "
foo bar spam
spam foo bar
bar spam foo
foo bar spam
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w e r r r f"),
            expectations: Box::new([CurrentSelectedTexts(&["foo"]), CurrentMode(Mode::Normal)]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        }].to_vec(),
        },
        RecipeGroup {
            filename: "delete-cursor",
            recipes: [
            Recipe {
            description: "Example",
            content: "foo bar spam baz cam zeta om"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w r r : : : r v r v / r v r v"),
            expectations: Box::new([CurrentSelectedTexts(&["foo", "bar", "om"])]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        }].to_vec(),
        },
        RecipeGroup {
            filename: "mark-file",
            recipes: [
            Recipe {
            description: "Mark file and navigate marked files",
            content: ""
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("space ; q s r c enter enter q m a enter enter alt+z space ; q f o enter enter alt+z space ; q g i t enter enter alt+z alt+l alt+l alt+j alt+j alt+z"),
            expectations: Box::new([CurrentComponentTitle("\u{200b} # 🦀 foo.rs \u{200b} # 🦀 main.rs ".to_string())]),
            terminal_height: Some(10),
            similar_vim_combos: &[],
            only: false,
        }].to_vec(),
        },
        RecipeGroup {
            filename: "enter-normal-mode",
            recipes: [
            Recipe {
                description: "Enter Normal mode",
                content: "foo bar spam",
                file_extension: "md",
                prepare_events: &[],
                events: keys!("s l o ; k esc"),
                expectations: Box::new([CurrentSelectedTexts(&["k"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            }].to_vec(),
        },
        RecipeGroup {
            filename: "sticky-column",
            recipes: [
            Recipe {
                description: "Sticky Column",
                content: "foo spam\nbar\njav script",
                file_extension: "md",
                prepare_events: &[],
                events: keys!("s l k k i i"),
                expectations: Box::new([CurrentSelectedTexts(&["spam"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            }].to_vec(),
        },
        RecipeGroup {
            filename: "recipes",
            recipes: recipes(),
        },
    ]
    .to_vec()
}

fn showcase() -> RecipeGroup {
    RecipeGroup {
            filename: "showcase",
            recipes: [
                Recipe {
                    description: "Remove all println statements",
                    content: r#"
pub(crate) fn run(path: Option<CanonicalizedPath>) -> anyhow::Result<()> {
    println!("_args = {:?}", {
        let result = _args.collect::<Vec<_>>();
        result
    });

    let (sender, receiver) = std::sync::mpsc::channel();
    let syntax_highlighter_sender = syntax_highlight::start_thread(sender.clone());
    let mut app = App::from_channel(
        Arc::new(Mutex::new(Crossterm::default())),
        CanonicalizedPath::try_from(".")?;
        sender,
        receiver,
    )?;

    println!(
        "syntax_highlighter_sender = {:?}",
        syntax_highlighter_sender
    );

    app.set_syntax_highlight_request_sender(syntax_highlighter_sender);
    let sender = app.sender();

    let crossterm_join_handle = std::thread::spawn(move || loop {
        if crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| Ok(sender.send(AppMessage::Event(event.into()))?))
            .is_err()
        {
            println!("Something went wrong");
            break;
        }
    });

    app.run(path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    println!("Good bye!");
}
"#
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!("q p r i n t enter r r d v l"),
                    expectations: Box::new([CurrentComponentContent(r#"pub(crate) fn run(path: Option<CanonicalizedPath>) -> anyhow::Result<()> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let syntax_highlighter_sender = syntax_highlight::start_thread(sender.clone());
    let mut app = App::from_channel(
        Arc::new(Mutex::new(Crossterm::default())),
        CanonicalizedPath::try_from(".")?;
        sender,
        receiver,
    )?;

    app.set_syntax_highlight_request_sender(syntax_highlighter_sender);
    let sender = app.sender();

    let crossterm_join_handle = std::thread::spawn(move || loop {
        if crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| Ok(sender.send(AppMessage::Event(event.into()))?))
            .is_err()
        {
            break;
        }
    });

    app.run(path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;
}"#
                    )]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Sorting TODO list based on completion",
                    content: r#"
# Fake To-Do List

- [x] Buy groceries
- [x] Finish the report for work
- [ ] Call the plumber
- [x] Go to the gym
- [ ] Schedule a dentist appointment
- [x] Pay the bills
- [ ] Plan a weekend getaway
- [x] Read a new book
  - [x] Chapter 1
  - [x] Chapter 2
  - [x] Chapter 3
- [ ] Organize the closet
- [ ] Watch a movie
  - [ ] Action film
  - [ ] Comedy
  - [x] Documentary
- [x] Write documentation
"#
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!(
                        "q r / ^ - space backslash [ space backslash ] enter r r d c v l a p b ; backspace"
                    ),
                    expectations: Box::new([CurrentComponentContent(r#"# Fake To-Do List

- [x] Buy groceries
- [x] Finish the report for work
- [x] Go to the gym
- [x] Pay the bills
- [x] Read a new book
  - [x] Chapter 1
  - [x] Chapter 2
  - [x] Chapter 3
- [x] Write documentation
- [ ] Call the plumber
- [ ] Schedule a dentist appointment
- [ ] Plan a weekend getaway
- [ ] Organize the closet
- [ ] Watch a movie
  - [ ] Action film
  - [ ] Comedy
  - [x] Documentary"#
                    )]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Wrap/unwrap the value of each key with Some in a struct",
                    content: r#"
pub(crate) fn from_text(language: Option<tree_sitter::Language>, text: &str) -> Self {
    Self {
        yx: SelectionSet {
            primary: Selection::default(),
            secondary: vec![],
            mode: SelectionMode::Custom,
            filters: Filters::default(),
        },
        jumps: None,
        mode: Mode::Normal,
        cursor_direction: Direction::Start,
        scroll_offset: 0,
        rectangle: Rectangle::default(),
        buffer: Rc::new(RefCell::new(Buffer::new(language, text))),
        title: None,
        id: ComponentId::new(),
        current_view_alignment: None,
        regex_highlight_rules: Vec::new(),
        selection_set_history: History::new(),
    }
}
"#
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!(
                        "q y x enter d r r k l g , j h S o m e esc d k l k T r f"
                    ),
                    expectations: Box::new([]),
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        }
}

fn syntax_node() -> RecipeGroup {
    RecipeGroup {
        filename: "syntax-node",
        recipes: [
            Recipe {
                description: "Select a syntax node (Rust)",
                content: "fn main() {}\nfn foo() {}".trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("d"),
                expectations: Box::new([CurrentSelectedTexts(&["fn main() {}"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Select a syntax node (Python)",
                content: "def main():\n\tpass".trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("d"),
                expectations: Box::new([CurrentSelectedTexts(&["def main():\n\tpass"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Select a syntax node (JSON)",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("w o"),
                events: keys!("d"),
                expectations: Box::new([CurrentSelectedTexts(&["{\"x\": 123}"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Navigate named sibling nodes via Left/Right movement",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("w o"),
                events: keys!("d l l l j j"),
                expectations: Box::new([CurrentSelectedTexts(&["{\"x\": 123}"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Navigate to first/last named sibling nodes via First/Last movement",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("w o"),
                events: keys!("d p y"),
                expectations: Box::new([CurrentSelectedTexts(&["{\"x\": 123}"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Navigate all sibling nodes via Previous/Next movement",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("w o"),
                events: keys!("d p u u u o o"),
                expectations: Box::new([CurrentSelectedTexts(&[","])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Select Parent",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("w l"),
                events: keys!("d i i i i"),
                expectations: Box::new([CurrentSelectedTexts(&[
                    "[{\"x\": 123}, true, {\"y\": {}}]",
                ])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Select First-Child",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("d"),
                events: keys!("d k k k k"),
                expectations: Box::new([CurrentSelectedTexts(&["x"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
        ]
        .to_vec(),
    }
}

fn multicursors() -> RecipeGroup {
    RecipeGroup {
        filename: "multi-cursor",
        recipes: [
            Recipe {
                description: "Non-movements keys exit multicursor mode",
                content: "hello world hello world".trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("s e r l f z"),
                expectations: Box::new([CurrentComponentContent("z world z world")]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Use space to exit multicursor mode",
                content: "foo bar spam baz".trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("s r l space l"),
                expectations: Box::new([CurrentSelectedTexts(&["bar", "spam"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
        ]
        .to_vec(),
    }
}

fn reveal_selections() -> RecipeGroup {
    RecipeGroup {
        filename: "reveal-selections",
        recipes: [
            Recipe {
                description: "Reveal Searches",
                content: "
head
1foo
1bar
1spam

2foo
2bar
2spam

3foo
3bar
3spam"
                    .trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("q f o o enter space u l l j j space u"),
                expectations: Box::new([CurrentSelectedTexts(&["foo"])]),
                terminal_height: Some(9),
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Reveal Sibling Nodes",
                content: "
fn foo() {
    // fooing
    // still fooing
    // more foo
}

fn bar() {
    // some bar
}

fn spam() {
    // spam yes
}
                
fn baz() {}
                "
                .trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("d space u l l j j space u"),
                expectations: Box::new([CurrentSelectedTexts(&[
                    "fn foo() {\n    // fooing\n    // still fooing\n    // more foo\n}",
                ])]),
                terminal_height: Some(9),
                similar_vim_combos: &[],
                only: false,
            },
        ]
        .to_vec(),
    }
}

fn reveal_cursors() -> RecipeGroup {
    RecipeGroup {
        filename: "reveal-cursors",
        recipes: [Recipe {
            description: "Reveal Cursors",
            content: "
# Section 1
1foo
1bar
1spam

# Section 2
2foo
2bar
2spam

# Section 3
3foo
3bar
3spam"
                .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("q f o o enter r r ; x esc s"),
            expectations: Box::new([CurrentSelectedTexts(&["1foox", "2foox", "3foox"])]),
            terminal_height: Some(9),
            similar_vim_combos: &[],
            only: false,
        }]
        .to_vec(),
    }
}

fn swap_cursors() -> RecipeGroup {
    RecipeGroup {
        filename: "swap-cursors",
        recipes: [
            Recipe {
                description: "Swap Cursors to view out-of-bound selection end",
                content: "
fn main() {
   foo();
   bar();
   spam();
   baz();
   bomb();
} // Last line of main()
"
                .trim(),
                file_extension: "rs",
                prepare_events: &[],
                events: keys!("d / /"),
                expectations: Box::new([EditorCursorPosition(Position { line: 0, column: 0 })]),
                terminal_height: Some(5),
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Swap Cursors to select last word of current line",
                content: "foo bar spam baz()".trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("a / s"),
                expectations: Box::new([CurrentSelectedTexts(&[")"])]),
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
        ]
        .to_vec(),
    }
}

fn swap_current_selection_using_a_different_selection_mode() -> RecipeGroup {
    RecipeGroup {
        filename: "swap-current-selection-using-a-different-selection-mode",
        recipes: [Recipe {
            description: "Example 1",
            content: "
betty bought some butter, so betty bought some better butter
**but the butter was bitter**
"
            .trim(),
            file_extension: "rs",
            prepare_events: &[],
            events: keys!("a l g s t j j j j j j"),
            expectations: Box::new([CurrentComponentContent("betty bought some butter, **but the butter was bitter** so betty bought some better\nbutter")]),
            terminal_height: Some(10),
            similar_vim_combos: &[],
            only: false,
        }]
        .to_vec(),
    }
}

fn reveal_marks() -> RecipeGroup {
    RecipeGroup {
        filename: "reveal-marks",
        recipes: [Recipe {
            description: "Reveal Marks",
            content: "
# Section 1
1foo
1bar
1spam

# Section 2
2foo
2bar
2spam

# Section 3
3foo
3bar
3spam"
                .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("q f o o enter l b space o l j j"),
            expectations: Box::new([CurrentSelectedTexts(&["foo"])]),
            terminal_height: Some(9),
            similar_vim_combos: &[],
            only: false,
        }]
        .to_vec(),
    }
}

fn recipes() -> Vec<Recipe> {
    [
        Recipe {
            description: "Duplicate current line",
            content: "
To be, or not to be?
That, is the question.
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("a c b"),
            expectations: Box::new([CurrentComponentContent(
                "To be, or not to be?
To be, or not to be?
That, is the question.",
            )]),
            terminal_height: None,
            similar_vim_combos: &["y y p"],
            only: false,
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
            events: keys!("a g g"),
            expectations: Box::new([CurrentSelectedTexts(&[
                "To be, or not to be, that is the question:
Whether 'tis nobler in the mind to suffer
The slings and arrows of outrageous fortune,
Or to take arms against a sea of troubles
And by opposing end them. To die—to sleep,",
            ])]),
            terminal_height: None,
            similar_vim_combos: &["g g V G"],
            only: false,
        },
        Recipe {
            description: "Word movement",
            content: "hello-world camelCase snake_case",
            file_extension: "md",
            prepare_events: &[],
            events: keys!("s l l"),
            expectations: Box::new([CurrentSelectedTexts(&["snake_case"])]),
            terminal_height: None,
            similar_vim_combos: &["W", "E", "B"],
            only: false,
        },
        Recipe {
            description: "Subword movement",
            content: "
camelCase
hello_world
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w k l i j"),
            expectations: Box::new([CurrentSelectedTexts(&["camel"])]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Undo & Redo",
            content: "camelCase".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w v l . >"),
            expectations: Box::new([CurrentComponentContent("Case")]),
            terminal_height: None,
            similar_vim_combos: &["u", "ctrl+r"],
            only: false,
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
            events: keys!("w e r l l esc ; x"),
            expectations: Box::new([CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foo bar spam",
            )]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Move the first two elements to the last",
            content: "[{\"a\": b}, \"c\", [], {}]".trim(),
            file_extension: "json",
            prepare_events: keys!("w o"),
            events: keys!("d g l c v l p b"),
            expectations: Box::new([CurrentComponentContent("[[], {}, {\"a\": b}, \"c\"]")]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Change the first two subword",
            content: "This is am Ki".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("w v l f I esc"),
            expectations: Box::new([CurrentComponentContent("I am Ki")]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Raise / Replace parent node with current node",
            content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
            file_extension: "js",
            prepare_events: keys!("q x enter"),
            events: keys!("d T"),
            expectations: Box::new([CurrentComponentContent("x(bar(baz))")]),
            terminal_height: Some(7),
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Remove all sibling nodes except the current node",
            content: "[foo(), {xar: 'spam'}, baz + baz]".trim(),
            file_extension: "js",
            prepare_events: keys!("q { enter"),
            events: keys!("d c y g p x"),
            expectations: Box::new([CurrentComponentContent("[{xar: 'spam'}]")]),
            terminal_height: Some(7),
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Save",
            content: "hello world".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("enter"),
            expectations: Box::new([]),
            terminal_height: None,
            similar_vim_combos: &[": w enter"],
            only: false,
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
            prepare_events: keys!("q i f enter a"),
            events: keys!("alt+; alt+; alt+;"),
            expectations: Box::new([CurrentSelectedTexts(&[
                "If nautical nonsense be something you wish?",
            ])]),
            terminal_height: Some(8),
            similar_vim_combos: &["z t", "z z", "z b"],
            only: false,
        },
        Recipe {
            description: "Invert nesting (JSX)",
            content: "<Parent><Child><Grandson/></Child></Parent>".trim(),
            file_extension: "js",
            prepare_events: keys!("d k l k l"),
            events: keys!("c i C i C k l x"),
            expectations: Box::new([CurrentComponentContent(
                "<Child><Parent><Grandson/></Parent></Child>",
            )]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Invert nesting (Function Call)",
            content: "foo(bar(yo, spam(baz), baz), bomb)".trim(),
            file_extension: "js",
            prepare_events: keys!("q s enter d"),
            events: keys!("c i i C i i C k l k l x"),
            expectations: Box::new([CurrentComponentContent(
                "bar(yo, foo(spam(baz), bomb), baz)",
            )]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Collapse selection (End)",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("a $"),
            expectations: Box::new([CurrentSelectedTexts(&["m"])]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Select from current selection until end of line",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: keys!("s l"),
            events: keys!("g a $"),
            expectations: Box::new([CurrentSelectedTexts(&["bar spam"])]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Select last subword of current line",
            content: "Hello world?\nBye".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("a / w"),
            expectations: Box::new([CurrentSelectedTexts(&["?"])]),
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
    ]
    .to_vec()
}
