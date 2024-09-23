use itertools::Itertools;
use my_proc_macros::{key, keys};

use crate::{recipes, test_app::*};

#[test]
fn generate_recipes() -> anyhow::Result<()> {
    let recipes_output = recipes::recipes()
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
                        let temp_path = s
                            .temp_dir()
                            .to_path_buf()
                            .join(&format!("temp.{}", recipe.file_extension))
                            .to_str()
                            .unwrap()
                            .to_string();
                        [
                            App(TerminalDimensionChanged(crate::app::Dimension {
                                width: 60,
                                height: 10,
                            })),
                            App(AddPath(temp_path.clone())),
                            AppLater(Box::new(move || {
                                OpenFile(temp_path.clone().try_into().unwrap())
                            })),
                            // Editor(ApplySyntaxHighlight),
                            App(HandleKeyEvent(key!("esc"))),
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
pub(crate) struct Recipe {
    pub(crate) description: &'static str,
    pub(crate) content: &'static str,
    pub(crate) file_extension: &'static str,
    pub(crate) events: &'static [event::KeyEvent],
    pub(crate) expectations: &'static [ExpectKind],
}
#[derive(serde::Serialize)]
pub(crate) struct StepOutput {
    pub(crate) key: String,
    pub(crate) description: String,
    pub(crate) term_output: String,
}

#[derive(serde::Serialize)]
pub(crate) struct RecipeOutput {
    pub(crate) description: String,
    pub(crate) steps: Vec<StepOutput>,
}

#[derive(serde::Serialize)]
pub(crate) struct RecipesOutput {
    pub(crate) recipes_output: Vec<RecipeOutput>,
}

fn create_nested_vectors<T: Clone>(input: &[T]) -> Vec<Vec<T>> {
    input
        .iter()
        .enumerate()
        .map(|(i, _)| input[0..i].to_vec())
        .chain([input.to_vec()].to_vec())
        .collect()
}
