use std::ops::Range;

use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::Dimension,
    buffer::{Buffer, Line},
    char_index_range::CharIndexRange,
    components::{
        component::{Component, Cursor, SetCursorStyle},
        editor::{Mode, WINDOW_TITLE_HEIGHT},
    },
    context::Context,
    divide_viewport::divide_viewport,
    grid::{CellUpdate, Grid, LineUpdate, RenderContentLineNumber, StyleKey},
    selection::{CharIndex, Selection},
    selection_mode::{self, ByteRange},
    style::Style,
    themes::Theme,
    utils::get_non_consecutive_nums,
};

use super::{
    component::GetGridResult,
    editor::{Editor, Fold},
};

impl Editor {
    pub(crate) fn get_grid(&self, context: &Context, focused: bool) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.render_area();
        let buffer = editor.buffer();
        let rope = buffer.rope();

        let len_lines = rope.len_lines().max(1) as u16;
        let (hidden_parent_lines, visible_parent_lines) =
            self.get_parent_lines().unwrap_or_default();
        let top_offset = hidden_parent_lines.len() as u16;
        let scroll_offset = self.scroll_offset();
        let visible_lines = rope
            .lines()
            .enumerate()
            .skip(scroll_offset as usize)
            .take(height as usize)
            .map(|(line_index, slice)| (line_index, slice.to_string()));

        let visible_lines_grid: Grid = Grid::new(Dimension { height, width });

        let selection = &editor.selection_set.primary_selection();
        // If the buffer selection is updated less recently than the window's scroll offset,

        // use the window's scroll offset.

        let theme = context.theme();

        let folded_lines = match &self.fold {
            Some(fold) => {
                let possible_selections = match fold {
                    Fold::CurrentSelectionMode => self
                        .folded_selections(self.selection_set.primary_selection(), context)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|byte_range| byte_range.range().clone())
                        .collect_vec(),
                    Fold::Cursor => self
                        .selection_set
                        .map(|selection| selection.extended_range())
                        .into_iter()
                        .filter_map(|range| buffer.char_index_range_to_byte_range(range).ok())
                        .collect_vec(),
                    Fold::Mark => self
                        .buffer()
                        .marks()
                        .into_iter()
                        .chain(Some(
                            self.selection_set.primary_selection().extended_range(),
                        ))
                        .filter_map(|range| buffer.char_index_range_to_byte_range(range).ok())
                        .collect_vec(),
                }
                .into_iter()
                .filter_map(|byte_range| buffer.byte_to_line(byte_range.start).ok())
                .unique()
                .collect_vec();
                let line_numbers = possible_selections.clone();
                let focused_line_number = buffer
                    .char_to_line(
                        self.selection_set
                            .primary_selection()
                            .extended_range()
                            .to_char_index(&self.cursor_direction),
                    )
                    .unwrap_or_default();
                let viewport_sections = divide_viewport(
                    &line_numbers,
                    focused_line_number,
                    height as usize,
                    buffer.len_lines().saturating_sub(1),
                );
                viewport_sections
                    .into_iter()
                    .flat_map(|viewport_section| viewport_section.range_vec().into_iter())
                    .unique()
                    .sorted()
                    .filter_map(|line_index| {
                        let start_position = buffer.line_to_position(line_index).ok()?;
                        Some(Line {
                            origin_position: start_position,
                            line: start_position.line,
                            content: buffer
                                .get_line_by_line_index(start_position.line)?
                                .to_string(),
                        })
                    })
                    .collect_vec()
            }
            None => hidden_parent_lines,
        };
        let folded_line_ranges = folded_lines
            .iter()
            .map(|line| line.line..line.line + 1)
            .collect_vec();
        let visible_line_range = self.visible_line_range();
        let primary_selection_primary_cursor = buffer
            .char_to_position(selection.to_char_index(&self.cursor_direction))
            .ok()
            .map(|position| CellUpdate::new(position).set_is_cursor(true));
        let updates = self.get_highlight_spans(
            context,
            &visible_line_range,
            &folded_line_ranges,
            &visible_parent_lines,
        );

        let folded_grid = {
            let boundaries = folded_line_ranges
                .into_iter()
                .map(|folded_line_range| Boundary::new(&buffer, folded_line_range))
                .collect_vec();
            let non_consecutive_lines =
                get_non_consecutive_nums(&folded_lines.iter().map(|line| line.line).collect_vec());
            let updates = updates
                .clone()
                .into_iter()
                .flat_map(|span| span.to_cell_updates(&buffer, theme, &boundaries))
                .collect_vec();
            folded_lines.into_iter().fold(
                Grid::new(Dimension { height: 0, width }),
                |grid, line| {
                    let updates = updates
                        .iter()
                        .chain(&primary_selection_primary_cursor)
                        .filter_map(|update| {
                            if update.position.line == line.line {
                                Some(update.clone().set_position_line(0))
                            } else {
                                None
                            }
                        })
                        .collect_vec();
                    let line_updates = if non_consecutive_lines.contains(&line.line) {
                        [LineUpdate {
                            line_index: 0,
                            style: Style::new().background_color(theme.ui.parent_lines_background),
                        }]
                        .to_vec()
                    } else {
                        Default::default()
                    };
                    grid.merge_vertical(Grid::new(Dimension { height: 1, width }).render_content(
                        &line.content,
                        RenderContentLineNumber::LineNumber {
                            start_line_index: line.line,
                            max_line_number: len_lines as usize,
                        },
                        updates,
                        line_updates,
                        theme,
                    ))
                },
            )
        };

        let grid = if self.fold.is_some() {
            let folded_lines_length = folded_grid.rows.len();
            folded_grid.merge_vertical(Grid::new(Dimension {
                height: height.saturating_sub(folded_lines_length as u16),
                width,
            }))
        } else {
            let visible_lines_updates = {
                let boundaries = [Boundary::new(&buffer, visible_line_range)];
                updates
                    .iter()
                    .flat_map(|span| span.to_cell_updates(&buffer, theme, &boundaries))
                    .chain(primary_selection_primary_cursor)
                    .collect_vec()
            };
            let visible_lines_grid = visible_lines_grid.render_content(
                &visible_lines.map(|(_, line)| line).join(""),
                RenderContentLineNumber::LineNumber {
                    start_line_index: scroll_offset as usize,
                    max_line_number: len_lines as usize,
                },
                visible_lines_updates
                    .clone()
                    .into_iter()
                    .map(|cell_update| CellUpdate {
                        position: cell_update.position.move_up(scroll_offset as usize),
                        ..cell_update
                    })
                    .collect_vec(),
                Vec::new(),
                theme,
            );
            let cursor_beyond_view_bottom =
                if let Some(cursor_position) = visible_lines_grid.get_cursor_position() {
                    cursor_position
                    .line
                    .saturating_sub(height.saturating_sub(1).saturating_sub(top_offset) as usize)
                } else {
                    0
                };
            let visible_lines_grid = visible_lines_grid.clamp_top(cursor_beyond_view_bottom);
            let clamp_bottom_by = visible_lines_grid
                .dimension()
                .height
                .saturating_sub(height)
                .saturating_add(top_offset)
                .saturating_sub(cursor_beyond_view_bottom as u16);
            let bottom = visible_lines_grid.clamp_bottom(clamp_bottom_by);
            folded_grid.merge_vertical(bottom)
        };

        debug_assert_eq!(grid.rows.len(), height as usize);
        debug_assert!(grid.rows.iter().all(|row| row.len() == width as usize));
        let window_title_style = if focused {
            theme.ui.window_title_focused
        } else {
            theme.ui.window_title_unfocused
        };

        // NOTE: due to performance issue, we only highlight the content that are within view
        // This might result in some incorrectness, but that's a reasonable trade-off, because
        // highlighting the entire file becomes sluggish when the file has more than a thousand lines.

        let title_grid = Grid::new(Dimension {
            height: WINDOW_TITLE_HEIGHT as u16,
            width: editor.dimension().width,
        })
        .render_content(
            &self.title(context),
            RenderContentLineNumber::NoLineNumber,
            Vec::new(),
            [LineUpdate {
                line_index: 0,
                style: window_title_style,
            }]
            .to_vec(),
            theme,
        );

        let grid = title_grid.merge_vertical(grid);
        let cursor_position = grid.get_cursor_position();
        let style = match self.mode {
            Mode::Normal => SetCursorStyle::BlinkingBlock,
            Mode::Insert => SetCursorStyle::BlinkingBar,
            _ => SetCursorStyle::BlinkingUnderScore,
        };
        GetGridResult {
            cursor: cursor_position.map(|position| Cursor::new(position, style)),
            grid,
        }
    }

    fn get_highlight_spans(
        &self,
        context: &Context,
        visible_line_range: &Range<usize>,
        folded_line_ranges: &Vec<Range<usize>>,
        visible_parent_lines: &Vec<Line>,
    ) -> Vec<HighlightSpan> {
        use StyleKey::*;
        let theme = context.theme();
        let buffer = self.buffer();
        let possible_selections = if self.selection_set.mode.is_contiguous() {
            Default::default()
        } else {
            self.possible_selections(self.selection_set.primary_selection(), context)
                // .possible_selections_in_line_number_range( self.selection_set.primary_selection(), context, )
                .unwrap_or_default()
        }
        .into_iter()
        .map(|range| HighlightSpan {
            set_symbol: None,
            is_cursor: false,
            range: HighlightSpanRange::ByteRange(range.range().clone()),
            source: Source::StyleKey(UiPossibleSelection),
        });

        let marks = buffer.marks().into_iter().map(|mark| HighlightSpan {
            set_symbol: None,
            is_cursor: false,
            source: Source::StyleKey(UiMark),
            range: HighlightSpanRange::CharIndexRange(mark),
        });
        let secondary_selections = &self.selection_set.secondary_selections();

        let selection = &self.selection_set.primary_selection();
        let primary_selection = HighlightSpan {
            set_symbol: None,
            is_cursor: false,
            range: HighlightSpanRange::CharIndexRange(selection.extended_range()),
            source: Source::StyleKey(UiPrimarySelection),
        };

        let primary_selection_anchors =
            selection.anchors().into_iter().map(|anchor| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndexRange(anchor),
                source: Source::StyleKey(UiPrimarySelectionAnchors),
            });
        let primary_selection_secondary_cursor = if self.mode == Mode::Insert {
            None
        } else {
            Some(HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndex(
                    selection.to_char_index(&self.cursor_direction.reverse()),
                ),
                source: Source::StyleKey(StyleKey::UiPrimarySelectionSecondaryCursor),
            })
        };

        let secondary_selection =
            secondary_selections
                .iter()
                .map(|secondary_selection| HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    range: HighlightSpanRange::CharIndexRange(secondary_selection.extended_range()),
                    source: Source::StyleKey(UiSecondarySelection),
                });

        let seconday_selection_anchors = secondary_selections.iter().flat_map(|selection| {
            selection.anchors().into_iter().map(|anchor| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndexRange(anchor),
                source: Source::StyleKey(UiSecondarySelectionAnchors),
            })
        });
        let secondary_selection_cursors =
            secondary_selections.iter().flat_map(|secondary_selection| {
                [
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&self.cursor_direction.reverse()),
                        ),
                        source: Source::Style(theme.ui.secondary_selection_secondary_cursor),
                    },
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&self.cursor_direction),
                        ),
                        source: Source::Style(theme.ui.secondary_selection_primary_cursor),
                    },
                ]
                .into_iter()
                .collect::<Vec<_>>()
            });
        let content = buffer.rope().to_string();

        let diagnostics = buffer.diagnostics();
        let diagnostics = diagnostics
            .iter()
            .sorted_by(|a, b| a.severity.cmp(&b.severity))
            .rev()
            .map(|diagnostic| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndexRange(diagnostic.range),
                source: Source::StyleKey(match diagnostic.severity {
                    Some(DiagnosticSeverity::ERROR) => DiagnosticsError,
                    Some(DiagnosticSeverity::WARNING) => DiagnosticsWarning,
                    Some(DiagnosticSeverity::INFORMATION) => DiagnosticsInformation,
                    Some(DiagnosticSeverity::HINT) => DiagnosticsHint,
                    _ => DiagnosticsDefault,
                }),
            });

        let jumps = self.jumps().into_iter().enumerate().map(|(index, jump)| {
            let style = if index % 2 == 0 {
                theme.ui.jump_mark_even
            } else {
                theme.ui.jump_mark_odd
            };
            HighlightSpan {
                set_symbol: Some(jump.character.to_string()),
                is_cursor: false,
                source: Source::Style(style),
                range: HighlightSpanRange::CharIndex(
                    jump.selection.to_char_index(&self.cursor_direction),
                ),
            }
        });
        let extra_decorations = buffer.decorations().iter().flat_map(|decoration| {
            Some(HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndexRange(
                    decoration
                        .selection_range()
                        .to_char_index_range(&buffer)
                        .ok()?,
                ),
                source: Source::StyleKey(decoration.style_key().clone()),
            })
        });

        let visible_line_byte_range = buffer
            .line_range_to_byte_range(&visible_line_range)
            .unwrap_or_default();
        let spans = buffer.highlighted_spans();
        let filtered_highlighted_spans = {
            filter_items_by_range(
                &spans,
                visible_line_byte_range.start,
                visible_line_byte_range.end,
                |span| span.byte_range.clone(),
            )
            .iter()
            .chain(folded_line_ranges.iter().flat_map(|line_range| {
                let byte_range = buffer
                    .line_range_to_byte_range(&line_range)
                    .unwrap_or_default();
                filter_items_by_range(&spans, byte_range.start, byte_range.end, |span| {
                    span.byte_range.clone()
                })
            }))
            .map(|span| HighlightSpan {
                range: HighlightSpanRange::ByteRange(span.byte_range.clone()),
                source: Source::StyleKey(span.style_key.clone()),
                set_symbol: None,
                is_cursor: false,
            })
        };
        let custom_regex_highlights = lazy_regex::regex!("(?i)#[0-9a-f]{6}")
            .find_iter(&content)
            .map(|m| (m.as_str().to_string(), m.range()))
            .filter_map(|(hex, range)| {
                let color = crate::themes::Color::from_hex(&hex).ok()?;
                Some(HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    range: HighlightSpanRange::ByteRange(range),
                    source: Source::Style(
                        Style::new()
                            .background_color(color)
                            .foreground_color(color.get_contrasting_color()),
                    ),
                })
            });

        let regex_highlight_rules = self
            .regex_highlight_rules
            .iter()
            .filter_map(|rule| {
                let captures = rule.regex.captures(&content)?;
                let get_highlight_span = |name: &'static str, source: Source| {
                    let match_ = captures.name(name)?;

                    Some(HighlightSpan {
                        source,
                        range: HighlightSpanRange::ByteRange(match_.range()),
                        set_symbol: None,
                        is_cursor: false,
                    })
                };
                Some(
                    rule.capture_styles
                        .iter()
                        .flat_map(|capture_style| {
                            get_highlight_span(
                                capture_style.capture_name,
                                capture_style.source.clone(),
                            )
                        })
                        .collect_vec(),
                )
            })
            .flatten();

        let visible_parent_lines = if self.fold.is_none() {
            Box::new(visible_parent_lines.into_iter().map(|line| HighlightSpan {
                source: Source::StyleKey(StyleKey::ParentLine),
                range: HighlightSpanRange::Line(line.line),
                set_symbol: None,
                is_cursor: false,
            })) as Box<dyn Iterator<Item = HighlightSpan>>
        } else {
            Box::new(std::iter::empty())
        };
        vec![]
            .into_iter()
            .chain(visible_parent_lines)
            .chain(filtered_highlighted_spans)
            .chain(extra_decorations)
            .chain(possible_selections)
            .chain(Some(primary_selection))
            .chain(secondary_selection)
            .chain(primary_selection_anchors)
            .chain(seconday_selection_anchors)
            .chain(marks)
            .chain(diagnostics)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors)
            .chain(custom_regex_highlights)
            .chain(regex_highlight_rules)
            .collect_vec()
    }

    pub(crate) fn possible_selections_in_line_number_range(
        &self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(selection, true, context)?;
        if self.selection_set.mode.is_contiguous() {
            return Ok(Vec::new());
        }

        let line_range = self.visible_line_range();
        object.selections_in_line_number_range(
            &selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            },
            [line_range].to_vec(),
        )
    }

    pub(crate) fn possible_selections(
        &self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self
            .get_selection_mode_trait_object(selection, true, context)?
            .iter_filtered(selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            })?
            .collect())
    }

    pub(crate) fn folded_selections(
        &self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self
            .get_selection_mode_trait_object(selection, true, context)?
            .iter_folded(selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            })?
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HighlightSpan {
    pub(crate) source: Source,
    pub(crate) range: HighlightSpanRange,
    pub(crate) set_symbol: Option<String>,
    pub(crate) is_cursor: bool,
}

impl HighlightSpan {
    /// Convert this `HighlightSpans` into `Vec<CellUpdate>`,
    /// only perform conversions for positions that falls within the given `boundaries`,
    /// so that we can minimize the call to the expensive `buffer.xxx_to_position` methods
    fn to_cell_updates(
        &self,
        buffer: &Buffer,
        theme: &Theme,
        boundaries: &[Boundary],
    ) -> Vec<CellUpdate> {
        boundaries
            .iter()
            .filter_map(|boundary| {
                let char_index_range: CharIndexRange = match &self.range {
                    HighlightSpanRange::CharIndexRange(range) => {
                        range_intersection(&(range.start..range.end), &boundary.char_index_range)?
                            .into()
                    }
                    HighlightSpanRange::ByteRange(range) => buffer
                        .byte_range_to_char_index_range(&range_intersection(
                            range,
                            &boundary.byte_range,
                        )?)
                        .ok()?,
                    HighlightSpanRange::CharIndex(char_index) => range_intersection(
                        &(*char_index..(*char_index + 1)),
                        &boundary.char_index_range,
                    )?
                    .into(),
                    HighlightSpanRange::Line(line) => buffer
                        .line_range_to_char_index_range(range_intersection(
                            &(*line..line + 1),
                            &boundary.line_range,
                        )?)
                        .ok()?,
                };
                Some(
                    char_index_range
                        .iter()
                        .flat_map(|char_index| {
                            let position = buffer.char_to_position(char_index).ok()?;
                            Some(CellUpdate {
                                position,
                                symbol: self.set_symbol.clone(),
                                style: match &self.source {
                                    Source::StyleKey(key) => theme.get_style(key),
                                    Source::Style(style) => *style,
                                },
                                is_cursor: self.is_cursor,
                                source: match &self.source {
                                    Source::StyleKey(key) => Some(key.clone()),
                                    _ => None,
                                },
                            })
                        })
                        .collect_vec(),
                )
            })
            .flatten()
            .collect()
    }
}

fn range_intersection<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Option<Range<T>> {
    let start = std::cmp::max(a.start, b.start);
    let end = std::cmp::min(a.end, b.end);
    if start < end {
        Some(start..end)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Source {
    StyleKey(StyleKey),
    Style(Style),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum HighlightSpanRange {
    CharIndexRange(CharIndexRange),
    ByteRange(Range<usize>),
    CharIndex(CharIndex),
    /// 0-based index.
    Line(usize),
}

struct Boundary {
    byte_range: Range<usize>,
    char_index_range: Range<CharIndex>,
    /// 0-based index.
    line_range: Range<usize>,
}
impl Boundary {
    fn new(buffer: &Buffer, visible_line_range: Range<usize>) -> Self {
        let Range { start, end } = visible_line_range;
        let byte_start = buffer.line_to_byte(start).unwrap_or(0);
        let byte_end = buffer.line_to_byte(end).unwrap_or(u32::MAX as usize);
        let char_index_start = buffer.line_to_char(start).unwrap_or(CharIndex(0));
        let char_index_end = buffer
            .line_to_char(end)
            .unwrap_or(CharIndex(u32::MAX as usize));
        Self {
            byte_range: byte_start..byte_end,
            char_index_range: char_index_start..char_index_end,
            line_range: start..end,
        }
    }
}

#[cfg(test)]
mod test_render_editor {
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;

    use crate::{
        components::{component::Component, editor::Editor},
        context::Context,
        rectangle::Rectangle,
    };

    impl Arbitrary for Rectangle {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            const MAX_CHARACTER_WIDTH: u8 = 4;
            const LINE_NUMBER_UI_WIDTH: u8 = 2;
            const PADDING_FOR_CURSOR_AT_LAST_COLUMN: u8 = 1;
            Self {
                origin: Default::default(),
                width: (u8::arbitrary(g) / 10).max(
                    MAX_CHARACTER_WIDTH + LINE_NUMBER_UI_WIDTH + PADDING_FOR_CURSOR_AT_LAST_COLUMN,
                ) as u16,
                height: (u8::arbitrary(g) / 10) as u16,
            }
        }
    }

    #[quickcheck]
    fn get_grid_cells_should_be_always_within_bound(rectangle: Rectangle, content: String) -> bool {
        let mut editor = Editor::from_text(None, &content);
        editor.set_rectangle(rectangle.clone());
        let grid = editor.get_grid(&Context::default(), false);
        let cells = grid.grid.to_positioned_cells();
        cells.into_iter().all(|cell| {
            cell.position.line < (rectangle.height as usize)
                && cell.position.column < (rectangle.width as usize)
        })
    }
}

/// This functions utilize binary search to quickly extract the
/// ranges that intersects with the given `start..end`.
///
/// It computes the result without iterating through every item of `items`.
///
/// Precondition: `items` must be sorted by their range
fn filter_items_by_range<T, F>(items: &[T], start: usize, end: usize, get_range: F) -> &[T]
where
    F: Fn(&T) -> Range<usize>,
{
    debug_assert!(
        items.iter().map(&get_range).collect_vec()
            == items
                .iter()
                .map(&get_range)
                .sorted_by_key(|range| (range.start, range.end))
                .collect_vec(),
    );
    debug_assert!(start <= end);

    // Find the start index
    // We consider an item to be "greater" than start if its range's end is greater than start
    let start_idx = items.partition_point(|item| get_range(item).end <= start);

    // Find the end index
    // We consider an item to be "less than or equal" to end if its range's start is less than or equal to end
    let end_idx = items.partition_point(|item| get_range(item).start < end);

    // Ensure start_idx is not greater than end_idx to avoid panics
    let safe_start = std::cmp::min(start_idx, items.len());
    let safe_end = std::cmp::max(safe_start, end_idx);

    debug_assert!(safe_start <= safe_end);

    // Return the slice containing items that potentially overlap with [start, end]
    // Expect the result to be the same as using `range_intersection`
    debug_assert_eq!(
        (safe_start..safe_end).collect_vec(),
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| range_intersection(&get_range(item), &(start..end)).is_some())
            .map(|(index, _)| index)
            .collect_vec()
    );

    &items[safe_start..safe_end]
}

#[cfg(test)]
mod test_range_search {
    use super::filter_items_by_range;

    #[test]
    fn case_1() {
        let items = vec![(0..5), (3..8), (7..10), (9..15)];
        let result = filter_items_by_range(&items, 6, 12, |r| r.clone());
        assert_eq!(result, &[(3..8), (7..10), (9..15)]);
    }
}
