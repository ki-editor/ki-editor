use std::ops::Range;

use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::Dimension,
    buffer::{Buffer, Line},
    char_index_range::CharIndexRange,
    components::{
        component::{Component, Cursor, SetCursorStyle},
        editor::{Mode, ViewAlignment, WINDOW_TITLE_HEIGHT},
    },
    context::Context,
    divide_viewport::{divide_viewport, divide_viewport_new},
    grid::{CellUpdate, Grid, LineUpdate, RenderContentLineNumber, StyleKey},
    position::Position,
    selection::{CharIndex, Selection},
    selection_mode::{self, ByteRange},
    style::Style,
    themes::Theme,
    utils::{get_non_consecutive_nums, trim_array},
};

use super::{
    component::GetGridResult,
    editor::{Editor, Fold},
};

impl Editor {
    pub(crate) fn get_grid(&self, context: &Context, focused: bool) -> GetGridResult {
        let buffer = self.buffer();
        let grid = match &self.fold {
            None => {
                let cursor_char_index = self
                    .selection_set
                    .primary_selection()
                    .to_char_index(&self.cursor_direction);
                self.get_grid_with_dimension(
                    context,
                    self.render_area(),
                    self.scroll_offset(),
                    (cursor_char_index..cursor_char_index + 1).into(),
                    false,
                )
            }
            Some(fold) => {
                let ranges = match fold {
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
                        .filter_map(|range| buffer.char_index_range_to_byte_range(range).ok())
                        .collect_vec(),
                }
                .into_iter()
                .chain(
                    buffer
                        .char_index_range_to_byte_range(
                            self.selection_set.primary_selection().extended_range(),
                        )
                        .ok(),
                )
                .sorted_by_key(|range| (range.start, range.end))
                .unique()
                .collect_vec();
                let line_number_and_ranges: Vec<(usize, Range<usize>)> = ranges
                    .clone()
                    .into_iter()
                    .filter_map(|byte_range| {
                        Some((buffer.byte_to_line(byte_range.start).ok()?, byte_range))
                    })
                    .sorted_by_key(|(_, byte_range)| byte_range.start)
                    .collect_vec();
                let line_numbers = line_number_and_ranges
                    .iter()
                    .map(|(line_number, _)| *line_number)
                    .unique()
                    .collect_vec();
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
                    self.render_area().height as usize,
                    buffer.len_lines().saturating_sub(1),
                    self.current_view_alignment.unwrap_or(ViewAlignment::Center),
                );

                let viewport_sections = divide_viewport_new(
                    &ranges,
                    self.render_area().height as usize,
                    buffer
                        .char_index_range_to_byte_range(
                            self.selection_set.primary_selection().extended_range(),
                        )
                        .unwrap_or_default(),
                );

                viewport_sections.into_iter().fold(
                    Grid::new(Dimension {
                        height: 0,
                        width: self.dimension().width,
                    }),
                    |grid, viewport_section| {
                        let range = viewport_section.byte_range();
                        let protected_range_start = match self.cursor_direction {
                            super::editor::Direction::Start => range.start,
                            super::editor::Direction::End => range.end,
                        };
                        let line = buffer
                            .byte_to_line(protected_range_start)
                            .unwrap_or_default();

                        let scroll_offset = line.saturating_sub(
                            (viewport_section.height() as f64 / 2 as f64).floor() as usize,
                        ) as u16;

                        let protected_range = buffer
                            .byte_range_to_char_index_range(&range)
                            .unwrap_or_default();
                        grid.merge_vertical(self.get_grid_with_dimension(
                            context,
                            Dimension {
                                height: viewport_section.height() as u16,
                                width: self.render_area().width,
                            },
                            scroll_offset,
                            protected_range,
                            true,
                        ))
                    },
                )
            }
        };
        let theme = context.theme();
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
            width: self.dimension().width,
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

    /// Protected char index must not be trimmed and always be rendered
    ///
    /// `borderize_first_line` should only be true when splitting.
    fn get_grid_with_dimension(
        &self,
        context: &Context,
        dimension: Dimension,
        scroll_offset: u16,
        protected_range: CharIndexRange,
        borderize_first_line: bool,
    ) -> Grid {
        let editor = self;
        let Dimension { height, width } = dimension;
        let buffer = editor.buffer();
        let rope = buffer.rope();
        let protected_char_index = protected_range.to_char_index(&self.cursor_direction);
        let protected_range_start_line = buffer
            .char_to_line(protected_char_index)
            .unwrap_or_default();

        let len_lines = rope.len_lines().max(1) as u16;
        let (hidden_parent_lines, visible_parent_lines) = self
            .get_parent_lines_given_line_index_and_scroll_offset(
                protected_range_start_line,
                scroll_offset,
            )
            .unwrap_or_default();
        let visible_lines = rope
            .lines()
            .enumerate()
            .skip(scroll_offset as usize)
            .take(height as usize)
            .map(|(line_index, slice)| (line_index, slice.to_string()));

        let visible_lines_grid: Grid = Grid::new(Dimension { height, width });

        let primary_selection = &editor.selection_set.primary_selection();

        // If the buffer selection is updated less recently than the window's scroll offset,
        // use the window's scroll offset.

        let theme = context.theme();

        let hidden_parent_line_ranges = hidden_parent_lines
            .iter()
            .map(|line| line.line..line.line + 1)
            .collect_vec();
        let visible_line_range =
            self.visible_line_range_given_scroll_offset_and_height(scroll_offset, height);
        let primary_cursor_char_index = primary_selection.to_char_index(&self.cursor_direction);
        let updates = self.get_highlight_spans(
            context,
            &visible_line_range,
            &hidden_parent_line_ranges,
            &visible_parent_lines,
            protected_range,
        );

        let hidden_parent_lines_grid = {
            let boundaries = hidden_parent_line_ranges
                .into_iter()
                .map(|folded_line_range| Boundary::new(&buffer, folded_line_range))
                .collect_vec();
            let updates = hidden_parent_lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    let char_index_start = buffer.position_to_char(line.origin_position).ok()?;
                    let char_index_end = buffer
                        .position_to_char(Position {
                            line: line.line,
                            column: line.origin_position.column + line.content.chars().count(),
                        })
                        .ok()?;
                    Some(HighlightSpan {
                        source: Source::StyleKey(StyleKey::ParentLine),
                        range: HighlightSpanRange::Line(line.line),
                        set_symbol: None,
                        is_cursor: false,
                        is_protected_range_start: false,
                    })
                })
                .chain(updates.clone())
                .flat_map(|span| span.to_cell_updates(&buffer, theme, &boundaries))
                .collect_vec();
            hidden_parent_lines.into_iter().fold(
                Grid::new(Dimension { height: 0, width }),
                |grid, line| {
                    let updates = updates
                        .iter()
                        .filter_map(|update| {
                            if update.position.line == line.line {
                                Some(update.clone().set_position_line(0))
                            } else {
                                None
                            }
                        })
                        .collect_vec();
                    grid.merge_vertical(Grid::new(Dimension { height: 1, width }).render_content(
                        &line.content,
                        RenderContentLineNumber::LineNumber {
                            start_line_index: line.line,
                            max_line_number: len_lines as usize,
                        },
                        updates,
                        Default::default(),
                        theme,
                    ))
                },
            )
        };

        let grid = {
            let visible_lines_updates = {
                let boundaries = [Boundary::new(&buffer, visible_line_range)];
                updates
                    .iter()
                    .flat_map(|span| span.to_cell_updates(&buffer, theme, &boundaries))
                    // Insert the primary cursor cell update by force.
                    // This is necessary because when the content is empty
                    // all cell updates will be excluded when `to_cell_updates` is run,
                    // and then no cursor will be rendered for empty buffer.
                    .chain(
                        buffer
                            .char_to_position(protected_char_index)
                            .map(|position| {
                                CellUpdate::new(position)
                                    .set_is_protected_range_start(true)
                                    .set_is_cursor(
                                        primary_cursor_char_index == protected_char_index,
                                    )
                            }),
                    )
                    .collect_vec()
            };
            let visible_lines_content = visible_lines.map(|(_, line)| line).join("");
            let visible_lines_grid = visible_lines_grid.render_content(
                &visible_lines_content,
                RenderContentLineNumber::LineNumber {
                    start_line_index: scroll_offset as usize,
                    max_line_number: len_lines as usize,
                },
                visible_lines_updates
                    .clone()
                    .into_iter()
                    .filter_map(|cell_update| {
                        Some(CellUpdate {
                            position: cell_update.position.move_up(scroll_offset as usize)?,
                            ..cell_update
                        })
                    })
                    .collect_vec(),
                Default::default(),
                theme,
            );
            let protected_range = visible_lines_grid
                .get_protected_range_start_position()
                .map(|position| position.line..position.line + 1);

            let hidden_parent_lines_count = hidden_parent_lines_grid.rows.len();
            let max_hidden_parent_lines_count =
                hidden_parent_lines_count.min(visible_lines_grid.rows.len() / 2);

            let trim_result = trim_array(
                &visible_lines_grid.rows,
                protected_range.unwrap_or_default(),
                max_hidden_parent_lines_count,
            );
            let clamped_hidden_parent_lines_grid = hidden_parent_lines_grid.clamp_bottom(
                (trim_result.remaining_trim_count
                    + (hidden_parent_lines_count - max_hidden_parent_lines_count))
                    as u16,
            );
            let trimmed_visible_lines_grid = Grid {
                width: visible_lines_grid.width,
                rows: trim_result.trimmed_array,
            };

            // Verify that the maximum number of hidden parent lines only take
            // at most 50% of the render area (less one row of title)
            debug_assert!(
                (clamped_hidden_parent_lines_grid.height() as f64)
                    / (visible_lines_grid.height() as f64)
                    <= 0.5
            );

            let result =
                clamped_hidden_parent_lines_grid.merge_vertical(trimmed_visible_lines_grid);

            let section_divider_cell_updates = (borderize_first_line
                && visible_lines_grid.height() > 1)
                .then(|| {
                    (0..width)
                        .map(|column| CellUpdate {
                            position: Position::new(0, column as usize),
                            symbol: None,
                            style: Style::new()
                                .background_color(theme.ui.section_divider_background),
                            is_cursor: false,
                            is_protected_range_start: false,
                            source: Some(StyleKey::UiSectionDivider),
                        })
                        .collect()
                })
                .unwrap_or_default();

            result.apply_cell_updates(section_divider_cell_updates)
        };

        debug_assert_eq!(grid.rows.len(), height as usize);
        debug_assert!(grid.rows.iter().all(|row| row.len() == width as usize));
        grid
    }

    fn get_highlight_spans(
        &self,
        context: &Context,
        visible_line_range: &Range<usize>,
        hidden_parent_line_ranges: &Vec<Range<usize>>,
        visible_parent_lines: &Vec<Line>,
        protected_range: CharIndexRange,
    ) -> Vec<HighlightSpan> {
        use StyleKey::*;
        let theme = context.theme();
        let buffer = self.buffer();
        let possible_selections = if self.selection_set.mode.is_contiguous() {
            Default::default()
        } else if self.fold == Some(Fold::CurrentSelectionMode) {
            buffer
                .char_index_range_to_byte_range(protected_range)
                .ok()
                .into_iter()
                .map(|byte_range| ByteRange::new(byte_range))
                .collect()
        } else {
            self
                //.possible_selections(self.selection_set.primary_selection(), context)
                .possible_selections_in_line_number_range(
                    self.selection_set.primary_selection(),
                    context,
                    visible_line_range,
                )
                .unwrap_or_default()
        }
        .into_iter()
        .map(|range| HighlightSpan {
            set_symbol: None,
            is_cursor: false,
            range: HighlightSpanRange::ByteRange(range.range().clone()),
            source: Source::StyleKey(UiPossibleSelection),
            is_protected_range_start: false,
        });

        let marks = buffer
            .marks()
            .into_iter()
            .filter(|mark| {
                if let Some(Fold::Mark) = self.fold {
                    mark == &protected_range
                } else {
                    true
                }
            })
            .map(|mark| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                source: Source::StyleKey(UiMark),
                range: HighlightSpanRange::CharIndexRange(mark),
                is_protected_range_start: false,
            });
        let secondary_selections = &self
            .selection_set
            .secondary_selections()
            .into_iter()
            .filter(|secondary_selection| {
                if let Some(Fold::Cursor) = self.fold {
                    secondary_selection.extended_range() == protected_range
                } else {
                    true
                }
            })
            .collect_vec();

        let primary_selection = &self.selection_set.primary_selection();

        let (
            primary_selection_highlight_span,
            primary_selection_anchors,
            primary_selection_secondary_cursor,
        ) = {
            let no_primary_selection = (
                None,
                Box::new(std::iter::empty()) as Box<dyn Iterator<Item = HighlightSpan>>,
                None,
            );
            match self.fold {
                Some(Fold::CurrentSelectionMode)
                    if protected_range != primary_selection.extended_range() =>
                {
                    no_primary_selection
                }
                Some(Fold::Cursor)
                    if secondary_selections.iter().any(|secondary_selection| {
                        secondary_selection.extended_range() == protected_range
                    }) =>
                {
                    no_primary_selection
                }
                Some(Fold::Mark)
                    if protected_range != primary_selection.extended_range()
                        && buffer.marks().iter().any(|mark| mark == &protected_range) =>
                {
                    no_primary_selection
                }
                _ => {
                    let primary_selection_highlight_span = HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndexRange(
                            primary_selection.extended_range(),
                        ),
                        source: Source::StyleKey(UiPrimarySelection),
                        is_protected_range_start: false,
                    };
                    let primary_selection_anchors =
                        primary_selection
                            .anchors()
                            .into_iter()
                            .map(|anchor| HighlightSpan {
                                set_symbol: None,
                                is_cursor: false,
                                range: HighlightSpanRange::CharIndexRange(anchor),
                                source: Source::StyleKey(UiPrimarySelectionAnchors),
                                is_protected_range_start: false,
                            });
                    let primary_selection_secondary_cursor = if self.mode == Mode::Insert {
                        None
                    } else {
                        Some(HighlightSpan {
                            set_symbol: None,
                            is_cursor: false,
                            range: HighlightSpanRange::CharIndex(
                                primary_selection.to_char_index(&self.cursor_direction.reverse()),
                            ),
                            source: Source::StyleKey(StyleKey::UiPrimarySelectionSecondaryCursor),
                            is_protected_range_start: false,
                        })
                    };
                    (
                        Some(primary_selection_highlight_span),
                        Box::new(primary_selection_anchors)
                            as Box<dyn Iterator<Item = HighlightSpan>>,
                        primary_selection_secondary_cursor,
                    )
                }
            }
        };

        let secondary_selections_highlight_spans =
            secondary_selections
                .iter()
                .map(|secondary_selection| HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    range: HighlightSpanRange::CharIndexRange(secondary_selection.extended_range()),
                    source: Source::StyleKey(UiSecondarySelection),
                    is_protected_range_start: false,
                });

        let secondary_selection_anchors = secondary_selections.iter().flat_map(|selection| {
            selection.anchors().into_iter().map(|anchor| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                range: HighlightSpanRange::CharIndexRange(anchor),
                source: Source::StyleKey(UiSecondarySelectionAnchors),
                is_protected_range_start: false,
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
                        source: Source::StyleKey(StyleKey::SecondarySelectionSecondaryCursor),
                        is_protected_range_start: false,
                    },
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&self.cursor_direction),
                        ),
                        source: Source::StyleKey(StyleKey::SecondarySelectionPrimaryCursor),
                        is_protected_range_start: false,
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
                is_protected_range_start: false,
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
                is_protected_range_start: false,
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
                is_protected_range_start: false,
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
            .chain(hidden_parent_line_ranges.iter().flat_map(|line_range| {
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
                is_protected_range_start: false,
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
                    is_protected_range_start: false,
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
                        is_protected_range_start: false,
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
            is_protected_range_start: false,
        });
        vec![]
            .into_iter()
            .chain(visible_parent_lines)
            .chain(filtered_highlighted_spans)
            .chain(extra_decorations)
            .chain(possible_selections)
            .chain(primary_selection_highlight_span)
            .chain(secondary_selections_highlight_spans)
            .chain(primary_selection_anchors)
            .chain(secondary_selection_anchors)
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
        line_number_range: &Range<usize>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(selection, true, context)?;
        if self.selection_set.mode.is_contiguous() {
            return Ok(Vec::new());
        }

        object.selections_in_line_number_range(
            &selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            },
            [line_number_range.clone()].to_vec(),
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
    is_protected_range_start: bool,
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
                                is_protected_range_start: self.is_protected_range_start,
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
