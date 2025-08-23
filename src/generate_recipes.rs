use itertools::{Either, Itertools};
use my_proc_macros::key;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

// TODO:
// 1. Emoji not rendering properly

use crate::{buffer::BufferOwner, position::Position, recipes, rectangle::Rectangle, test_app::*};

#[test]
fn doc_assets_generate_recipes() -> anyhow::Result<()> {
    let recipe_groups = recipes::recipe_groups();
    let contains_only_recipes = recipe_groups.iter().any(|group| {
        group.recipes.iter().any(|recipe| {
            if is_ci::cached() && recipe.only {
                panic!("Recipe \"{}\" is 'only'-ed!", recipe.description)
            }
            recipe.only
        })
    });
    recipe_groups
        .into_par_iter()
        .filter(|recipe_group| {
            !contains_only_recipes || recipe_group.recipes.iter().any(|recipe| recipe.only)
        })
        .map(|recipe_group| -> anyhow::Result<_> {
            let recipes = recipe_group.recipes;
            let (errors, recipes_output): (Vec<_>, Vec<_>) = recipes
                .into_par_iter()
                .filter(|recipe| !contains_only_recipes || recipe.only)
                .map(|recipe| -> Result<RecipeOutput, String> {
                    let run = || {
                        let accum_events = create_nested_vectors(recipe.events);
                        let accum_events_len = accum_events.len();
                        let width = 60;
                        let height = recipe
                            .terminal_height
                            .unwrap_or(recipe.content.lines().count() + 3);
                        let steps = accum_events
                            .into_iter()
                            .enumerate()
                            .map(|(index, events)| -> anyhow::Result<_> {
                                println!(
                                    "------- \n\n\tRunning: {} \n\n----------",
                                    recipe.description
                                );
                                let callback = |s: State| {
                                    let temp_path = s
                                        .temp_dir()
                                        .to_path_buf()
                                        .join(format!("temp.{}", recipe.file_extension))
                                        .to_str()
                                        .expect("Failed to create temp _path")
                                        .to_string();
                                    [
                                        App(TerminalDimensionChanged(crate::app::Dimension {
                                            width,
                                            height: height as u16,
                                        })),
                                        App(AddPath(temp_path.clone())),
                                        AppLater(Box::new(move || OpenFile {
                                            path: temp_path.clone().try_into().unwrap(),
                                            owner: BufferOwner::User,
                                            focus: true,
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
                                            shared::language::from_extension(recipe.file_extension)
                                                .unwrap(),
                                        )),
                                    ]
                                    .into_iter()
                                    .chain(Some(App(HandleKeyEvents(
                                        recipe.prepare_events.to_vec(),
                                    ))))
                                    .chain(Some(App(HandleKeyEvents(events.clone()))))
                                    .chain(
                                        if index == accum_events_len - 1 {
                                            recipe.expectations.clone()
                                        } else {
                                            Default::default()
                                        }
                                        .iter()
                                        .map(|kind| Expect(kind.clone())),
                                    )
                                    .collect_vec()
                                    .into_boxed_slice()
                                };
                                let (term_output, buffer_content) =
                                    execute_recipe(callback, false)?;
                                Ok(StepOutput {
                                    key: events
                                        .last()
                                        .map(|event| event.display())
                                        .unwrap_or(" ".to_string()),
                                    description: "".to_string(),
                                    term_output: term_output.unwrap(),
                                    buffer_content: buffer_content.unwrap(),
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
                    };
                    run().map_err(|err: anyhow::Error| format!("{} {err}", recipe.description,))
                })
                .partition_map(|result| match result {
                    Ok(recipe_output) => Either::Right(recipe_output),
                    Err(err) => Either::Left(err),
                });
            if !errors.is_empty() {
                panic!("Run recipes errors = {}", errors.join("\n"));
            }

            let json = serde_json::to_string(&RecipesOutput { recipes_output })?;

            use std::io::Write;
            let path = format!("docs/static/recipes/{}.json", recipe_group.filename);

            // Write the JSON to the file
            std::fs::File::create(path)?.write_all(json.as_bytes())?;

            Ok(())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct RecipeGroup {
    pub(crate) filename: &'static str,
    pub(crate) recipes: Vec<Recipe>,
}

#[derive(Clone)]
pub(crate) struct Recipe {
    pub(crate) description: &'static str,
    pub(crate) content: &'static str,
    pub(crate) file_extension: &'static str,
    pub(crate) prepare_events: &'static [event::KeyEvent],
    pub(crate) events: &'static [event::KeyEvent],
    pub(crate) expectations: Box<[ExpectKind]>,
    pub(crate) terminal_height: Option<usize>,
    pub(crate) similar_vim_combos: &'static [&'static str],
    pub(crate) only: bool,
}
#[derive(serde::Serialize)]
pub(crate) struct StepOutput {
    pub(crate) key: String,
    pub(crate) description: String,
    pub(crate) term_output: String,
    pub(crate) buffer_content: String,
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
