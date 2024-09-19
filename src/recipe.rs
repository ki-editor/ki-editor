use itertools::Itertools;
use my_proc_macros::keys;

use crate::{components::editor::IfCurrentNotFound, test_app::*};

fn recipes() -> Vec<Recipe> {
    [
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
            description: "Delete a line",
            content: "
To be, or not to be?
That, is the question.
"
            .trim(),
            file_extension: "md",
            events: keys!("e d"),
            expectations: &[CurrentSelectedTexts(&["That, is the question."])],
        },
        Recipe {
            description: "Delete two lines consecutively",
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
    ]
    .to_vec()
}

#[test]
fn generate_recipes() -> anyhow::Result<()> {
    let recipes_output = recipes()
        .into_iter()
        .map(|recipe| -> anyhow::Result<RecipeOutput> {
            let path = format!("docu/assets/{}/", recipe.description);
            std::fs::create_dir_all(path.clone())?;
            let accum_events = create_nested_vectors(recipe.events);
            let accum_events_len = accum_events.len();
            let steps = accum_events
                .into_iter()
                .enumerate()
                .map(|(index, events)| -> anyhow::Result<StepOutput> {
                    let result = execute_recipe(format!("{}/{}", path, index), |s| {
                        [
                            App(OpenFile(s.main_rs())),
                            Editor(SetContent(recipe.content.to_string())),
                            Editor(SetLanguage(
                                shared::language::from_extension(recipe.file_extension).unwrap(),
                            )),
                            App(HandleKeyEvents(events.clone())),
                        ]
                        .into_iter()
                        .chain(
                            if index == accum_events_len - 1 {
                                recipe.expectations
                            } else {
                                Default::default()
                            }
                            .into_iter()
                            .map(|kind| Expect(kind.clone())),
                        )
                        .collect_vec()
                        .into_boxed_slice()
                    })?;
                    Ok(StepOutput {
                        key: events
                            .last()
                            .map(|event| key_event_to_string(event.code))
                            .unwrap_or(" ".to_string()),
                        description: "".to_string(),
                        term_output: result.unwrap(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(RecipeOutput {
                description: recipe.description.to_string(),
                steps,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let json = serde_json::to_string(&RecipesOutput { recipes_output })?;

    use std::io::Write;

    let mut file = std::fs::File::create("docu/assets/recipes.json")?;

    // Write the JSON to the file
    file.write_all(json.as_bytes())?;

    Ok(())
}

use crossterm::event::KeyCode;
fn key_event_to_string(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Backspace => String::from("backspace"),
        KeyCode::Enter => String::from("enter"),
        KeyCode::Left => String::from("left"),
        KeyCode::Right => String::from("right"),
        KeyCode::Up => String::from("up"),
        KeyCode::Down => String::from("down"),
        KeyCode::Home => String::from("home"),
        KeyCode::End => String::from("end"),
        KeyCode::PageUp => String::from("pageup"),
        KeyCode::PageDown => String::from("pagedown"),
        KeyCode::Tab => String::from("tab"),
        KeyCode::BackTab => String::from("backtab"),
        KeyCode::Delete => String::from("delete"),
        KeyCode::Insert => String::from("insert"),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Null => String::from("Null"),
        KeyCode::Esc => String::from("Esc"),
        // Add more cases as needed
        _ => String::from("Unknown"),
    }
}

#[derive(Clone)]
struct Recipe {
    description: &'static str,
    content: &'static str,
    file_extension: &'static str,
    events: &'static [event::KeyEvent],
    expectations: &'static [ExpectKind],
}
#[derive(serde::Serialize)]
struct StepOutput {
    key: String,
    description: String,
    term_output: String,
}

#[derive(serde::Serialize)]
struct RecipeOutput {
    description: String,
    steps: Vec<StepOutput>,
}

#[derive(serde::Serialize)]
struct RecipesOutput {
    recipes_output: Vec<RecipeOutput>,
}

fn create_nested_vectors<T: Clone>(input: &[T]) -> Vec<Vec<T>> {
    input
        .iter()
        .enumerate()
        .map(|(i, _)| input[0..i].to_vec())
        .chain([input.to_vec()].to_vec())
        .collect()
}
