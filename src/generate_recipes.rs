use itertools::Itertools;
use my_proc_macros::key;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

// TODO:
// 1. Emoji not rendering properly
// 2. keymap legend should be shown as vertical-split, not horizontal

use crate::{position::Position, recipes, rectangle::Rectangle, test_app::*};

#[test]
fn generate_recipes() -> anyhow::Result<()> {
    let recipes_output = recipes::recipes()
        .into_par_iter()
        .map(|recipe| -> anyhow::Result<RecipeOutput> {
            let path = format!("docs/assets/{}/", recipe.description);
            std::fs::create_dir_all(path.clone())?;
            let accum_events = create_nested_vectors(recipe.events);
            let accum_events_len = accum_events.len();
            let width = 60;
            let height = recipe
                .terminal_height
                .unwrap_or(recipe.content.lines().count() + 3);
            let steps = accum_events
                .into_iter()
                .enumerate()
                .map(|(index, events)| -> anyhow::Result<StepOutput> {
                    let result = execute_recipe(|s| {
                        let temp_path = s
                            .temp_dir()
                            .to_path_buf()
                            .join(&format!("temp.{}", recipe.file_extension))
                            .to_str()
                            .unwrap()
                            .to_string();
                        [
                            App(TerminalDimensionChanged(crate::app::Dimension {
                                width,
                                height: height as u16,
                            })),
                            App(AddPath(temp_path.clone())),
                            AppLater(Box::new(move || {
                                OpenFile(temp_path.clone().try_into().unwrap())
                            })),
                            Editor(SetRectangle(Rectangle {
                                origin: Position::default(),
                                width,
                                height: height as u16,
                            })),
                            // Editor(ApplySyntaxHighlight),
                            App(HandleKeyEvent(key!("esc"))),
                            Editor(SetContent(recipe.content.to_string())),
                            Editor(SetLanguage(
                                shared::language::from_extension(recipe.file_extension).unwrap(),
                            )),
                        ]
                        .into_iter()
                        .chain(Some(App(HandleKeyEvents(recipe.prepare_events.to_vec()))))
                        .chain(Some(App(HandleKeyEvents(events.clone()))))
                        .chain(
                            if index == accum_events_len - 1 {
                                recipe.expectations
                            } else {
                                Default::default()
                            }
                            .iter()
                            .map(|kind| Expect(kind.clone())),
                        )
                        .collect_vec()
                        .into_boxed_slice()
                    })?;
                    Ok(StepOutput {
                        key: events
                            .last()
                            .map(|event| event.display())
                            .unwrap_or(" ".to_string()),
                        description: "".to_string(),
                        term_output: result.unwrap(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(RecipeOutput {
                description: recipe.description.to_string(),
                steps,
                terminal_height: height,
                terminal_width: width as usize,
                similar_vim_combos: recipe.similar_vim_combos,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let json = serde_json::to_string(&RecipesOutput { recipes_output })?;

    use std::io::Write;

    let mut file = std::fs::File::create("docs/assets/recipes.json")?;

    // Write the JSON to the file
    file.write_all(json.as_bytes())?;

    Ok(())
}

#[derive(Clone)]
pub(crate) struct Recipe {
    pub(crate) description: &'static str,
    pub(crate) content: &'static str,
    pub(crate) file_extension: &'static str,
    pub(crate) prepare_events: &'static [event::KeyEvent],
    pub(crate) events: &'static [event::KeyEvent],
    pub(crate) expectations: &'static [ExpectKind],
    pub(crate) terminal_height: Option<usize>,
    pub(crate) similar_vim_combos: &'static [&'static str],
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
    pub(crate) terminal_height: usize,
    pub(crate) terminal_width: usize,
    pub(crate) similar_vim_combos: &'static [&'static str],
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