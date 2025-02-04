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

use StyleKey::*;

impl Editor {
    pub(crate) fn get_grid(&self, context: &Context, focused: bool) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.render_area();
        let buffer = editor.buffer();
        let rope = buffer.rope();
        let content = rope.to_string();

        let diagnostics = buffer.diagnostics();

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
        let secondary_selections = &editor.selection_set.secondary_selections();
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
        let primary_selection_primary_cursor = if focused {
            buffer
                .char_to_position(selection.to_char_index(&editor.cursor_direction))
                .ok()
                .map(|position| CellUpdate::new(position).set_is_cursor(true))
        } else {
            None
        };

        let primary_selection_secondary_cursor = if self.mode == Mode::Insert {
            None
        } else {
            Some(HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndex(
                    selection.to_char_index(&editor.cursor_direction.reverse()),
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
                            secondary_selection.to_char_index(&editor.cursor_direction.reverse()),
                        ),
                        source: Source::Style(theme.ui.secondary_selection_secondary_cursor),
                    },
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&editor.cursor_direction),
                        ),
                        source: Source::Style(theme.ui.secondary_selection_primary_cursor),
                    },
                ]
                .into_iter()
                .collect::<Vec<_>>()
            });
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

        let jumps = editor.jumps().into_iter().enumerate().map(|(index, jump)| {
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

        let hidden_parent_lines = match &self.fold {
            Some(fold) => {
                // TODO: add test case for this
                let possible_selections = match fold {
                    Fold::CurrentSelectionMode => self
                        .possible_selections(self.selection_set.primary_selection(), context)
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
                let viewport_sections = sections_divider::divide_viewport(
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

        let hidden_parent_line_ranges = hidden_parent_lines
            .iter()
            .map(|line| line.line..line.line + 1);
        let visible_line_range = self.visible_line_range();
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
            .chain(hidden_parent_line_ranges.clone().flat_map(|line_range| {
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

        let visible_parent_lines = visible_parent_lines.into_iter().map(|line| HighlightSpan {
            source: Source::StyleKey(StyleKey::ParentLine),
            range: HighlightSpanRange::Line(line.line),
            set_symbol: None,
            is_cursor: false,
        });
        let updates = vec![]
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
            .collect_vec();
        let hidden_parent_lines_grid = {
            let boundaries = hidden_parent_line_ranges
                .into_iter()
                .map(|hidden_parent_line_range| Boundary::new(&buffer, hidden_parent_line_range))
                .collect_vec();
            let non_consecutive_lines = get_non_consecutive_nums(
                &hidden_parent_lines
                    .iter()
                    .map(|line| line.line)
                    .collect_vec(),
            );
            let updates = updates
                .clone()
                .into_iter()
                .flat_map(|span| span.to_cell_updates(&buffer, theme, &boundaries))
                .collect_vec();
            hidden_parent_lines.into_iter().fold(
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
            let hidden_parent_lines_length = hidden_parent_lines_grid.rows.len();
            hidden_parent_lines_grid.merge_vertical(Grid::new(Dimension {
                height: height.saturating_sub(hidden_parent_lines_length as u16),
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
            hidden_parent_lines_grid.merge_vertical(bottom)
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
mod sections_divider {

    use std::collections::HashSet;

    use itertools::Itertools;

    use crate::{components::render_editor::calculate_distribution, utils::distribute_items};

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) struct ViewportSection {
        /// Inclusive
        start: usize,
        /// Inclusive
        end: usize,
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) struct ViewportSectionWithOrigin {
        /// Inclusive
        start: usize,
        /// Inclusive
        end: usize,
        /// Inclusive
        start_original: usize,
        /// Inclusive
        end_original: usize,
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) struct ViewportSectionOnlyOrigin {
        /// Inclusive
        start_original: usize,
        /// Inclusive
        end_original: usize,
    }
    impl ViewportSectionOnlyOrigin {
        fn into_viewport_section(self) -> ViewportSection {
            ViewportSection {
                start: self.start_original,
                end: self.end_original,
            }
        }

        fn len(&self) -> usize {
            self.end_original + 1 - self.start_original
        }

        fn range_set(&self) -> HashSet<usize> {
            (self.start_original..self.end_original + 1).collect()
        }
    }
    impl ViewportSectionWithOrigin {
        fn range_set(&self) -> HashSet<usize> {
            (self.start..self.end + 1).collect()
        }

        fn into_viewport_section_only_origin(self) -> ViewportSectionOnlyOrigin {
            ViewportSectionOnlyOrigin {
                start_original: self.start_original,
                end_original: self.end_original,
            }
        }

        fn into_viewport_section(self) -> ViewportSection {
            ViewportSection {
                start: self.start,
                end: self.end,
            }
        }
    }

    impl ViewportSection {
        fn range_set(&self) -> HashSet<usize> {
            (self.start..self.end + 1).collect()
        }

        pub(crate) fn range_vec(&self) -> Vec<usize> {
            (self.start..self.end + 1).collect()
        }

        fn len(&self) -> usize {
            self.end + 1 - self.start
        }
    }

    pub(crate) fn divide_viewport(
        line_numbers: &[usize],
        focused_line_number: usize,
        viewport_height: usize,
        max_line_index: usize,
    ) -> Vec<ViewportSection> {
        debug_assert!(line_numbers
            .iter()
            .all(|line_number| *line_number <= max_line_index));
        let line_numbers = line_numbers
            .into_iter()
            .map(|line_number| *line_number)
            .unique()
            .sorted()
            .collect_vec();
        if line_numbers.len() > viewport_height {
            return extract_centered_window(&line_numbers, focused_line_number, viewport_height)
                .into_iter()
                .map(|line_number| ViewportSection {
                    start: line_number,
                    end: line_number,
                })
                .collect_vec();
        }

        if line_numbers.is_empty() {
            return Vec::new();
        }
        let result = divide_viewport_impl(
            line_numbers
                .into_iter()
                .map(|line_number| ViewportSectionOnlyOrigin {
                    start_original: line_number,
                    end_original: line_number,
                })
                .collect_vec(),
            viewport_height,
            max_line_index,
        );
        // println!("result = {result:?}");
        result
    }

    fn extract_centered_window<T: Ord + Eq + Clone + std::fmt::Debug>(
        elements: &[T],
        element: T,
        window_size: usize,
    ) -> Vec<T> {
        debug_assert!(elements.contains(&element));
        debug_assert!(elements.iter().is_sorted());
        let Some(index) = elements
            .iter()
            .position(|line_number| *line_number == element)
        else {
            // This should be unreachable
            // but let's say it happens we just simply trim `elements` by `window_size`.
            return elements
                .into_iter()
                .take(window_size)
                .map(|element| element.clone())
                .collect_vec();
        };

        let (go_left_by, go_right_by) = distribute_items(window_size.saturating_sub(1), 2)
            .into_iter()
            .collect_tuple()
            .unwrap();

        let go_left_by =
            go_left_by + (index + go_right_by).saturating_sub(elements.len().saturating_sub(1));

        let go_right_by = go_right_by + go_left_by.saturating_sub(index);
        let start_index = index.saturating_sub(go_left_by);
        let end_index = (index + go_right_by).min(elements.len().saturating_sub(1));
        let result = elements[start_index..end_index + 1].to_vec();

        debug_assert!(result.contains(&element));
        debug_assert_eq!(result.len(), window_size);
        result
    }
    fn divide_viewport_impl(
        input_sections: Vec<ViewportSectionOnlyOrigin>,
        viewport_height: usize,
        max_line_index: usize,
    ) -> Vec<ViewportSection> {
        // println!("\n\n input sections = {:?}", input_sections);
        if viewport_height <= input_sections.len() {
            return input_sections
                .into_iter()
                .map(|section| section.into_viewport_section())
                .collect_vec();
        }

        let sections_length = input_sections.len();
        // println!("viewport_height = {viewport_height}");
        // println!( "input_lengths = {:?}", input_sections .iter() .map(|section| section.len()) .collect_vec() );
        let input_lengths = input_sections
            .iter()
            .map(|section| section.len())
            .collect_vec();
        let context_lines_lengths = calculate_distribution(
            &input_lengths,
            (viewport_height as usize).saturating_sub(input_lengths.iter().sum()),
        ); //distribute_items(viewport_height as usize, sections_length);
           // println!("context_lines_lengths = {context_lines_lengths:?}");

        let result_sections = input_sections
            .iter()
            .zip(context_lines_lengths)
            .map(|(section, context_lines_length)| {
                // println!("section = {:?}", section);
                let (lower_context_lines_length, upper_context_lines_length) =
                    distribute_items(context_lines_length.saturating_sub(section.len()), 2)
                        .into_iter()
                        .collect_tuple()
                        .unwrap();
                let lower_context_lines_length = lower_context_lines_length
                    + upper_context_lines_length
                        .saturating_sub(max_line_index.saturating_sub(section.end_original));
                let upper_context_lines_length = upper_context_lines_length
                    + lower_context_lines_length.saturating_sub(section.start_original);
                // println!("lower_context_lines_length = {lower_context_lines_length}");
                // println!("upper_context_lines_length = {upper_context_lines_length}");
                ViewportSectionWithOrigin {
                    start: section
                        .start_original
                        .saturating_sub(lower_context_lines_length as usize),
                    end: section
                        .end_original
                        .saturating_add(upper_context_lines_length)
                        .min(max_line_index),
                    start_original: section.start_original,
                    end_original: section.end_original,
                }
            })
            .collect_vec();
        // println!("result_sections = {:?}", result_sections);
        // Merge overlapping sections
        let merged = result_sections.iter().fold(
            vec![],
            |accum: Vec<ViewportSectionWithOrigin>, current_section| {
                if let Some((last_section, init)) = accum.split_last() {
                    let last_range_set = last_section.range_set();
                    let current_range_set = current_section.range_set();
                    let intersected_lines = last_range_set
                        .intersection(&current_range_set)
                        .collect_vec();
                    if intersected_lines.is_empty() {
                        init.into_iter()
                            .map(|section| section.clone())
                            .chain(Some(last_section.clone()))
                            .chain(Some(current_section.clone()))
                            .collect_vec()
                    } else {
                        let merged_section = ViewportSectionWithOrigin {
                            start: last_section.start_original,
                            end: current_section.end_original,
                            start_original: last_section.start_original,
                            end_original: current_section.end_original,
                        };
                        init.into_iter()
                            .map(|section| section.clone())
                            .chain(Some(merged_section))
                            .collect()
                    }
                } else {
                    accum
                        .into_iter()
                        .chain(Some(current_section.clone()))
                        .collect_vec()
                }
            },
        );
        if merged.len() < sections_length {
            divide_viewport_impl(
                merged
                    .into_iter()
                    .map(|section| section.into_viewport_section_only_origin())
                    .collect_vec(),
                viewport_height,
                max_line_index,
            )
        } else {
            result_sections
                .into_iter()
                .map(|section| section.into_viewport_section())
                .collect_vec()
        }
    }

    #[cfg(test)]
    mod tests {
        use quickcheck::{Arbitrary, Gen};
        use quickcheck_macros::quickcheck;
        use rand::random;

        use super::*;

        #[test]
        fn prioritize_above_over_bottom_for_uneven_split() {
            let result = divide_viewport(&[10], 10, 4, 100);
            // Two above line 10
            // One belowe line 10
            assert_eq!(result, vec![ViewportSection { start: 8, end: 11 }]);
        }

        #[test]
        fn line_numbers_length_more_than_viewport_height_focus_start() {
            let result = divide_viewport(&[1, 2, 3, 4], 1, 3, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 1, end: 1 },
                    ViewportSection { start: 2, end: 2 },
                    ViewportSection { start: 3, end: 3 }
                ]
            );
        }

        #[test]
        fn line_numbers_length_more_than_viewport_height_focus_middle() {
            let result = divide_viewport(&[1, 2, 3, 4, 5], 3, 3, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 2, end: 2 },
                    ViewportSection { start: 3, end: 3 },
                    ViewportSection { start: 4, end: 4 },
                ]
            )
        }

        #[test]
        fn line_numbers_length_more_than_viewport_height_focus_end() {
            let result = divide_viewport(&[1, 2, 3, 4, 5], 5, 3, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 3, end: 3 },
                    ViewportSection { start: 4, end: 4 },
                    ViewportSection { start: 5, end: 5 },
                ]
            )
        }

        #[test]
        fn test_single_cursor_line() {
            let result = divide_viewport(&[10], 10, 5, 100);
            assert_eq!(result, vec![ViewportSection { start: 8, end: 12 }]);
        }

        #[test]
        fn test_duplicate_lines() {
            let result = divide_viewport(&[10, 10, 10], 10, 5, 100);
            assert_eq!(result, vec![ViewportSection { start: 8, end: 12 }]);
        }

        #[test]
        fn test_adjacent_lines_merged() {
            let result = divide_viewport(&[10, 11], 10, 5, 100);
            assert_eq!(result, vec![ViewportSection { start: 8, end: 12 }]);
        }

        #[test]
        fn test_distant_lines_split() {
            let result = divide_viewport(&[10, 20], 10, 6, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 9, end: 11 },
                    ViewportSection { start: 19, end: 21 }
                ]
            );
        }

        #[test]
        fn test_smaller_line_numbers_receive_larger_portions_on_uneven_division() {
            let result = divide_viewport(&[10, 11, 12, 13], 11, 6, 100);
            assert_eq!(result, vec![ViewportSection { start: 9, end: 14 }]);
        }

        #[test]
        fn test_first_line_edge_case() {
            let result = divide_viewport(&[0], 0, 4, 100);
            assert_eq!(result, vec![ViewportSection { start: 0, end: 3 }]);
        }

        #[test]
        fn test_last_line_edge_case() {
            let result = divide_viewport(&[99], 99, 4, 100);
            assert_eq!(
                result,
                vec![ViewportSection {
                    start: 97,
                    end: 100
                }]
            );
        }

        #[test]
        fn test_mixed_edge_cases_1() {
            let result = divide_viewport(&[0, 1, 98, 99], 1, 8, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 0, end: 3 },
                    ViewportSection {
                        start: 97,
                        end: 100
                    }
                ]
            );
        }

        #[test]
        fn test_mixed_edge_cases_2() {
            let result = divide_viewport(&[0, 1, 100], 1, 8, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 0, end: 3 },
                    ViewportSection {
                        start: 97,
                        end: 100
                    }
                ]
            );
        }

        #[test]
        fn test_even_distribution() {
            let result = divide_viewport(&[10, 20, 30], 20, 9, 100);
            assert_eq!(
                result,
                vec![
                    ViewportSection { start: 9, end: 11 },
                    ViewportSection { start: 19, end: 21 },
                    ViewportSection { start: 29, end: 31 }
                ]
            );
        }

        #[test]
        fn test_sections_within_viewport() {
            let viewport_height = 8;
            let lines = vec![5, 6, 15];
            let result = divide_viewport(&lines, 6, viewport_height, 100);

            let total_lines: usize = result
                .iter()
                .map(|section| section.end - section.start + 1)
                .sum();
            assert!(
                total_lines <= viewport_height,
                "Total lines {} exceeds viewport height {}",
                total_lines,
                viewport_height
            );
        }

        #[derive(Debug, Clone)]
        struct Input {
            max_line_index: usize,
            viewport_height: usize,
            line_numbers: Vec<usize>,
        }

        impl Arbitrary for Input {
            fn arbitrary(g: &mut Gen) -> Self {
                let viewport_height = (usize::arbitrary(g) % 10).max(1);

                // `max_line_index` should be always larger than `viewport_height`
                // for this specific quickcheck test case
                let max_line_index = (usize::arbitrary(g) % 10).max(viewport_height);

                let line_numbers_length = (usize::arbitrary(g) % 10).max(1);
                let line_numbers = (0..line_numbers_length)
                    .map(|_| usize::arbitrary(g) % (max_line_index + 1))
                    .collect_vec();

                Input {
                    max_line_index,
                    line_numbers,
                    viewport_height,
                }
            }
        }
        #[quickcheck]
        fn sum_of_viewport_sections_height_should_equal_viewport_height(input: Input) -> bool {
            let Input {
                max_line_index,
                viewport_height,
                line_numbers,
            } = input;
            let random_index = random::<usize>() % line_numbers.len();
            let focused_line_number = line_numbers.get(random_index).unwrap();
            let sections = divide_viewport(
                &line_numbers,
                *focused_line_number,
                viewport_height,
                max_line_index,
            );

            let sum: usize = sections
                .into_iter()
                .map(|section| section.range_set().len())
                .sum();
            sum == viewport_height
        }

        #[quickcheck]
        fn no_viewport_sections_should_intersect(input: Input) -> bool {
            let Input {
                max_line_index,
                viewport_height,
                line_numbers,
            } = input;
            let random_index = random::<usize>() % line_numbers.len();
            let focused_line_number = line_numbers.get(random_index).unwrap();
            let sections = divide_viewport(
                &line_numbers,
                *focused_line_number,
                viewport_height,
                max_line_index,
            );

            sections.iter().enumerate().all(|(index, section)| {
                !sections
                    .iter()
                    .enumerate()
                    .filter(|(other_index, _)| other_index != &index)
                    .any(|(_, other_section)| {
                        other_section
                            .range_set()
                            .intersection(&section.range_set())
                            .count()
                            > 0
                    })
            })
        }
    }
}

fn calculate_distribution(initial: &[usize], resources: usize) -> Vec<usize> {
    let mut final_amounts = initial.to_vec();
    let mut remaining = resources;

    while remaining > 0 {
        // Find index of minimum value
        let min_idx = final_amounts
            .iter()
            .enumerate()
            .min_by_key(|&(_, value)| value)
            .map(|(index, _)| index)
            .unwrap();

        final_amounts[min_idx] += 1;
        remaining -= 1;
    }

    final_amounts
}

#[cfg(test)]
mod tests_calculate_distribution {
    use super::*;

    #[test]
    fn test_calculate_distribution() {
        // Basic case
        assert_eq!(calculate_distribution(&[5, 2, 0], 3), vec![5, 3, 2]);

        // Empty resources
        assert_eq!(calculate_distribution(&[1, 1, 1], 0), vec![1, 1, 1]);

        // Equal initial values
        assert_eq!(calculate_distribution(&[2, 2, 2], 3), vec![3, 3, 3]);

        // Single element
        assert_eq!(calculate_distribution(&[0], 5), vec![5]);

        // Large gap between values
        assert_eq!(calculate_distribution(&[10, 0, 0], 6), vec![10, 3, 3]);

        // More resources than needed for perfect balance
        assert_eq!(calculate_distribution(&[3, 0], 10), vec![7, 6]);

        // All zeros initial
        assert_eq!(calculate_distribution(&[0, 0, 0, 0], 7), vec![2, 2, 2, 1]);

        // Different lengths
        assert_eq!(
            calculate_distribution(&[5, 4, 3, 2, 1], 5),
            vec![5, 4, 4, 4, 3]
        );
    }
}
