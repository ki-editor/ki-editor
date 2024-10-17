use my_proc_macros::keys;

use crate::{
    generate_recipes::{Recipe, RecipeGroup},
    test_app::*,
};

pub(crate) fn recipe_groups() -> Vec<RecipeGroup> {
    [
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
                    events: keys!("/ p r i n t enter space a s d"),
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
                        "' x / ^ - space backslash [ space backslash ] enter space a s y d e . p a backspace"
                    ),
                    expectations: &[],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Swapping body of if-else",
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
                        "/ y x enter s space a b n v s ( i S o m e esc s b n n T space o"
                    ),
                    expectations: &[],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
                Recipe {
                    description: "Case-agnostic search and replace",
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
"#
                    .trim(),
                    file_extension: "rs",
                    prepare_events: &[],
                    events: keys!(
                        "' c / s e l e c t i o n space m o d e enter ' r f o o space b a r enter ctrl+c space a ctrl+r space o"
                    ),
                    expectations: &[],
                    terminal_height: None,
                    similar_vim_combos: &[],
                    only: false,
                },
            ]
            .to_vec(),
        },
        RecipeGroup {
            filename: "recipes",
            recipes: recipes(),
        },
    ]
    .to_vec()
}

fn recipes() -> Vec<Recipe> {
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
            description: "Move to sibling node",
            content: "[{\"x\": 123}, true, {\"y\": {}}]".trim(),
            file_extension: "json",
            prepare_events: keys!("^ l"),
            events: keys!("s n n N N"),
            expectations: &[CurrentSelectedTexts(&["{\"x\": 123}"])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Swap sibling node",
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
            description: "Swap sibling node",
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
            expectations: &[CurrentSelectedTexts(&["snake_"])],
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
            expectations: &[CurrentSelectedTexts(&["Case"])],
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
            description: "Default Search (literal, no escaping needed)",
            content: "foo bar (x) baz (x)".trim(),
            file_extension: "md",
            prepare_events: &[],
            events: keys!("/ ( x ) enter l h"),
            expectations: &[CurrentSelectedTexts(&["(x)"])],
            terminal_height: Some(7),
            similar_vim_combos: &[],
            only: false,
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
            only: false,
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
            only: false,
        },
        Recipe {
            description: "Search current selection",
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
            events: keys!("s * l"),
            expectations: &[CurrentSelectedTexts(&["foo\n  .bar()"])],
            terminal_height: Some(14),
            similar_vim_combos: &[],
            only: false,
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
            only: false,
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
            only: false,
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
        Recipe {
            description: "Search & Replace All (regex)",
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
            description: "Search & Replace All (case agnostic)",
            content: "foBa x fo_ba x fo ba x fo-ba".trim(),
            file_extension: "js",
            prepare_events: &[],
            events: keys!("/ f o space b a enter ' c ' r k a _ t o enter R"),
            expectations: &[CurrentComponentContent("kaTo x ka_to x ka to x ka-to")],
            terminal_height: Some(7),
            similar_vim_combos: &[],
            only: false,
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
            events: keys!("w * space a a x"),
            expectations: &[CurrentComponentContent(
                "foox bar spam
spam foox bar
bar spam foox
foox bar spam",
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
        Recipe {
            description: "Keep selections matching search",
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
            events: keys!("s space a K / / enter"),
            expectations: &[CurrentSelectedTexts(&[
                "/// Spam is good\n",
                "/// Fifa means filifala\n",
            ])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
        Recipe {
            description: "Remove selections matching search",
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
            events: keys!("s space a alt+k / / enter"),
            expectations: &[CurrentSelectedTexts(&[
                "Bar(baz)",
                "Spam { what: String }",
                "Fifa",
            ])],
            terminal_height: None,
            similar_vim_combos: &[],
            only: false,
        },
    ]
    .to_vec()
}
