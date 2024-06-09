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
    grid::{CellUpdate, Grid, LineUpdate, RenderContentLineNumber, StyleKey},
    selection::{CharIndex, Selection},
    selection_mode::{self, ByteRange},
    style::Style,
    themes::Theme,
};

use super::{component::GetGridResult, editor::Editor};

use StyleKey::*;

impl Editor {
    pub(crate) fn get_grid(&self, context: &Context) -> GetGridResult {
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
        let visible_lines = &rope
            .lines()
            .enumerate()
            .skip(scroll_offset as usize)
            .take(height as usize)
            .map(|(line_index, slice)| (line_index, slice.to_string()))
            .collect_vec();

        let visible_lines_grid: Grid = Grid::new(Dimension { height, width });

        let selection = &editor.selection_set.primary;
        // If the buffer selection is updated less recently than the window's scroll offset,

        // use the window's scroll offset.

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
        let custom_regex_highlights = lazy_regex::regex!("(?i)#[0-9a-f]{6}")
            .find_iter(&rope.to_string())
            .map(|m| (m.as_str().to_string(), m.range()))
            .filter_map(|(hex, range)| {
                let color = crate::themes::Color::from_hex(&hex).ok()?;
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

        let visible_parent_lines = visible_parent_lines.into_iter().map(|line| HighlightSpan {
            source: Source::StyleKey(StyleKey::ParentLine),
            ranges: HighlightSpanRange::Line(line.line),
            set_symbol: None,
            is_cursor: false,
        });
        let updates = vec![]
            .into_iter()
            .chain(visible_parent_lines)
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
            .collect_vec();
        let visible_lines_updates = {
            let boundaries = [Boundary::new(&buffer, self.visible_line_range())];
            updates
                .iter()
                .flat_map(|span| span.to_cell_update(&buffer, theme, &boundaries))
                .chain(primary_selection_primary_cursor)
                .collect_vec()
        };

        let visible_lines_grid = visible_lines_grid.render_content(
            &visible_lines.iter().map(|(_, line)| line).join(""),
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

        let hidden_parent_lines_grid = {
            let line_indices = hidden_parent_lines.iter().map(|line| line.line);
            let hidden_parent_line_range = line_indices.clone().min().unwrap_or_default()
                ..line_indices.max().unwrap_or_default() + 1;
            let boundaries = [Boundary::new(&buffer, hidden_parent_line_range)];
            let updates = hidden_parent_lines
                .iter()
                .map(|line| HighlightSpan {
                    source: Source::StyleKey(StyleKey::ParentLine),
                    ranges: HighlightSpanRange::Line(line.line),
                    set_symbol: None,
                    is_cursor: false,
                })
                .chain(updates)
                .flat_map(|span| span.to_cell_update(&buffer, theme, &boundaries))
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
            height: editor.dimension().height - grid.rows.len() as u16,
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
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(selection, true)?;
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

pub(crate) struct HighlightSpan {
    pub(crate) source: Source,
    pub(crate) ranges: HighlightSpanRange,
    pub(crate) set_symbol: Option<String>,
    pub(crate) is_cursor: bool,
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
                    HighlightSpanRange::Line(line) => buffer
                        .line_range_to_char_index_range(range_intersection(
                            *line..line + 1,
                            boundary.line_range.clone(),
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
pub(crate) enum Source {
    StyleKey(StyleKey),
    Style(Style),
}

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
        let grid = editor.get_grid(&Context::default());
        let cells = grid.grid.to_positioned_cells();
        cells.into_iter().all(|cell| {
            cell.position.line < (rectangle.height as usize)
                && cell.position.column < (rectangle.width as usize)
        })
    }
}
