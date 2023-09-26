use itertools::{Either, Itertools};
use std::collections::HashSet;

use crate::{app::Dimension, position::Position};

#[derive(Debug, PartialEq, Eq, Default, Clone)]
// A struct to represent a rectangle with origin, width and height
pub struct Rectangle {
    pub origin: Position,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, PartialEq, Eq, Clone)]
// A struct to represent a border with direction, start and length
pub struct Border {
    pub direction: BorderDirection,
    pub start: Position,
}

impl Border {
    fn area(&self, dimension: &Dimension) -> usize {
        match self.direction {
            BorderDirection::Horizontal => {
                dimension.width.saturating_sub(self.start.column as u16) as usize
            }
            BorderDirection::Vertical => {
                dimension.height.saturating_sub(self.start.line as u16) as usize
            }
        }
    }

    fn positions(&self, dimension: &Dimension) -> HashSet<Position> {
        match self.direction {
            BorderDirection::Horizontal => (self.start.column..(dimension.width as usize))
                .map(|column| Position {
                    line: self.start.line,
                    column,
                })
                .collect(),
            BorderDirection::Vertical => (self.start.line..(dimension.height as usize))
                .map(|line| Position {
                    line,
                    column: self.start.column,
                })
                .collect(),
        }
    }

    fn intersection(&self, other: &Border, dimension: &Dimension) -> HashSet<Position> {
        self.positions(dimension)
            .intersection(&other.positions(dimension))
            .cloned()
            .collect()
    }

    fn new_vertical(start: Position) -> Border {
        Border {
            direction: BorderDirection::Vertical,
            start,
        }
    }

    fn new_horizontal(position: Position) -> Border {
        Border {
            direction: BorderDirection::Horizontal,
            start: position,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
// An enum to represent the direction of a border (horizontal or vertical)
pub enum BorderDirection {
    Horizontal,
    Vertical,
}

enum Element {
    Rectangle(Rectangle),
    Border(Border),
}

pub fn spread(length: usize, count: usize) -> Vec<usize> {
    if count == 0 {
        return vec![];
    }
    let element_size = length / count;
    let remainder = length % count;
    (0..count)
        .map(|index| element_size + if index < remainder { 1 } else { 0 })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Split {
    kind: SplitKind,
    /// 0-based, can be either a line or a column
    origin: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SplitKind {
    Rectangle {
        /// Can be either the length or the width
        size: u16,
    },
    Border,
}

fn split(length: usize, count: usize) -> Vec<Split> {
    let border_count = count - 1;
    let rectangles = spread(length - border_count, count)
        .into_iter()
        .map(|size| SplitKind::Rectangle { size: size as u16 });

    Itertools::intersperse(rectangles, SplitKind::Border)
        .scan(0_u16, |origin, kind| {
            let size = match kind {
                SplitKind::Rectangle { size } => size,
                SplitKind::Border => 1,
            };
            let split = Split {
                kind,
                origin: *origin,
            };
            *origin += size;
            Some(split)
        })
        .collect()
}

impl Rectangle {
    // A method to split a rectangle into two smaller ones based on a fixed ratio of 0.5 and return a border between them
    fn split(&self, vertical: bool) -> (Rectangle, Rectangle, Border) {
        if vertical {
            // Split vertically
            let width1 = self.width / 2;
            let width2 = self.width - width1 - 1; // Corrected the width2 to leave space for the border

            let rectangle1 = Rectangle {
                width: width1,
                ..*self
            };
            let rectangle2 = Rectangle {
                origin: Position {
                    column: self.origin.column + (width1 as usize) + 1,
                    ..self.origin
                },
                width: width2,
                ..*self
            };
            // Create a vertical border between the two rectangles
            let border = Border {
                direction: BorderDirection::Vertical,
                start: Position {
                    column: self.origin.column + (width1 as usize),
                    line: self.origin.line,
                },
            };
            (rectangle1, rectangle2, border)
        } else {
            // Split horizontally
            let height1 = self.height / 2;
            let height2 = self.height - height1 - 1; // Corrected the height2 to leave space for the border
            let rectangle1 = Rectangle {
                height: height1,
                ..*self
            };
            let rectangle2 = Rectangle {
                origin: Position {
                    line: self.origin.line + height1 as usize + 1,
                    ..self.origin
                },
                height: height2,
                ..*self
            };
            // Create a horizontal border between the two rectangles
            let border = Border {
                direction: BorderDirection::Horizontal,
                start: Position {
                    line: self.origin.line + height1 as usize,
                    column: self.origin.column,
                },
            };
            (rectangle1, rectangle2, border)
        }
    }

    fn generate_wide(
        count: usize,
        top_rectangle_height_percentage: f32,
        dimension: Dimension,
    ) -> (Vec<Rectangle>, Vec<Border>) {
        let rectangle = Rectangle {
            origin: Position::new(0, 0),
            width: dimension.width,
            height: dimension.height,
        };

        if count == 1 {
            return (vec![rectangle], vec![]);
        }

        let split_at = (dimension.height as f32 * top_rectangle_height_percentage) as usize;

        let (up, bottom) = rectangle.split_horizontally_at(split_at);
        let (rectangles, borders): (Vec<Rectangle>, Vec<Border>) = bottom
            .split_vertically(count - 1)
            .into_iter()
            .partition_map(|element| match element {
                Element::Rectangle(rectangle) => Either::Left(rectangle),
                Element::Border(border) => Either::Right(border),
            });
        let rectangles = vec![up].into_iter().chain(rectangles.into_iter()).collect();

        (rectangles, borders)
    }

    fn split_vertically(&self, count: usize) -> Vec<Element> {
        split(self.width as usize, count)
            .into_iter()
            .map(|split| {
                let column = self.origin.column + split.origin as usize;
                let position = Position {
                    column,
                    ..self.origin
                };
                match split.kind {
                    SplitKind::Rectangle { size } => Element::Rectangle(Rectangle {
                        origin: position,
                        width: size,
                        height: self.height,
                    }),
                    SplitKind::Border => Element::Border(Border::new_vertical(position)),
                }
            })
            .collect()
    }

    fn split_horizontally(&self, count: usize) -> Vec<Element> {
        split(self.height as usize, count)
            .into_iter()
            .map(|split| {
                let line = self.origin.line + split.origin as usize;
                let position = Position {
                    line,
                    ..self.origin
                };
                match split.kind {
                    SplitKind::Rectangle { size } => Element::Rectangle(Rectangle {
                        origin: position,
                        width: self.width,
                        height: size,
                    }),
                    SplitKind::Border => Element::Border(Border::new_horizontal(position)),
                }
            })
            .collect()
    }

    fn generate_tall(
        count: usize,
        left_rectangle_width_percentage: f32,
        dimension: Dimension,
    ) -> (Vec<Rectangle>, Vec<Border>) {
        let split_at = (dimension.width as f32 * left_rectangle_width_percentage) as usize;
        let initial = Rectangle {
            origin: Position::new(0, 0),
            width: dimension.width,
            height: dimension.height,
        };

        if count == 1 {
            return (vec![initial], vec![]);
        }

        let (left, right) = initial.split_vertically_at(split_at);

        let center_border = Border::new_vertical(Position {
            column: split_at,
            line: 0,
        });
        let right = right.clamp_left(1);

        let (rectangles, borders): (Vec<Rectangle>, Vec<Border>) = right
            .split_horizontally(count - 1)
            .into_iter()
            .partition_map(|element| match element {
                Element::Rectangle(rectangle) => Either::Left(rectangle),
                Element::Border(border) => Either::Right(border),
            });

        let rectangles = vec![left]
            .into_iter()
            .chain(rectangles.into_iter())
            .collect();
        let borders = vec![center_border]
            .into_iter()
            .chain(borders.into_iter())
            .collect();
        (rectangles, borders)
    }

    // A method to generate a vector of rectangles and a vector of borders based on bspwm tiling algorithm
    pub fn generate_binary_partition(
        count: usize,
        dimension: Dimension,
    ) -> (Vec<Rectangle>, Vec<Border>) {
        // Create an empty vector to store the rectangles
        let mut rectangles = Vec::new();

        // Create an empty vector to store the borders
        let mut borders = Vec::new();

        // Create a root rectangle that covers the whole screen
        let root = Rectangle {
            origin: Position { line: 0, column: 0 },
            width: dimension.width,
            height: dimension.height,
        };

        // Push the root rectangle to the vector
        rectangles.push(root);

        // Loop through the count and split the last rectangle in the vector
        for _ in 0..count.saturating_sub(1) {
            // Pop the last rectangle from the vector
            let last = rectangles.pop().unwrap();

            // Choose the direction to split based on the rectangle's height and width
            let cursor_width_to_cursor_height_ratio = 3;
            let vertical = last.width >= last.height * cursor_width_to_cursor_height_ratio;

            // Split the last rectangle into two smaller ones and get a border between them
            let (rectangle1, rectangle2, border) = last.split(vertical);

            // Push the two smaller rectangles to the vector
            rectangles.push(rectangle1);
            rectangles.push(rectangle2);

            // Push the border to the vector
            borders.push(border);
        }

        // Return the vector of rectangles and the vector of borders
        (rectangles, borders)
    }

    pub fn dimension(&self) -> Dimension {
        Dimension {
            width: self.width,
            height: self.height,
        }
    }

    /// Split the rectangle horizontally at the given line.
    pub fn split_horizontally_at(&self, line: usize) -> (Rectangle, Rectangle) {
        let up = Rectangle {
            origin: self.origin,
            width: self.width,
            height: line as u16,
        };
        let bottom = Rectangle {
            origin: self.origin.move_down(line),
            width: self.width,
            height: self.height.saturating_sub(line as u16),
        };
        (up, bottom)
    }

    pub fn split_vertically_at(&self, column: usize) -> (Rectangle, Rectangle) {
        let left = Rectangle {
            origin: self.origin,
            width: column as u16,
            height: self.height,
        };
        let right = Rectangle {
            origin: self.origin.move_right(column as u16),
            width: self.width.saturating_sub(column as u16),
            height: self.height,
        };
        (left, right)
    }

    pub fn move_up(&self, offset: usize) -> Rectangle {
        Rectangle {
            origin: self.origin.move_up(offset),
            ..*self
        }
    }

    pub fn set_height(&self, height: usize) -> Rectangle {
        Rectangle {
            height: height as u16,
            ..*self
        }
    }

    #[cfg(test)]
    fn area(&self) -> usize {
        self.width as usize * self.height as usize
    }

    #[cfg(test)]
    fn positions(&self) -> HashSet<Position> {
        (self.origin.line..self.origin.line + self.height as usize)
            .flat_map(|line| {
                (self.origin.column..self.origin.column + self.width as usize)
                    .map(move |column| Position { line, column })
            })
            .collect()
    }

    #[cfg(test)]
    fn intersection(&self, other: &Rectangle) -> HashSet<Position> {
        self.positions()
            .intersection(&other.positions())
            .cloned()
            .collect()
    }

    pub fn clamp_top(&self, by: usize) -> Rectangle {
        Rectangle {
            origin: Position {
                line: self.origin.line.saturating_add(by),
                ..self.origin
            },
            height: self.height.saturating_sub(by as u16),
            width: self.width,
        }
    }

    fn move_right(&self, arg: i32) -> Rectangle {
        Rectangle {
            origin: Position {
                column: self
                    .origin
                    .column
                    .saturating_add(arg as usize)
                    .min(self.width.saturating_sub(1) as usize),
                ..self.origin
            },
            ..*self
        }
    }

    fn clamp_left(&self, width: usize) -> Rectangle {
        Rectangle {
            origin: Position {
                column: self.origin.column.saturating_add(width),
                ..self.origin
            },
            width: self.width.saturating_sub(width as u16),
            ..*self
        }
    }

    pub fn generate(
        kind: LayoutKind,
        count: usize,
        ratio: f32,
        dimension: Dimension,
    ) -> (Vec<Rectangle>, Vec<Border>) {
        match kind {
            LayoutKind::Tall => Rectangle::generate_tall(count, ratio, dimension),
            LayoutKind::Wide => Rectangle::generate_wide(count, ratio, dimension),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Tall,
    Wide,
}

#[cfg(test)]
mod test_rectangle {

    use crate::app::Dimension;
    use crate::position::Position;
    use crate::rectangle::split;
    use crate::rectangle::spread;
    use crate::rectangle::Border;
    use crate::rectangle::Split;
    use crate::rectangle::SplitKind;

    use super::BorderDirection::*;
    use super::Rectangle;

    #[test]
    fn test_spread() {
        assert_eq!(spread(10, 3), [4, 3, 3].to_vec());
        assert_eq!(spread(10, 4), [3, 3, 2, 2].to_vec());
        assert_eq!(spread(10, 5), [2, 2, 2, 2, 2].to_vec())
    }

    #[test]
    fn test_split() {
        assert_eq!(
            split(10, 3),
            [
                Split {
                    kind: SplitKind::Rectangle { size: 3 },
                    origin: 0
                },
                Split {
                    kind: SplitKind::Border,
                    origin: 3
                },
                Split {
                    kind: SplitKind::Rectangle { size: 3 },
                    origin: 4
                },
                Split {
                    kind: SplitKind::Border,
                    origin: 7
                },
                Split {
                    kind: SplitKind::Rectangle { size: 2 },
                    origin: 8
                }
            ]
            .to_vec()
        );
    }

    #[test]
    fn generate_wide() {
        let (rectangles, _) = Rectangle::generate_wide(
            3,
            0.5,
            Dimension {
                width: 10,
                height: 10,
            },
        );

        assert_eq!(
            rectangles,
            [
                Rectangle {
                    origin: Position { line: 0, column: 0 },
                    width: 10,
                    height: 5
                },
                Rectangle {
                    origin: Position { line: 5, column: 0 },
                    width: 5,
                    height: 5
                },
                Rectangle {
                    origin: Position { line: 5, column: 6 },
                    width: 4,
                    height: 5
                }
            ]
            .to_vec()
        );
    }

    mod generate_tall {

        use std::collections::HashSet;

        use crate::rectangle::LayoutKind;

        use super::*;

        use quickcheck::{Arbitrary, Gen};
        use quickcheck_macros::quickcheck;

        impl Arbitrary for Dimension {
            fn arbitrary(g: &mut Gen) -> Dimension {
                Dimension {
                    width: 10,
                    height: *g.choose(&(10..20).collect::<Vec<u16>>()).unwrap(),
                }
            }
        }

        #[derive(Debug, Clone)]
        struct Count(usize);

        impl Arbitrary for Count {
            fn arbitrary(g: &mut Gen) -> Count {
                Count(*g.choose(&[1, 2, 3, 4, 5]).unwrap())
            }
        }

        impl Arbitrary for LayoutKind {
            fn arbitrary(g: &mut Gen) -> Self {
                *g.choose(&[LayoutKind::Tall, LayoutKind::Wide]).unwrap()
            }
        }

        #[quickcheck]
        fn qc_rectangles_and_borders_area_equals_dimension_area(
            count: Count,
            layout_kind: LayoutKind,
            dimension: Dimension,
        ) -> bool {
            let (rectangles, borders) = Rectangle::generate(layout_kind, count.0, 0.5, dimension);
            let rectangles_area: usize = rectangles.iter().map(|r| r.area()).sum();
            let borders_area: usize = borders.iter().map(|b| b.area(&dimension)).sum();
            let dimension_area = dimension.area();

            rectangles_area + borders_area == dimension_area
        }

        #[quickcheck]
        fn qc_all_dimension_positions_are_filled_perfectly(
            count: Count,
            layout_kind: LayoutKind,
            dimension: Dimension,
        ) -> bool {
            let (rectangles, borders) = Rectangle::generate(layout_kind, count.0, 0.5, dimension);

            let rectangle_and_border_positions = rectangles
                .iter()
                .flat_map(|rectangle| rectangle.positions())
                .chain(
                    borders
                        .iter()
                        .flat_map(|border| border.positions(&dimension)),
                )
                .collect::<HashSet<Position>>();

            let dimension_positions = dimension.positions();

            let dimension_diff_rectangle_and_border = (dimension_positions)
                .difference(&rectangle_and_border_positions)
                .collect::<Vec<_>>();

            let rectangle_and_border_diff_dimension = (rectangle_and_border_positions)
                .difference(&dimension_positions)
                .collect::<Vec<_>>();

            dimension_diff_rectangle_and_border.is_empty()
                && rectangle_and_border_diff_dimension.is_empty()
        }

        #[quickcheck]
        fn qc_no_rectangles_and_borders_overlapped(
            count: Count,
            layout_kind: LayoutKind,
            dimension: Dimension,
        ) -> bool {
            let (rectangles, borders) = Rectangle::generate(layout_kind, count.0, 0.5, dimension);

            let rectangles_intersections = rectangles
                .iter()
                .flat_map(|r1| {
                    rectangles
                        .iter()
                        .filter(|r2| *r1 != **r2)
                        .map(|r2| r1.intersection(r2))
                })
                .filter(|intersection| !intersection.is_empty())
                .collect::<Vec<_>>();

            let borders_intersections = borders
                .iter()
                .flat_map(|b1| {
                    borders
                        .iter()
                        .filter(|b2| *b1 != **b2)
                        .map(|b2| b1.intersection(b2, &dimension))
                })
                .filter(|intersection| !intersection.is_empty())
                .collect::<Vec<_>>();

            let rectangles_and_borders_intersections = rectangles
                .iter()
                .flat_map(|rectangle| {
                    borders
                        .iter()
                        .map(|border| {
                            let border_positions = border.positions(&dimension);
                            rectangle
                                .positions()
                                .intersection(&border_positions)
                                .copied()
                                .collect::<HashSet<Position>>()
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|intersection| !intersection.is_empty())
                .collect::<Vec<_>>();

            rectangles_intersections.is_empty()
                && borders_intersections.is_empty()
                && rectangles_and_borders_intersections.is_empty()
        }
    }

    mod generate_binary_partition {
        use super::*;
        #[test]
        fn split_horizontally_at() {
            let rectangle = Rectangle {
                origin: Position { line: 0, column: 0 },
                width: 100,
                height: 100,
            };
            let (rectangle1, rectangle2) = rectangle.split_horizontally_at(50);
            assert_eq!(
                rectangle1,
                Rectangle {
                    origin: Position { line: 0, column: 0 },
                    width: 100,
                    height: 50,
                }
            );
            assert_eq!(
                rectangle2,
                Rectangle {
                    origin: Position {
                        line: 50,
                        column: 0
                    },
                    width: 100,
                    height: 50,
                }
            );
        }

        #[test]
        fn generate_same_height_and_width() {
            let (rectangles, borders) = Rectangle::generate_binary_partition(
                4,
                Dimension {
                    width: 100,
                    height: 100,
                },
            );

            assert_eq!(rectangles.len(), 4);
            assert_eq!(borders.len(), 3);

            assert_eq!(
                borders,
                vec![
                    Border {
                        direction: Horizontal,
                        start: Position {
                            line: 50,
                            column: 0
                        }
                    },
                    Border {
                        direction: Horizontal,
                        start: Position {
                            line: 75,
                            column: 0
                        }
                    },
                    Border {
                        direction: Vertical,
                        start: Position {
                            line: 76,
                            column: 50
                        }
                    }
                ]
            );

            assert_eq!(
                rectangles,
                vec![
                    Rectangle {
                        origin: Position { line: 0, column: 0 },
                        width: 100,
                        height: 50
                    },
                    Rectangle {
                        origin: Position {
                            line: 51,
                            column: 0
                        },
                        width: 100,
                        height: 24
                    },
                    Rectangle {
                        origin: Position {
                            line: 76,
                            column: 0
                        },
                        width: 50,
                        height: 24
                    },
                    Rectangle {
                        origin: Position {
                            line: 76,
                            column: 51
                        },
                        width: 49,
                        height: 24
                    }
                ]
            );
        }
    }
}
