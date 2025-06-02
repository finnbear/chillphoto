use genpdfi::{elements::CellDecorator, render, style::LineStyle, Margins, Mm, Position};

/// This is a copy of [`genpdfi`]'s [`FrameCellDecorator`] with more margins.
#[derive(Clone, Debug, Default)]
pub struct MarginFrameCellDecorator {
    inner: bool,
    outer: bool,
    cont: bool,
    line_style: LineStyle,
    num_columns: usize,
    num_rows: usize,
    last_row: Option<usize>,
    margin: Mm,
}

impl MarginFrameCellDecorator {
    /// Creates a new frame cell decorator with the given settings for inner, outer and
    /// continuation borders.
    pub fn new(inner: bool, outer: bool, cont: bool) -> Self {
        Self {
            inner,
            outer,
            cont,
            margin: 3f32.into(),
            ..Default::default()
        }
    }

    /// Creates a new frame cell decorator with the given border settings, as well as a line style.
    #[allow(unused)]
    pub fn with_line_style(
        inner: bool,
        outer: bool,
        cont: bool,
        line_style: impl Into<LineStyle>,
    ) -> Self {
        Self {
            inner,
            outer,
            cont,
            line_style: line_style.into(),
            ..Default::default()
        }
    }

    fn print_left(&self, column: usize) -> bool {
        if column == 0 {
            self.outer
        } else {
            self.inner
        }
    }

    fn print_right(&self, column: usize) -> bool {
        if column + 1 == self.num_columns {
            self.outer
        } else {
            false
        }
    }

    fn print_top(&self, row: usize) -> bool {
        if self.last_row.map(|last_row| row > last_row).unwrap_or(true) {
            if row == 0 {
                self.outer
            } else {
                self.inner
            }
        } else {
            self.cont
        }
    }

    fn print_bottom(&self, row: usize, has_more: bool) -> bool {
        if has_more {
            self.cont
        } else if row + 1 == self.num_rows {
            self.outer
        } else {
            false
        }
    }
}

impl CellDecorator for MarginFrameCellDecorator {
    fn set_table_size(&mut self, num_columns: usize, num_rows: usize) {
        self.num_columns = num_columns;
        self.num_rows = num_rows;
    }

    fn prepare_cell<'p>(
        &self,
        column: usize,
        row: usize,
        mut area: render::Area<'p>,
    ) -> render::Area<'p> {
        let margin = self.margin + self.line_style.thickness();
        let margins = Margins::trbl(
            if self.print_top(row) {
                margin
            } else {
                0.into()
            },
            if self.print_right(column) {
                margin
            } else {
                0.into()
            },
            if self.print_bottom(row, false) {
                margin
            } else {
                0.into()
            },
            if self.print_left(column) {
                margin
            } else {
                0.into()
            },
        );
        area.add_margins(margins);
        area
    }

    fn decorate_cell(
        &mut self,
        column: usize,
        row: usize,
        has_more: bool,
        area: render::Area<'_>,
        row_height: Mm,
    ) -> Mm {
        let print_top = self.print_top(row);
        let print_bottom = self.print_bottom(row, has_more);
        let print_left = self.print_left(column);
        let print_right = self.print_right(column);

        let size = area.size();
        let line_offset = self.line_style.thickness() / 2.0;

        let left = Mm::from(0);
        let right = size.width;
        let top = Mm::from(0);
        let bottom = row_height
            + if print_bottom {
                self.line_style.thickness() + self.margin
            } else {
                0.into()
            }
            + if print_top {
                self.line_style.thickness() + self.margin
            } else {
                0.into()
            };

        let mut total_height = row_height;

        if print_top {
            area.draw_line(
                vec![
                    Position::new(left, top + line_offset),
                    Position::new(right, top + line_offset),
                ],
                self.line_style,
            );
            total_height += self.margin;
            total_height += self.line_style.thickness();
        }

        if print_right {
            area.draw_line(
                vec![
                    Position::new(right - line_offset, top),
                    Position::new(right - line_offset, bottom),
                ],
                self.line_style,
            );
        }

        if print_bottom {
            area.draw_line(
                vec![
                    Position::new(left, bottom - line_offset),
                    Position::new(right, bottom - line_offset),
                ],
                self.line_style,
            );
            total_height += self.margin;
            total_height += self.line_style.thickness();
        }

        if print_left {
            area.draw_line(
                vec![
                    Position::new(left + line_offset, top),
                    Position::new(left + line_offset, bottom),
                ],
                self.line_style,
            );
        }

        if column + 1 == self.num_columns {
            self.last_row = Some(row);
        }

        total_height
    }
}
