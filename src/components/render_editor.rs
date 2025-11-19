use std::ops::{Not, Range};

use itertools::Itertools;
use lazy_regex::Lazy;
use lsp_types::DiagnosticSeverity;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::Dimension,
    buffer::{Buffer, Line},
    char_index_range::CharIndexRange,
    components::{
        component::{Component, Cursor, SetCursorStyle},
        editor::Mode,
    },
    context::Context,
    divide_viewport::{calculate_window_position, divide_viewport},
    format_path_list::get_formatted_paths,
    git::hunk::SimpleHunk,
    grid::{CellUpdate, Grid, RenderContentLineNumber, StyleKey},
    position::Position,
    quickfix_list::QuickfixListItem,
    selection::{CharIndex, Selection},
    selection_mode::{self, ByteRange},
    soft_wrap::wrap_items,
    style::Style,
    themes::{Theme, UiStyles},
    utils::trim_array,
};

use super::{
    component::GetGridResult,
    editor::{Editor, RegexHighlightRule, RegexHighlightRuleCaptureStyle, Reveal, ViewAlignment},
};

static FOCUSED_TAB_REGEX: &Lazy<regex::Regex> =
    // We need multiline string so that wrapped filename will be highlighted as well
    lazy_regex::regex!("(?s)(?<focused_tab>\u{200B}(.*)\u{200B})");

pub(crate) fn markup_focused_tab(path: &str) -> String {
    format!("\u{200B}{path}\u{200B}")
}

impl Editor {
    pub(crate) fn get_grid(&mut self, context: &Context, focused: bool) -> GetGridResult {
        let hunks = self.buffer_mut().simple_hunks(context).unwrap_or_default();
        self.get_grid_with_scroll_offset(context, focused, self.scroll_offset(), &hunks)
    }
    pub(crate) fn get_grid_with_scroll_offset(
        &self,
        context: &Context,
        focused: bool,
        scroll_offset: u16,
        hunks: &[SimpleHunk],
    ) -> GetGridResult {
        let title = self.title(context);
        let title_grid_height = title.lines().count() as u16;
        let render_area = {
            let Dimension { height, width } = self.dimension();
            Dimension {
                height: height.saturating_sub(title_grid_height),
                width,
            }
        };
        let marks = context.get_marks(self.path());
        let grid = match &self.reveal {
            None => self.get_grid_with_dimension(
                context.theme(),
                context.current_working_directory(),
                context.quickfix_list_items(),
                render_area,
                scroll_offset,
                Some(self.selection_set.primary_selection().range()),
                false,
                true,
                focused,
                hunks,
                &marks,
            ),
            Some(reveal) => {
                self.get_splitted_grid(context, reveal, render_area, focused, hunks, &marks)
            }
        };
        let theme = context.theme();
        let window_title_style = if focused {
            theme.ui.window_title_focused
        } else {
            theme.ui.window_title_unfocused
        };

        let title_grid = {
            let mut editor = Editor::from_text(None, &title);
            editor.set_regex_highlight_rules(
                [RegexHighlightRule {
                    regex: (**FOCUSED_TAB_REGEX).clone(),
                    capture_styles: [RegexHighlightRuleCaptureStyle {
                        capture_name: "focused_tab",
                        source: Source::StyleKey(StyleKey::UiFocusedTab),
                    }]
                    .into_iter()
                    .collect(),
                }]
                .into_iter()
                .collect(),
            );
            let dimension = Dimension {
                height: title_grid_height,
                width: self.dimension().width,
            };
            let theme = Theme {
                ui: UiStyles {
                    background_color: window_title_style.background_color.unwrap_or_default(),
                    text_foreground: window_title_style.foreground_color.unwrap_or_default(),
                    ..theme.ui
                },
                ..context.theme().clone()
            };

            editor.get_grid_with_dimension(
                &theme,
                context.current_working_directory(),
                Vec::new(),
                dimension,
                0,
                None,
                false,
                false,
                focused,
                hunks,
                &[],
            )
        };
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

    pub(crate) fn title_impl(&self, context: &Context) -> Option<String> {
        let dimension = self.dimension();
        let result = if dimension.height <= 1 {
            vec![]
        } else {
            let wrapped_items = wrap_items(
                get_formatted_paths(
                    &context.get_marked_files(),
                    &self.path()?,
                    context.current_working_directory(),
                    self.buffer().dirty(),
                ),
                // Reference: NEED_TO_REDUCE_WIDTH_BY_1
                (dimension.width as usize).saturating_sub(1),
            );

            trim_array(
                &wrapped_items,
                wrapped_items.len().saturating_sub(1)..wrapped_items.len(),
                wrapped_items
                    .len()
                    .saturating_sub(dimension.height.saturating_sub(1).into()),
            )
            .trimmed_array
        };
        let result = result.join("\n");

        // Ensure that at least one height is spared for rendering the editor's content
        debug_assert!(result.lines().count() <= dimension.height.saturating_sub(1) as usize);
        Some(result)
    }

    fn get_splitted_grid(
        &self,
        context: &Context,
        reveal: &Reveal,
        render_area: Dimension,
        focused: bool,
        hunks: &[SimpleHunk],
        marks: &[CharIndexRange],
    ) -> crate::grid::Grid {
        let buffer = self.buffer();
        let ranges = match reveal {
            Reveal::CurrentSelectionMode => self
                .revealed_selections(
                    self.selection_set.primary_selection(),
                    context.current_working_directory(),
                    context.quickfix_list_items(),
                    marks,
                )
                .unwrap_or_default()
                .into_iter()
                .map(|byte_range| byte_range.range().clone())
                .collect_vec(),
            Reveal::Cursor => self
                .selection_set
                .map(|selection| selection.range())
                .into_iter()
                .filter_map(|range| buffer.char_index_range_to_byte_range(range).ok())
                .collect_vec(),
            Reveal::Mark => context
                .get_marks(buffer.path())
                .into_iter()
                .filter_map(|range| buffer.char_index_range_to_byte_range(range).ok())
                .collect_vec(),
        }
        .into_iter()
        .chain(
            // The primary selection should always be rendered as a section
            buffer
                .char_index_range_to_byte_range(self.selection_set.primary_selection().range())
                .ok(),
        )
        .sorted_by_key(|range| (range.start, range.end))
        .unique()
        .collect_vec();
        let viewport_sections = divide_viewport(
            &ranges,
            render_area.height as usize,
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
                let range = viewport_section.item();
                let protected_range_start = match self.cursor_direction {
                    super::editor::Direction::Start => range.start,
                    super::editor::Direction::End => range.end,
                };
                let line = buffer
                    .byte_to_line(protected_range_start)
                    .unwrap_or_default();

                let window = calculate_window_position(
                    line,
                    buffer.len_lines(),
                    viewport_section.height(),
                    self.current_view_alignment.unwrap_or(ViewAlignment::Center),
                );

                let protected_range = buffer
                    .byte_range_to_char_index_range(&range)
                    .unwrap_or_default();
                grid.merge_vertical(self.get_grid_with_dimension(
                    context.theme(),
                    context.current_working_directory(),
                    context.quickfix_list_items(),
                    Dimension {
                        height: viewport_section.height() as u16,
                        width: self.dimension().width,
                    },
                    window.start as u16,
                    Some(protected_range),
                    true,
                    true,
                    focused,
                    hunks,
                    marks,
                ))
            },
        )
    }

    /// Protected char index must not be trimmed and always be rendered.
    ///
    /// `borderize_first_line` should only be true when splitting.
    #[allow(clippy::too_many_arguments)]
    fn get_grid_with_dimension(
        &self,
        theme: &Theme,
        working_directory: &CanonicalizedPath,
        quickfix_list_items: Vec<&QuickfixListItem>,
        dimension: Dimension,
        scroll_offset: u16,
        protected_range: Option<CharIndexRange>,
        borderize_first_line: bool,
        render_line_number: bool,
        focused: bool,
        hunks: &[SimpleHunk],
        marks: &[CharIndexRange],
    ) -> Grid {
        let editor = self;
        let cursor_position = self.get_cursor_position().unwrap_or_default();
        let Dimension { height, width } = dimension;
        let buffer = editor.buffer();
        let rope = buffer.rope();
        let protected_char_index = protected_range
            .map(|protected_range| protected_range.as_char_index(&self.cursor_direction));
        let protected_range_start_line = protected_char_index.map(|protected_char_index| {
            buffer
                .char_to_line(protected_char_index)
                .unwrap_or_default()
        });

        let len_lines = rope.len_lines().max(1) as u16;
        let (hidden_parent_lines, visible_parent_lines) = self
            .get_parent_lines_given_line_index_and_scroll_offset(
                protected_range_start_line.unwrap_or_default(),
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

        let hidden_parent_line_ranges = hidden_parent_lines
            .iter()
            .map(|line| line.line..line.line + 1)
            .collect_vec();
        let visible_line_range =
            self.visible_line_range_given_scroll_offset_and_height(scroll_offset, height);
        let primary_cursor_char_index = primary_selection.to_char_index(&self.cursor_direction);
        let (hidden_parent_lines_grid, remaining_highlight_spans) = {
            let highlight_spans = self.get_highlight_spans(
                theme,
                working_directory,
                &visible_line_range,
                &hidden_parent_line_ranges,
                &visible_parent_lines,
                protected_range,
                quickfix_list_items,
                marks,
            );
            let boundaries = hidden_parent_line_ranges
                .into_iter()
                .map(|hidden_parent_line_range| Boundary::new(&buffer, hidden_parent_line_range))
                .collect_vec();
            let (updates, remaining_highlight_spans): (Vec<_>, Vec<_>) = hidden_parent_lines
                .iter()
                .filter_map(|line| {
                    if self.reveal.is_some() {
                        return None;
                    }
                    Some(HighlightSpan {
                        source: Source::StyleKey(StyleKey::ParentLine),
                        range: HighlightSpanRange::Line(line.line),
                        set_symbol: None,
                        is_cursor: false,
                        is_protected_range_start: false,
                    })
                })
                .chain(highlight_spans)
                .map(|span| span.into_cell_updates(&buffer, theme, &boundaries))
                .unzip();
            let updates = updates.into_iter().flatten().collect_vec();
            let remaining_highlight_spans = remaining_highlight_spans
                .into_iter()
                .flatten()
                .collect_vec();
            let grid = hidden_parent_lines.into_iter().fold(
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
                        if render_line_number {
                            RenderContentLineNumber::LineNumber {
                                start_line_index: line.line,
                                max_line_number: len_lines as usize,
                            }
                        } else {
                            RenderContentLineNumber::NoLineNumber
                        },
                        updates,
                        Default::default(),
                        theme,
                        None,
                        hunks,
                    ))
                },
            );
            (grid, remaining_highlight_spans)
        };

        debug_assert!(remaining_highlight_spans.is_empty().not());

        let grid = {
            let visible_lines_updates = {
                let boundaries = [Boundary::new(&buffer, visible_line_range)];
                remaining_highlight_spans
                    .into_iter()
                    .flat_map(|span| span.into_cell_updates(&buffer, theme, &boundaries).0)
                    // Insert the primary cursor cell update by force.
                    // This is necessary because when the content is empty
                    // all cell updates will be excluded when `to_cell_updates` is run,
                    // and then no cursor will be rendered for empty buffer.
                    .chain(protected_char_index.and_then(|protected_char_index| {
                        buffer
                            .char_to_position(protected_char_index)
                            .map(|position| {
                                CellUpdate::new(position)
                                    .set_is_protected_range_start(true)
                                    .set_is_cursor(
                                        primary_cursor_char_index == protected_char_index,
                                    )
                            })
                            .ok()
                    }))
                    .collect_vec()
            };
            let visible_lines_content = visible_lines.map(|(_, line)| line).join("");
            let visible_lines_grid = visible_lines_grid.render_content(
                &visible_lines_content,
                if render_line_number {
                    RenderContentLineNumber::LineNumber {
                        start_line_index: scroll_offset as usize,
                        max_line_number: len_lines as usize,
                    }
                } else {
                    RenderContentLineNumber::NoLineNumber
                },
                visible_lines_updates
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
                if focused
                    && protected_range
                        == Some(self.selection_set.primary_selection().extended_range())
                {
                    Some(
                        cursor_position
                            .set_line(cursor_position.line.saturating_sub(scroll_offset as usize)),
                    )
                } else {
                    None
                },
                hunks,
            );
            let protected_range = visible_lines_grid
                .get_protected_range_start_position()
                .map(|position| position.line..position.line + 1);

            let hidden_parent_lines_count = hidden_parent_lines_grid.rows.len();
            let max_hidden_parent_lines_count =
                hidden_parent_lines_count.min(visible_lines_grid.height() / 2);

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
                visible_lines_grid.height() == 0
                    || (clamped_hidden_parent_lines_grid.height() as f64)
                        / (visible_lines_grid.height() as f64)
                        <= 0.5
            );

            let result =
                clamped_hidden_parent_lines_grid.merge_vertical(trimmed_visible_lines_grid);

            let section_divider_cell_updates =
                if borderize_first_line && visible_lines_grid.height() > 1 {
                    {
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
                    }
                } else {
                    Default::default()
                };

            result.apply_cell_updates(section_divider_cell_updates)
        };

        debug_assert_eq!(grid.rows.len(), height as usize);
        debug_assert!(grid.rows.iter().all(|row| row.len() == width as usize));
        grid
    }

    #[allow(clippy::too_many_arguments)]
    fn get_highlight_spans(
        &self,
        theme: &Theme,
        working_directory: &CanonicalizedPath,
        visible_line_range: &Range<usize>,
        hidden_parent_line_ranges: &[Range<usize>],
        visible_parent_lines: &[Line],
        protected_range: Option<CharIndexRange>,
        quickfix_list_items: Vec<&QuickfixListItem>,
        marks: &[CharIndexRange],
    ) -> Vec<HighlightSpan> {
        use StyleKey::*;
        let buffer = self.buffer();

        let possible_selections = if self.incremental_search_matches.is_some() {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = HighlightSpan>>
        } else {
            Box::new(
                if self.selection_set.mode().is_contiguous() && self.reveal.is_none() {
                    Default::default()
                } else if self.reveal == Some(Reveal::CurrentSelectionMode) {
                    protected_range
                        .and_then(|protected_range| {
                            buffer.char_index_range_to_byte_range(protected_range).ok()
                        })
                        .into_iter()
                        .map(ByteRange::new)
                        .collect()
                } else {
                    self.possible_selections_in_line_number_range(
                        self.selection_set.primary_selection(),
                        working_directory,
                        visible_line_range,
                        quickfix_list_items,
                        marks,
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
                }),
            )
        };

        let incremental_search_matches = self
            .incremental_search_matches
            .as_ref()
            .map(|matches| {
                Box::new(matches.iter().map(|range| HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    range: HighlightSpanRange::ByteRange(range.clone()),
                    source: Source::StyleKey(UiIncrementalSearchMatch),
                    is_protected_range_start: false,
                })) as Box<dyn Iterator<Item = HighlightSpan>>
            })
            .unwrap_or_else(|| {
                Box::new(std::iter::empty()) as Box<dyn Iterator<Item = HighlightSpan>>
            });

        let marks_spans = marks
            .iter()
            .filter(|mark_range| {
                if let Some(Reveal::Mark) = self.reveal {
                    Some(*mark_range) == protected_range.as_ref()
                } else {
                    true
                }
            })
            .map(|mark| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                source: Source::StyleKey(UiMark),
                range: HighlightSpanRange::CharIndexRange(*mark),
                is_protected_range_start: false,
            });
        let secondary_selections = &self
            .selection_set
            .secondary_selections()
            .into_iter()
            .filter(|secondary_selection| {
                if let Some(Reveal::Cursor) = self.reveal {
                    Some(secondary_selection.range()) == protected_range
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
            match self.reveal {
                Some(Reveal::CurrentSelectionMode)
                    if protected_range != Some(primary_selection.extended_range()) =>
                {
                    no_primary_selection
                }
                Some(Reveal::Cursor)
                    if secondary_selections.iter().any(|secondary_selection| {
                        Some(secondary_selection.extended_range()) == protected_range
                    }) =>
                {
                    no_primary_selection
                }
                Some(Reveal::Mark)
                    if protected_range != Some(primary_selection.extended_range())
                        && marks
                            .iter()
                            .any(|mark| Some(mark) == protected_range.as_ref()) =>
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
                        source: Source::StyleKey(StyleKey::UiSecondarySelectionSecondaryCursor),
                        is_protected_range_start: false,
                    },
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        range: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&self.cursor_direction),
                        ),
                        source: Source::StyleKey(StyleKey::UiSecondarySelectionPrimaryCursor),
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
                set_symbol: Some(jump.character),
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
            .line_range_to_byte_range(visible_line_range)
            .unwrap_or_default();
        let spans = buffer.highlighted_spans();
        let filtered_highlighted_spans = {
            filter_items_by_range(
                spans,
                visible_line_byte_range.start,
                visible_line_byte_range.end,
                |span| span.byte_range.clone(),
            )
            .iter()
            .chain(hidden_parent_line_ranges.iter().flat_map(|line_range| {
                let byte_range = buffer
                    .line_range_to_byte_range(line_range)
                    .unwrap_or_default();
                filter_items_by_range(spans, byte_range.start, byte_range.end, |span| {
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

        let visible_parent_lines = if self.reveal.is_none() {
            Box::new(visible_parent_lines.iter().map(|line| HighlightSpan {
                source: Source::StyleKey(StyleKey::ParentLine),
                range: HighlightSpanRange::Line(line.line),
                set_symbol: None,
                is_cursor: false,
                is_protected_range_start: false,
            })) as Box<dyn Iterator<Item = HighlightSpan>>
        } else {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = HighlightSpan>>
        };

        // Note: the order here is important
        //   - First = lowest precedence
        //   - Last  = highest precedence
        //
        // Highlight spans with higher precedence will overwrite
        // highlight spans with lower precedence.
        vec![]
            .into_iter()
            .chain(visible_parent_lines)
            .chain(filtered_highlighted_spans)
            .chain(possible_selections)
            .chain(primary_selection_highlight_span)
            .chain(secondary_selections_highlight_spans)
            .chain(primary_selection_anchors)
            .chain(secondary_selection_anchors)
            .chain(marks_spans)
            .chain(diagnostics)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors)
            .chain(custom_regex_highlights)
            .chain(regex_highlight_rules)
            .chain(extra_decorations)
            .chain(incremental_search_matches)
            .collect_vec()
    }

    pub(crate) fn possible_selections_in_line_number_range(
        &self,
        selection: &Selection,
        working_directory: &CanonicalizedPath,
        line_number_range: &Range<usize>,
        quickfix_list_items: Vec<&QuickfixListItem>,
        marks: &[CharIndexRange],
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(
            selection,
            true,
            working_directory,
            quickfix_list_items,
            marks,
        )?;
        if self.selection_set.mode().is_contiguous() {
            return Ok(Vec::new());
        }

        object.selections_in_line_number_ranges(
            &selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            },
            [line_number_range.clone()].to_vec(),
        )
    }

    pub(crate) fn revealed_selections(
        &self,
        selection: &Selection,
        working_directory: &CanonicalizedPath,
        quickfix_list_items: Vec<&QuickfixListItem>,
        marks: &[CharIndexRange],
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.get_selection_mode_trait_object(
            selection,
            true,
            working_directory,
            quickfix_list_items,
            marks,
        )?
        .revealed_selections(&selection_mode::SelectionModeParams {
            buffer: &self.buffer(),
            current_selection: selection,
            cursor_direction: &self.cursor_direction,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct HighlightSpan {
    pub(crate) source: Source,
    pub(crate) range: HighlightSpanRange,
    pub(crate) set_symbol: Option<char>,
    pub(crate) is_cursor: bool,
    is_protected_range_start: bool,
}

impl HighlightSpan {
    /// Convert this `HighlightSpan` into `CellUpdate`s and remaining spans after processing a single boundary.
    ///
    /// Returns a tuple of `(Vec<CellUpdate>, Vec<HighlightSpan>)` where:
    /// - First element: `CellUpdate`s for the portion of this span that intersects with the boundary
    /// - Second element: Remaining `HighlightSpan`s for portions that don't intersect with the boundary
    ///
    /// The remaining spans represent the "leftover" parts after the intersection is converted to cell updates.
    /// This allows the span to be progressively "consumed" as it's processed through multiple boundaries.
    ///
    /// This method uses the boundary's native range type to minimize expensive range conversions.
    fn process_boundary(
        self,
        buffer: &Buffer,
        theme: &Theme,
        boundary: &Boundary,
    ) -> (Vec<CellUpdate>, Vec<HighlightSpan>) {
        // Find intersection using the appropriate range type to avoid expensive conversions
        let (intersection_range, remaining_ranges) = match &self.range {
            HighlightSpanRange::CharIndexRange(range) => {
                let intersection =
                    match range_intersection(&(range.start..range.end), &boundary.char_index_range)
                    {
                        Some(range) => range,
                        None => return (Vec::new(), vec![self]),
                    };

                let remaining = [
                    (range.start < intersection.start).then(|| {
                        HighlightSpanRange::CharIndexRange((range.start..intersection.start).into())
                    }),
                    (intersection.end < range.end).then(|| {
                        HighlightSpanRange::CharIndexRange((intersection.end..range.end).into())
                    }),
                ];

                (
                    HighlightSpanRange::CharIndexRange(intersection.into()),
                    remaining,
                )
            }

            HighlightSpanRange::ByteRange(range) => {
                let intersection = match range_intersection(range, &boundary.byte_range) {
                    Some(range) => range,
                    None => return (Vec::new(), vec![self]),
                };

                let remaining = [
                    (range.start < intersection.start)
                        .then(|| HighlightSpanRange::ByteRange(range.start..intersection.start)),
                    (intersection.end < range.end)
                        .then(|| HighlightSpanRange::ByteRange(intersection.end..range.end)),
                ];

                (HighlightSpanRange::ByteRange(intersection), remaining)
            }

            HighlightSpanRange::Line(line) => {
                let line_range = *line..line + 1;
                let intersection = match range_intersection(&line_range, &boundary.line_range) {
                    Some(range) => range,
                    None => return (Vec::new(), vec![self]),
                };

                let remaining = [
                    (line_range.start < intersection.start)
                        .then_some(HighlightSpanRange::Line(line_range.start))
                        .filter(|_| intersection.start > line_range.start),
                    (intersection.end < line_range.end)
                        .then_some(HighlightSpanRange::Line(intersection.end))
                        .filter(|_| intersection.end < line_range.end),
                ];

                // For single line, if there's intersection, convert the whole line
                (HighlightSpanRange::Line(*line), remaining)
            }

            HighlightSpanRange::CharIndex(char_index) => {
                let char_range = *char_index..CharIndex(char_index.0 + 1);

                if range_intersection(&char_range, &boundary.char_index_range).is_none() {
                    return (Vec::new(), vec![self]);
                }

                // Single char index - either fully contained or not
                (HighlightSpanRange::CharIndex(*char_index), [None, None])
            }
        };

        // Generate cell updates for intersected portion
        let cell_updates = match &intersection_range {
            HighlightSpanRange::CharIndexRange(range) => range
                .iter()
                .filter_map(|char_index| {
                    let position = buffer.char_to_position(char_index).ok()?;
                    Some(self.create_cell_update(position, theme))
                })
                .collect(),
            HighlightSpanRange::ByteRange(range) => {
                // Convert to char range only for the intersected portion
                buffer
                    .byte_range_to_char_index_range(range)
                    .map(|char_range| {
                        (char_range.start.0..char_range.end.0)
                            .filter_map(|char_index| {
                                let position =
                                    buffer.char_to_position(CharIndex(char_index)).ok()?;
                                Some(self.create_cell_update(position, theme))
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            HighlightSpanRange::Line(line) => buffer
                .line_range_to_char_index_range(&(*line..line + 1))
                .map(|char_range| {
                    (char_range.start.0..char_range.end.0)
                        .filter_map(|char_index| {
                            let position = buffer.char_to_position(CharIndex(char_index)).ok()?;
                            Some(self.create_cell_update(position, theme))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            HighlightSpanRange::CharIndex(char_index) => buffer
                .char_to_position(*char_index)
                .map(|position| vec![self.create_cell_update(position, theme)])
                .unwrap_or_default(),
        };

        // Generate remaining spans
        let remaining_spans = remaining_ranges
            .into_iter()
            .flatten()
            .map(|range| HighlightSpan {
                range,
                ..self.clone()
            })
            .collect();

        (cell_updates, remaining_spans)
    }

    // Helper method to create CellUpdate
    fn create_cell_update(&self, position: Position, theme: &Theme) -> CellUpdate {
        CellUpdate {
            position,
            symbol: self.set_symbol,
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
        }
    }

    /// Convert this `HighlightSpan` into `CellUpdate`s by processing it through multiple boundaries.
    ///
    /// Returns a tuple of `(Vec<CellUpdate>, Vec<HighlightSpan>)` where:
    /// - First element: All `CellUpdate`s generated from portions of this span that intersected with any boundary
    /// - Second element: Remaining `HighlightSpan`s that didn't intersect with any boundary
    ///
    /// The span is progressively processed through each boundary, with intersecting portions converted
    /// to `CellUpdate`s and non-intersecting portions carried forward to the next boundary.
    /// Any portions that don't intersect with any boundary are returned as remaining spans.
    ///
    /// This design minimizes expensive `buffer.xxx_to_position` calls by only converting portions
    /// that actually fall within the given boundaries, and avoids expensive range conversions by
    /// using each boundary's native range type.
    fn into_cell_updates(
        self,
        buffer: &Buffer,
        theme: &Theme,
        boundaries: &[Boundary],
    ) -> (Vec<CellUpdate>, Vec<HighlightSpan>) {
        boundaries.iter().fold(
            (Vec::new(), vec![self]),
            |(all_cell_updates, spans), boundary| {
                let (new_cell_updates, remaining_spans): (Vec<_>, Vec<_>) = spans
                    .into_iter()
                    .map(|span| span.process_boundary(buffer, theme, boundary))
                    .unzip();

                (
                    all_cell_updates
                        .into_iter()
                        .chain(new_cell_updates.into_iter().flatten())
                        .collect_vec(),
                    remaining_spans.into_iter().flatten().collect(),
                )
            },
        )
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

#[derive(Debug)]
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
                height: ((u8::arbitrary(g) / 10) as u16).max(1),
            }
        }
    }

    #[quickcheck]
    fn get_grid_cells_should_be_always_within_bound(rectangle: Rectangle, content: String) -> bool {
        let content = content.replace("\r", "");
        let mut editor = Editor::from_text(None, &content);
        editor.set_rectangle(rectangle.clone(), &Context::default());
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
