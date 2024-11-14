use my_proc_macros::keys;

use crate::{
    components::editor::Mode,
    generate_recipes::{Recipe, RecipeGroup},
    test_app::*,
};

pub(crate) fn recipe_groups() -> Vec<RecipeGroup> {
    [
        showcase(),
        syntax_node(),
        RecipeGroup {
            filename: "expand",
            recipes: [Recipe {
                description: "Expand to nearest brackets/quotes",
                content: "hello '{World Foo} bar'".trim(),
                file_extension: "md",
                prepare_events: keys!("w n"),
                events: keys!("t t t t"),
                expectations: &[CurrentSelectedTexts(&["'{World Foo} bar'"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "join",
            recipes: [Recipe {
                description: "Example",
                content: "
This is 
a multiple line
string.
"
                .trim(),
                file_extension: "md",
                prepare_events: &[],
                events: keys!("e v j j J"),
                expectations: &[CurrentSelectedTexts(&["This is a multiple line string."])],
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
                prepare_events: keys!("/ s p a m enter"),
                events: keys!("K K"),
                expectations: &[CurrentSelectedTexts(&["spam"]), CurrentComponentContent("def foo():
    bar = 1;
    
    spam = 2;")],
                terminal_height: Some(7),
                similar_vim_combos: &[],
                only: false,
            }]
            .to_vec(),
        },
        RecipeGroup {
            filename: "exchange",
            recipes: [
                Recipe {
                    description: "Exchange sibling node",
                    content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                    file_extension: "json",
                    prepare_events: keys!("^ l"),
                    events: keys!("s x n n"),
                    expectations: &[
                        CurrentSelectedTexts(&["{\"x\": 123}"]),
                        CurrentComponentContent("[true, {\"y\": {}}, {\"x\": 123}]"),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Exchange sibling node",
                    content: "<x><y>foo</y><div/></x>".trim(),
                    file_extension: "xml",
                    prepare_events: keys!("^ l l l"),
                    events: keys!("s b x n"),
                    expectations: &[
                        CurrentSelectedTexts(&["<y>foo</y>"]),
                        CurrentComponentContent("<x><div/><y>foo</y></x>"),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Exchange till the first",
                    content: "fn main(foo: F, bar: B, spam: S, zap: Z) {}".trim(),
                    file_extension: "rs",
                    prepare_events: keys!("/ s p a m enter"),
                    events: keys!("s x ,"),
                    expectations: &[
                        CurrentSelectedTexts(&["spam: S"]),
                        CurrentComponentContent("fn main(spam: S, foo: F, bar: B, zap: Z) {}"),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Exchange till the last",
                    content: "fn main(foo: F, bar: B, spam: S, zap: Z) {}".trim(),
                    file_extension: "rs",
                    prepare_events: keys!("/ b a r enter"),
                    events: keys!("s x ."),
                    expectations: &[
                        CurrentSelectedTexts(&["bar: B"]),
                        CurrentComponentContent("fn main(foo: F, spam: S, zap: Z, bar: B) {}"),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Exchange distant expressions using jump",
                    content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
                    file_extension: "js",
                    prepare_events: keys!("/ x enter"),
                    events: keys!("s x f ' a"),
                    expectations: &[CurrentComponentContent(
                        "if(condition) { 'hello world' } else { x(bar(baz)) }",
                    )],
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Exchange body of if-else",
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
                        "/ { enter s x f { b"
                    ),
                    expectations: &[],
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
                    prepare_events: keys!("/ s p a m enter"),
                    events: keys!("s o x esc O y"),
                    expectations: &[CurrentComponentContent("def foo(bar: Bar, spam: Spam, y, x): pass")],
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
                    prepare_events: keys!("/ l e t space y enter"),
                    events: keys!("s o l e t space z"),
                    expectations: &[CurrentComponentContent("function foo() {
  let x = hello();
  let y = hey()
     .bar();
  let z
}")],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Open: non-syntax node selection mode",
                    content: "
fn foo() {
    bar();
}".trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w o x esc O y"),
                    expectations: &[CurrentComponentContent("fn foo() {
    y
    x
    bar();
}")],
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
                    description: "Subword: up/down/left/right movement",
                    content: "
HTTPNetwork 88 kebab-case 
snake_case 99 PascalCase
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("W l l l l l j h h h h h k"),
                    expectations: &[CurrentSelectedTexts(&["HTTP"])],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Subword: next/previous movement (skip symbols)",
                    content: "
camelCase , kebab-case snake_case
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("W n n n n n N N N N N"),
                    expectations: &[CurrentSelectedTexts(&["camel"])],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Subword: first/last movement",
                    content: "hello HTTPNetworkRequestMiddleware world"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("W l . ,"),
                    expectations: &[CurrentSelectedTexts(&["HTTP"])],
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
                    description: "Word: up/down/left/right movement",
                    content: "
camelCase ,  kebab-case 
snake_case + PascalCase
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w l l j h h k"),
                    expectations: &[CurrentSelectedTexts(&["camelCase"])],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Word: next/previous movement (skip symbols)",
                    content: "
camelCase , kebab-case 
snake_case + PascalCase
"
                    .trim(),
                    file_extension: "md",
                    prepare_events: &[],
                    events: keys!("w n n n N N N"),
                    expectations: &[CurrentSelectedTexts(&["camelCase"])],
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
                    prepare_events: keys!("/ x enter"),
                    events: keys!("s T"),
                    expectations: &[
                        CurrentSelectedTexts(&["x + 2"]),
                        CurrentComponentContent("x + 2"),
                    ],
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
                    prepare_events: keys!("/ < c h i l d enter"),
                    events: keys!("s T"),
                    expectations: &[
                        CurrentSelectedTexts(&["<Child x={y}/>"]),
                        CurrentComponentContent("<GParent>\n    <Child x={y}/>\n</GParent>"),
                    ],
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
                    prepare_events: keys!("/ r o u t e r enter"),
                    events: keys!("s T T"),
                    expectations: &[
                        CurrentSelectedTexts(&["router.route(foo, bar)"]),
                        CurrentComponentContent("app.post('/admin', () => router.route(foo, bar))"),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Raise: JSON",
                    content: r#"{"hello": {"world": [123], "foo": null}}"#.trim(),
                    file_extension: "js",
                    prepare_events: keys!("/ 1 2 3 enter"),
                    events: keys!("s T T"),
                    expectations: &[
                        CurrentSelectedTexts(&["123"]),
                        CurrentComponentContent(r#"{"hello": 123}"#),
                    ],
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
                    events: keys!("e v j q / f o o enter d"),
                    expectations: &[
                        CurrentComponentContent(
                            "z bar y
bar x w
foov foou bar",
                        ),
                        CurrentSelectedTexts(&["z", "y", "x", "w"]),
                    ],
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
                    events: keys!("e j t q e"),
                    expectations: &[CurrentSelectedTexts(&["bar();", "spam();", "baz();"])],
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
                    events: keys!("e v j q / f o o enter w q W q r - enter"),
                    expectations: &[CurrentSelectedTexts(&[
                        "foo", "da", "foo", "baz", "foo", "yo",
                    ])],
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
                    events: keys!("w q l l"),
                    expectations: &[
                        CurrentSelectedTexts(&["foo", "bar", "spam"]),
                    ],
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
                    events: keys!("w . q h h"),
                    expectations: &[
                        CurrentSelectedTexts(&["bar", "spam", "baz"]),
                    ],
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
                    events: keys!("w q f g"),
                    expectations: &[
                        CurrentSelectedTexts(&["alpha", "gamma"]),
                    ],
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
                    events: keys!("w l q ."),
                    expectations: &[
                        CurrentSelectedTexts(&["beta", "gamma", "omega", "zeta"]),
                    ],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Add cursor till the first selection",
                    content: "alpha beta gamma omega zeta"
                    .trim(),
                    file_extension: "md",
                    prepare_events: keys!("/ z enter"),
                    events: keys!("w h q ,"),
                    expectations: &[
                        CurrentSelectedTexts(&["alpha","beta", "gamma", "omega"]),
                    ],
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
                events: keys!("/ ( x o ) enter l h"),
                expectations: &[CurrentSelectedTexts(&["(xo)"])],
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
                events: keys!("/ f o enter ' w l h"),
                expectations: &[CurrentSelectedTexts(&["fo"])],
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
                events: keys!("/ F o enter ' c l h"),
                expectations: &[CurrentSelectedTexts(&["Fo"])],
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
                events: keys!("/ backslash ( . * backslash ) enter ' x"),
                expectations: &[CurrentSelectedTexts(&["(foo ba)"])],
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
                events: keys!("/ f o space b a enter ' n l"),
                expectations: &[CurrentSelectedTexts(&["fo-ba"])],
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
                events: keys!("/ f ( $ X ) enter ' a"),
                expectations: &[CurrentSelectedTexts(&["f(1+1)"])],
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
                    events: keys!("w * l"),
                    expectations: &[CurrentSelectedTexts(&["fo"])],
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
                    events: keys!("s * l"),
                    expectations: &[CurrentSelectedTexts(&["foo\n  .bar()"])],
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
                    events: keys!("' n / s e l e c t i o n space m o d e enter ' r f o o space b a r enter ctrl+c q q ctrl+r q o"),
                    expectations: &[],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
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
                    events: keys!("/ ( backslash d ) enter ' x ' r ( $ 1 ) enter R"),
                    expectations: &[CurrentComponentContent("(1) x (2)")],
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Example 2: Naming convention-Agnostic",
                    content: "foBa x fo_ba x fo ba x fo-ba".trim(),
                    file_extension: "js",
                    prepare_events: &[],
                    events: keys!("/ f o space b a enter ' n ' r k a _ t o enter R"),
                    expectations: &[CurrentComponentContent("kaTo x ka_to x ka to x ka-to")],
                    terminal_height: Some(7),
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Example 3: AST Grep",
                    content: "f(1+1); f(x); f('f()')".trim(),
                    file_extension: "js",
                    prepare_events: &[],
                    events: keys!("/ f ( $ X ) enter ' a ' r ( $ X ) . z enter R"),
                    expectations: &[CurrentComponentContent("(1+1).z; (x).z; ('f()').z")],
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
                prepare_events: keys!("/ b enter"),
                events: keys!("s q q q m / / enter"),
                expectations: &[
                    CurrentSelectedTexts(&[
                        "/// Spam is good\n",
                        "/// Fifa means filifala\n",
                    ]),
                    CurrentMode(Mode::Normal)
                ],
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
                    prepare_events: keys!("/ b enter"),
                    events: keys!("s q q q r / / enter"),
                    expectations: &[CurrentSelectedTexts(&[
                        "Bar(baz)",
                        "Spam { what: String }",
                        "Fifa",
                    ])],
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
            events: keys!("w * q q"),
            expectations: &[
                CurrentSelectedTexts(&["foo", "foo", "foo", "foo"]),
                CurrentMode(Mode::Normal)
            ],
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
            events: keys!("w * q q q o"),
            expectations: &[CurrentSelectedTexts(&["foo"]), CurrentMode(Mode::Normal)],
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
            events: keys!("w q q ) ) ) q d d D D"),
            expectations: &[CurrentSelectedTexts(&["foo", "bar", "om"])],
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
                    events: keys!("/ p r i n t enter q q s d"),
                    expectations: &[],
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
                        "' x / ^ - space backslash [ space backslash ] enter q q s y d e . p a backspace"
                    ),
                    expectations: &[],
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
                        "/ y x enter s q q b n v s ( i S o m e esc s b n n T q o"
                    ),
                    expectations: &[],
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
                events: keys!("s"),
                expectations: &[CurrentSelectedTexts(&["fn main() {}"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
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
                only: false,
            },
            Recipe {
                description: "Select a syntax node (JSON)",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("^ l"),
                events: keys!("s"),
                expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Navigate sibling nodes via Next/Previous/First/Last movement",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("^ l"),
                events: keys!("s n n N N . ,"),
                expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
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
                only: false,
            },
            Recipe {
                description: "Expand selection / Select Parent",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("^ l l"),
                events: keys!("S t t t t"),
                expectations: &[CurrentSelectedTexts(&["[{\"x\": 123}, true, {\"y\": {}}]"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description: "Shrink selection / Select First-Child",
                content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
                file_extension: "json",
                prepare_events: keys!("s"),
                events: keys!("s b b b b"),
                expectations: &[CurrentSelectedTexts(&["x"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
            Recipe {
                description:
                    "Select the nearest non-overlapping largest node to the right or below",
                content: "fn main(a: A, b: B) {}".trim(),
                file_extension: "rs",
                prepare_events: keys!("/ a : space A enter"),
                events: keys!("s l l l"),
                expectations: &[CurrentSelectedTexts(&[")"])],
                terminal_height: None,
                similar_vim_combos: &[],
                only: false,
            },
        ]
        .to_vec(),
    }
}

fn recipes() -> Vec<Recipe> {
    [
        Recipe {
            description: "Jump to a word",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: &[],
            events: keys!("w f t"),
            expectations: &[CurrentSelectedTexts(&["true"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
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
            only: false,
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
            events: keys!("e . ,"),
            expectations: &[CurrentSelectedTexts(&[
                "To be, or not to be, that is the question:",
            ])],
            terminal_height: None,
            similar_vim_combos: &["g g", "G"],
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
            only: false,
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
            only: false,
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
            only: false,
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
            only: false,
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
            only: false,
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
            events: keys!("W j l k h"),
            expectations: &[CurrentSelectedTexts(&["camel"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Delete subwords (forward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("W d d"),
            expectations: &[CurrentSelectedTexts(&["snake"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Delete subwords (backward)",
            content: "camelCase snake_case".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("W . D D"),
            expectations: &[CurrentSelectedTexts(&["snake"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Undo & Redo",
            content: "camelCase".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("W d u U"),
            expectations: &[CurrentComponentContent("Case")],
            terminal_height: None,
            similar_vim_combos: &["u", "ctrl+r"],
            only: false,
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
            only: false,
        },
        Recipe {
            description: "Delete Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("^ l"),
            events: keys!("v d ("),
            expectations: &[CurrentComponentContent("hello world")],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Change Surround",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("^ l"),
            events: keys!("v c ( {"),
            expectations: &[CurrentComponentContent("{hello world}")],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Select Inside Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("^ l"),
            events: keys!("v i ("),
            expectations: &[CurrentSelectedTexts(&["hello world"])],
            terminal_height: None,
            similar_vim_combos: &["v i ("],
            only: false,
        },
        Recipe {
            description: "Select Around Enclosures",
            content: "(hello world)".trim(),
            file_extension: "md",
            prepare_events: keys!("^ l"),
            events: keys!("v a ("),
            expectations: &[CurrentSelectedTexts(&["(hello world)"])],
            terminal_height: None,
            similar_vim_combos: &["v a ("],
            only: false,
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
            only: false,
        },
        Recipe {
            description: "Extend selection (Syntax Node)",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("^ l"),
            events: keys!("s v n n N"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}, true"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
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
            only: false,
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
            events: keys!("w * q l l esc a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foo bar spam",
            )],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Move the first two elements to the last",
            content: "[{\"a\": b}, \"c\", [], {}]".trim(),
            file_extension: "json",
            prepare_events: keys!("^ l"),
            events: keys!("s v n y d . p"),
            expectations: &[CurrentComponentContent("[[], {}, {\"a\": b}, \"c\"]")],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
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
            only: false,
        },
        Recipe {
            description: "Raise / Replace parent node with current node",
            content: "if(condition) { x(bar(baz)) } else { 'hello world' }".trim(),
            file_extension: "js",
            prepare_events: keys!("/ x enter"),
            events: keys!("s T r"),
            expectations: &[CurrentComponentContent("x(bar(baz))")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
            only: false,
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
            only: false,
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
            prepare_events: keys!("/ i f enter e"),
            events: keys!("ctrl+l ctrl+l ctrl+l"),
            expectations: &[CurrentSelectedTexts(&[
                "If nautical nonsense be something you wish?",
            ])],
            terminal_height: Some(8),
            similar_vim_combos: &["z t", "z z", "z b"],
            only: false,
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
            events: keys!("s y n R b n b n r"),
            expectations: &[CurrentComponentContent(
                "foo(bar, 1 + 1, spam)
foo(bar, 3 * 10, spam)",
            )],
            terminal_height: None,
            similar_vim_combos: &["p"],
            only: false,
        },
        Recipe {
            description: "Invert nesting (JSX)",
            content: "<Parent><Child><Grandson/></Child></Parent>".trim(),
            file_extension: "js",
            prepare_events: keys!("s b n b n"),
            events: keys!("y t R t R b n r"),
            expectations: &[CurrentComponentContent(
                "<Child><Parent><Grandson/></Parent></Child>",
            )],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Invert nesting (Function Call)",
            content: "foo(bar(yo, spam(baz), baz), bomb)".trim(),
            file_extension: "js",
            prepare_events: keys!("/ s enter s"),
            events: keys!("y t t R t t R b n b n r"),
            expectations: &[CurrentComponentContent(
                "bar(yo, foo(spam(baz), bomb), baz)",
            )],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Collapse selection (End)",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("e $"),
            expectations: &[CurrentSelectedTexts(&["m"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Collapse selection (Start)",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("e ^"),
            expectations: &[CurrentSelectedTexts(&["f"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Select from current selection until end of line",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: keys!("w l"),
            events: keys!("v e $"),
            expectations: &[CurrentSelectedTexts(&["bar spam"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Select from current selection until beginning of line",
            content: "foo bar spam".trim(),
            file_extension: "js",
            prepare_events: keys!("w l"),
            events: keys!("v e ^"),
            expectations: &[CurrentSelectedTexts(&["foo bar"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Paste with automatic gap insertion",
            content: "
foo bar
spam baz
"
            .trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e y p"),
            expectations: &[CurrentComponentContent("foo bar\nfoo bar\nspam baz")],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Paste without automatic gap insertion",
            content: "foo bar".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("e y $ p"),
            expectations: &[CurrentComponentContent("foo barfoo bar")],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
    ]
    .to_vec()
}
