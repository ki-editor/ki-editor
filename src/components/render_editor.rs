use std::ops::Range;

use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::Dimension,
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        component::{Component, Cursor, SetCursorStyle},
        editor::Mode,
    },
    context::Context,
    grid::{CellUpdate, Grid, StyleKey},
    selection::{CharIndex, Selection},
    selection_mode::{self, ByteRange},
    soft_wrap,
    style::Style,
    themes::Theme,
};

use super::{component::GetGridResult, editor::Editor};

use StyleKey::*;

impl Editor {
    pub fn get_grid(&self, context: &Context) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.render_area();
        let buffer = editor.buffer();
        let rope = buffer.rope();
        let content = rope.to_string();

        let diagnostics = buffer.diagnostics();

        let len_lines = rope.len_lines().max(1) as u16;
        let max_line_number_len = len_lines.to_string().len() as u16;
        let line_number_separator_width = 1;
        let (hidden_parent_lines, visible_parent_lines) =
            self.get_parent_lines().unwrap_or_default();

        let top_offset = hidden_parent_lines.len() as u16;

        let scroll_offset = self.scroll_offset();

        let visible_lines = &rope
            .lines()
            .skip(scroll_offset as usize)
            .take(height as usize)
            .map(|slice| slice.to_string())
            .collect_vec();

        let content_container_width = (width
            .saturating_sub(max_line_number_len)
            .saturating_sub(line_number_separator_width))
            as usize;

        let wrapped_lines = soft_wrap::soft_wrap(&visible_lines.join(""), content_container_width);

        let parent_lines_numbers = visible_parent_lines
            .iter()
            .chain(hidden_parent_lines.iter())
            .map(|line| line.line)
            .collect_vec();

        let visible_lines_grid: Grid = Grid::new(Dimension {
            height: (height as usize).max(wrapped_lines.wrapped_lines_count()) as u16,
            width,
        });

        let selection = &editor.selection_set.primary;
        // If the buffer selection is updated less recently than the window's scroll offset,

        // use the window's scroll offset.

        let lines = wrapped_lines
            .lines()
            .iter()
            .flat_map(|line| {
                let line_number = line.line_number();
                line.lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| RenderLine {
                        line_number: line_number + (scroll_offset as usize),
                        content: line,
                        wrapped: index > 0,
                    })
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        let theme = context.theme();

        let possible_selections = self
            .possible_selections_in_line_number_range(&self.selection_set.primary)
            .unwrap_or_default()
            .into_iter()
            .map(|range| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                ranges: HighlightSpanRange::ByteRange(range.range().clone()),
                source: Source::StyleKey(UiPossibleSelection),
            })
            .collect_vec();

        let bookmarks = buffer
            .bookmarks()
            .into_iter()
            .map(|bookmark| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                source: Source::StyleKey(UiBookmark),
                ranges: HighlightSpanRange::CharIndexRange(bookmark),
            })
            .collect_vec();
        let secondary_selections = &editor.selection_set.secondary;
        let primary_selection = HighlightSpan {
            set_symbol: None,
            is_cursor: false,
            ranges: HighlightSpanRange::CharIndexRange(selection.extended_range()),
            source: Source::StyleKey(UiPrimarySelection),
        };

        let primary_selection_anchors = selection
            .anchors()
            .into_iter()
            .map(|anchor| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                ranges: HighlightSpanRange::CharIndexRange(anchor),
                source: Source::StyleKey(UiPrimarySelectionAnchors),
            })
            .collect_vec();
        let primary_selection_primary_cursor = buffer
            .char_to_position(selection.to_char_index(&editor.cursor_direction))
            .ok()
            .map(|position| CellUpdate::new(position).set_is_cursor(true));

        let primary_selection_secondary_cursor = if self.mode == Mode::Insert {
            None
        } else {
            Some(HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                ranges: HighlightSpanRange::CharIndex(
                    selection.to_char_index(&editor.cursor_direction.reverse()),
                ),
                source: Source::Style(theme.ui.primary_selection_secondary_cursor),
            })
        };

        let secondary_selection = secondary_selections
            .iter()
            .map(|secondary_selection| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                ranges: HighlightSpanRange::CharIndexRange(secondary_selection.extended_range()),
                source: Source::StyleKey(UiSecondarySelection),
            })
            .collect_vec();

        let seconday_selection_anchors = secondary_selections
            .iter()
            .flat_map(|selection| {
                selection.anchors().into_iter().map(|anchor| HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    ranges: HighlightSpanRange::CharIndexRange(anchor),
                    source: Source::StyleKey(UiSecondarySelectionAnchors),
                })
            })
            .collect_vec();
        let secondary_selection_cursors =
            secondary_selections.iter().flat_map(|secondary_selection| {
                [
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        ranges: HighlightSpanRange::CharIndex(
                            secondary_selection.to_char_index(&editor.cursor_direction.reverse()),
                        ),
                        source: Source::Style(theme.ui.secondary_selection_secondary_cursor),
                    },
                    HighlightSpan {
                        set_symbol: None,
                        is_cursor: false,
                        ranges: HighlightSpanRange::CharIndex(
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
                ranges: HighlightSpanRange::CharIndexRange(diagnostic.range),
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
                ranges: HighlightSpanRange::CharIndex(
                    jump.selection.to_char_index(&self.cursor_direction),
                ),
            }
        });
        let extra_decorations = buffer
            .decorations()
            .iter()
            .flat_map(|decoration| {
                Some(HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    ranges: HighlightSpanRange::CharIndexRange(
                        decoration
                            .selection_range()
                            .to_char_index_range(&buffer)
                            .ok()?,
                    ),
                    source: Source::StyleKey(decoration.style_key().clone()),
                })
            })
            .collect_vec();
        let highlighted_spans = buffer
            .highlighted_spans()
            .into_iter()
            .map(|highlighted_span| HighlightSpan {
                set_symbol: None,
                is_cursor: false,
                ranges: HighlightSpanRange::ByteRange(highlighted_span.byte_range),
                source: Source::StyleKey(highlighted_span.style_key),
            })
            .collect_vec();
        let custom_regex_highlights = lazy_regex::regex!("#[0-9a-f]{6}")
            .find_iter(&rope.to_string())
            .map(|m| (m.as_str().to_string(), m.range()))
            .filter_map(|(hex, range)| {
                let color = crate::themes::Color::from_hex(hex.clone()).ok()?;
                Some(HighlightSpan {
                    set_symbol: None,
                    is_cursor: false,
                    ranges: HighlightSpanRange::ByteRange(range),
                    source: Source::Style(
                        Style::new()
                            .background_color(color)
                            .foreground_color(color.get_contrasting_color()),
                    ),
                })
            })
            .collect_vec();

        let regex_highlight_rules = self
            .regex_highlight_rules
            .iter()
            .filter_map(|rule| {
                let captures = rule.regex.captures(&content)?;
                let get_highlight_span = |name: &'static str, source: Source| {
                    let match_ = captures.name(name)?;
                    Some(HighlightSpan {
                        source,
                        ranges: HighlightSpanRange::ByteRange(match_.range()),
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
            .flatten()
            .collect_vec();

        let boundaries = {
            let line_indices = hidden_parent_lines.iter().map(|line| line.line);
            let hidden_parent_line_range = line_indices.clone().min().unwrap_or_default()
                ..line_indices.max().unwrap_or_default() + 1;
            &[
                Boundary::new(&buffer, self.visible_line_range()),
                Boundary::new(&buffer, hidden_parent_line_range),
            ]
        };
        let updates = vec![]
            .into_iter()
            .chain(highlighted_spans)
            .chain(extra_decorations)
            .chain(possible_selections)
            .chain(Some(primary_selection))
            .chain(secondary_selection)
            .chain(primary_selection_anchors)
            .chain(seconday_selection_anchors)
            .chain(bookmarks)
            .chain(diagnostics)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors)
            .chain(custom_regex_highlights)
            .chain(regex_highlight_rules)
            .flat_map(|span| span.to_cell_update(&buffer, theme, boundaries))
            .chain(primary_selection_primary_cursor)
            .collect_vec();

        #[derive(Debug, Clone)]
        struct RenderLine {
            line_number: usize,
            content: String,
            wrapped: bool,
        }

        let render_lines = |grid: Grid, lines: Vec<RenderLine>| {
            lines.into_iter().enumerate().fold(
                grid,
                |grid,
                 (
                    line_index,
                    RenderLine {
                        line_number,
                        content: line,
                        wrapped,
                    },
                )| {
                    let background_color = if parent_lines_numbers.iter().contains(&line_number) {
                        Some(theme.ui.parent_lines_background)
                    } else {
                        None
                    };
                    let line_number_str = {
                        let line_number = if wrapped {
                            "↪".to_string()
                        } else {
                            (line_number + 1).to_string()
                        };
                        format!(
                            "{: >width$}",
                            line_number.to_string(),
                            width = max_line_number_len as usize
                        )
                    };
                    Grid::new(Dimension {
                        height,
                        width: max_line_number_len,
                    });
                    grid.set_row(
                        line_index,
                        Some(0),
                        Some(max_line_number_len as usize),
                        &line_number_str,
                        &theme
                            .ui
                            .line_number
                            .set_some_background_color(background_color),
                    )
                    .set_row(
                        line_index,
                        Some(max_line_number_len as usize),
                        Some((max_line_number_len + 1) as usize),
                        "│",
                        &theme
                            .ui
                            .line_number_separator
                            .set_some_background_color(background_color),
                    )
                    .set_row(
                        line_index,
                        Some((max_line_number_len + 1) as usize),
                        None,
                        &line.chars().take(width as usize).collect::<String>(),
                        &theme.ui.text.set_some_background_color(background_color),
                    )
                },
            )
        };
        let visible_lines_updates = updates
            .clone()
            .into_iter()
            .filter_map(|update| {
                let update = update.move_up((scroll_offset).into())?;

                let position = wrapped_lines.calibrate(update.position).ok()?;

                let position =
                    position.move_right(max_line_number_len + line_number_separator_width);

                Some(CellUpdate { position, ..update })
            })
            .collect::<Vec<_>>();
        let visible_render_lines = if lines.is_empty() {
            [RenderLine {
                line_number: 0,
                content: String::new(),
                wrapped: false,
            }]
            .to_vec()
        } else {
            lines
        };

        let visible_lines_grid = render_lines(visible_lines_grid, visible_render_lines)
            .apply_cell_updates(visible_lines_updates);

        let (hidden_parent_lines_grid, hidden_parent_lines_updates) =
            {
                let height = hidden_parent_lines.len() as u16;
                let hidden_parent_lines = hidden_parent_lines
                    .iter()
                    .map(|line| RenderLine {
                        line_number: line.line,
                        content: line.content.clone(),
                        wrapped: false,
                    })
                    .collect_vec();
                let updates =
                    {
                        let hidden_parent_lines_with_index =
                            hidden_parent_lines.iter().enumerate().collect_vec();
                        updates
                            .iter()
                            .filter_map(|update| {
                                if let Some((index, _)) = hidden_parent_lines_with_index
                                    .iter()
                                    .find(|(_, line)| update.position.line == line.line_number)
                                {
                                    Some(update.clone().set_position_line(*index).move_right(
                                        max_line_number_len + line_number_separator_width,
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect_vec()
                    };

                let grid = render_lines(
                    Grid::new(Dimension {
                        width: editor.dimension().width,
                        height,
                    }),
                    hidden_parent_lines,
                );
                (grid, updates)
            };
        let hidden_parent_lines_grid =
            hidden_parent_lines_grid.apply_cell_updates(hidden_parent_lines_updates);

        let cursor_beyond_view_bottom =
            if let Some(cursor_position) = visible_lines_grid.get_cursor_position() {
                cursor_position
                    .line
                    .saturating_sub(height.saturating_sub(1).saturating_sub(top_offset) as usize)
            } else {
                0
            };
        let grid = {
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
        let window_title_style = theme.ui.window_title;

        // NOTE: due to performance issue, we only highlight the content that are within view
        // This might result in some incorrectness, but that's a reasonable trade-off, because
        // highlighting the entire file becomes sluggish when the file has more than a thousand lines.

        let title_grid = Grid::new(Dimension {
            height: 1,
            width: editor.dimension().width,
        })
        .set_line(0, &self.title(context), &window_title_style);

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

    pub fn possible_selections_in_line_number_range(
        &self,
        selection: &Selection,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(selection)?;
        if self.selection_set.mode.is_contiguous() && self.selection_set.filters.is_empty() {
            return Ok(Vec::new());
        }

        let line_range = self.visible_line_range();
        object.selections_in_line_number_range(
            &selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
                filters: &self.selection_set.filters,
            },
            line_range,
        )
    }
}

pub struct HighlightSpan {
    pub source: Source,
    pub ranges: HighlightSpanRange,
    pub set_symbol: Option<String>,
    pub is_cursor: bool,
}

impl HighlightSpan {
    /// Convert this `HighlightSpans` into `Vec<CellUpdate>`,
    /// only perform conversions for positions that falls within the given `boundaries`,
    /// so that we can minimize the call to the expensive `buffer.xxx_to_position` methods
    fn to_cell_update(
        &self,
        buffer: &Buffer,
        theme: &Theme,
        boundaries: &[Boundary],
    ) -> Vec<CellUpdate> {
        boundaries
            .iter()
            .filter_map(|boundary| {
                let char_index_range: CharIndexRange = match &self.ranges {
                    HighlightSpanRange::CharIndexRange(range) => range_intersection(
                        range.start..range.end,
                        boundary.char_index_range.clone(),
                    )?
                    .into(),
                    HighlightSpanRange::ByteRange(range) => buffer
                        .byte_range_to_char_index_range(&range_intersection(
                            range.clone(),
                            boundary.byte_range.clone(),
                        )?)
                        .ok()?,
                    HighlightSpanRange::CharIndex(char_index) => range_intersection(
                        *char_index..(*char_index + 1),
                        boundary.char_index_range.clone(),
                    )?
                    .into(),
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

fn range_intersection<T: Ord>(a: Range<T>, b: Range<T>) -> Option<Range<T>> {
    let start = std::cmp::max(a.start, b.start);
    let end = std::cmp::min(a.end, b.end);
    if start < end {
        Some(start..end)
    } else {
        None
    }
}

#[derive(Clone)]
pub enum Source {
    StyleKey(StyleKey),
    Style(Style),
}

pub enum HighlightSpanRange {
    CharIndexRange(CharIndexRange),
    ByteRange(Range<usize>),
    CharIndex(CharIndex),
}

struct Boundary {
    byte_range: Range<usize>,
    char_index_range: Range<CharIndex>,
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
        }
    }
}
