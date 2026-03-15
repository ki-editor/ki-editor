use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    queue,
    style::{
        Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor, SetUnderlineColor,
    },
    Command,
};

use crate::{
    frontend::MyWriter,
    grid::{CellLineStyle, PositionedCell},
    position::Position,
    themes::Color,
};

struct Renderer<'a>(&'a mut Box<dyn MyWriter>);

fn cell_underline_color(cell: &PositionedCell) -> Option<Color> {
    cell.cell.line.map(|line| line.color)
}

fn cell_underline_state(cell: &PositionedCell) -> Option<CellLineStyle> {
    cell.cell.line.map(|line| line.style)
}

impl<'a> Renderer<'a> {
    fn command<C: Command>(&mut self, command: C) -> anyhow::Result<()> {
        queue!(self.0, command).map_err(Into::into)
    }

    fn move_to(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(MoveTo(
            cell.position.column as u16,
            cell.position.line as u16,
        ))
    }

    fn move_to_column(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(MoveToColumn(cell.position.column as u16))
    }

    fn bold(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(SetAttribute(if cell.cell.is_bold {
            Attribute::Bold
        } else {
            Attribute::NoBold
        }))
    }

    fn underline_color(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(SetUnderlineColor(
            cell_underline_color(cell).map_or(crossterm::style::Color::Reset, Into::into),
        ))
    }

    fn underline_state(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(SetAttribute(
            cell_underline_state(cell)
                .map(|style| match style {
                    crate::grid::CellLineStyle::Undercurl => Attribute::Undercurled,
                    crate::grid::CellLineStyle::Underline => Attribute::Underlined,
                })
                .unwrap_or(Attribute::NoUnderline),
        ))
    }

    fn background_color(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(SetBackgroundColor(cell.cell.background_color.into()))
    }

    fn foreground_color(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(SetForegroundColor(cell.cell.foreground_color.into()))
    }

    fn print(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        self.command(Print(reveal(cell.cell.symbol)))
    }
}

/// This struct is created so that we can avoid emitting redundant terminal
/// escape sequences.
/// Once terminal attributes (colors, bold, underline) are
/// set, they persist for all subsequent character prints until explicitly
/// changed.
/// By tracking the current state, we only emit styling commands when
/// attributes actually differ from the previous cell, rather than redundantly
/// setting the same styles for every cell.
///
/// There is one caveat: on windows, changing styles causes the foreground and
/// background colors to reset, so we need to reapply them.
struct TerminalState<'a> {
    renderer: Renderer<'a>,
    position: Position,
    bold: bool,
    underline_color: Option<Color>,
    underline_state: Option<CellLineStyle>,
    foreground_color: Color,
    background_color: Color,
}

impl<'a> TerminalState<'a> {
    fn render(&mut self, cell: &PositionedCell) -> anyhow::Result<()> {
        // On windows, text attributes reset the fg/bg color.
        let force_color = if cfg!(windows) {
            self.bold != cell.cell.is_bold
                || self.underline_color != cell_underline_color(cell)
                || self.underline_state != cell_underline_state(cell)
        } else {
            false
        };
        if self.position.line == cell.position.line {
            self.renderer.move_to_column(cell)?;
        } else {
            self.renderer.move_to(cell)?;
        };
        if self.bold != cell.cell.is_bold {
            self.renderer.bold(cell)?;
        }
        if self.underline_color != cell_underline_color(cell) {
            self.renderer.underline_color(cell)?;
        }
        if self.underline_state != cell_underline_state(cell) {
            self.renderer.underline_state(cell)?;
        }
        if self.background_color != cell.cell.background_color || force_color {
            self.renderer.background_color(cell)?;
        }
        if self.foreground_color != cell.cell.foreground_color || force_color {
            self.renderer.foreground_color(cell)?;
        }

        self.position = cell.position;
        self.bold = cell.cell.is_bold;
        self.underline_color = cell_underline_color(cell);
        self.underline_state = cell_underline_state(cell);
        self.background_color = cell.cell.background_color;
        self.foreground_color = cell.cell.foreground_color;

        // The symbol is always printed, otherwise the attributes aren't applied
        self.renderer.print(cell)
    }
}

pub(super) fn render_cells(
    writer: &mut Box<dyn MyWriter>,
    cells: Vec<PositionedCell>,
) -> anyhow::Result<()> {
    let mut renderer = Renderer(writer);

    let mut cells = cells.into_iter();
    let cell = match cells.next() {
        Some(cell) => cell,
        None => return Ok(()),
    };

    renderer.move_to(&cell)?;
    renderer.bold(&cell)?;
    renderer.underline_color(&cell)?;
    renderer.underline_state(&cell)?;
    renderer.background_color(&cell)?;
    renderer.foreground_color(&cell)?;
    renderer.print(&cell)?;

    let mut terminal_state = TerminalState {
        renderer,
        position: cell.position,
        bold: cell.cell.is_bold,
        underline_color: cell_underline_color(&cell),
        underline_state: cell.cell.line.map(|line| line.style),
        foreground_color: cell.cell.foreground_color,
        background_color: cell.cell.background_color,
    };

    for cell in cells {
        terminal_state.render(&cell)?;
    }
    Ok(())
}

/// Convert invisible character to visible character
fn reveal(s: char) -> char {
    match s {
        '\n' => ' ',
        '\t' => ' ',
        _ => s,
    }
}
