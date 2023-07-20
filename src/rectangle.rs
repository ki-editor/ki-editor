use std::collections::HashSet;

use crate::{position::Position, screen::Dimension};

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
}

#[derive(Debug, PartialEq, Eq, Clone)]
// An enum to represent the direction of a border (horizontal or vertical)
pub enum BorderDirection {
    Horizontal,
    Vertical,
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

    pub fn generate_tall(count: usize, dimension: Dimension) -> (Vec<Rectangle>, Vec<Border>) {
        if count == 0 {
            return (vec![], vec![]);
        }
        if count == 1 {
            return (
                [Rectangle {
                    origin: Position::new(0, 0),
                    width: dimension.width,
                    height: dimension.height,
                }]
                .to_vec(),
                Vec::new(),
            );
        }

        let left_width = dimension.width / 2;
        let left = Rectangle {
            origin: Position::new(0, 0),
            width: left_width,
            height: dimension.height,
        };

        let center_border = Border {
            direction: BorderDirection::Vertical,
            start: Position::new(0, left_width as usize),
        };

        let right_rectangles_count = count - 1;

        let right_width = dimension.width - left_width - 1;
        let column = (right_width + 2) as usize;

        let right_rectangle_height = dimension.height / right_rectangles_count as u16;
        let remainder = dimension.height % right_rectangles_count as u16;

        let right_rectangle_heights = (0..right_rectangles_count)
            .map(|index| right_rectangle_height + if index < remainder as usize { 1 } else { 0 })
            .collect::<Vec<u16>>();

        let cumulative_rectangle_heights = [&0]
            .to_vec()
            .into_iter()
            .chain(
                right_rectangle_heights
                    .iter()
                    .take(right_rectangles_count - 1),
            )
            .scan(0, |cumulative_height, height| {
                *cumulative_height += height;
                Some(*cumulative_height)
            })
            .collect::<Vec<u16>>();

        let zipped = right_rectangle_heights
            .into_iter()
            .zip(cumulative_rectangle_heights.into_iter())
            .collect::<Vec<(u16, u16)>>();

        let (right_rectangles, borders): (Vec<Rectangle>, Vec<Option<Border>>) = zipped
            .into_iter()
            .enumerate()
            .map(|(index, (height, cumulative_height))| {
                let border = if index < right_rectangles_count - 1 {
                    Some(Border {
                        direction: BorderDirection::Horizontal,
                        start: Position::new(
                            (cumulative_height as usize + height as usize).saturating_sub(1),
                            column,
                        ),
                    })
                } else {
                    None
                };
                let rectangle = Rectangle {
                    origin: Position::new(cumulative_height as usize, column),
                    width: right_width,
                    height: height.saturating_sub(if border.is_some() { 1 } else { 0 } as u16),
                };
                (rectangle, border)
            })
            .unzip();

        // let (right_rectangles, borders): (Vec<Rectangle>, Vec<Option<Border>>) = (0
        //     ..right_rectangles_count)
        //     .map(|index| {
        //         let height =
        //             right_rectangle_height + if index < remainder as usize { 1 } else { 0 };

        //         let line = height as usize * index;
        //         let rectangle = Rectangle {
        //             origin: Position::new(line, column),
        //             width: right_width,
        //             height,
        //         };

        //         (rectangle, None)
        //     })
        //     .unzip();

        (
            [left].into_iter().chain(right_rectangles).collect(),
            [Some(center_border)]
                .into_iter()
                .chain(borders.into_iter())
                .flatten()
                .collect(),
        )
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

    /// Split the rectangle vertically at the given line.
    pub fn split_vertically_at(&self, line: usize) -> (Rectangle, Rectangle) {
        let rectangle1 = Rectangle {
            origin: self.origin,
            width: self.width,
            height: line as u16,
        };
        let rectangle2 = Rectangle {
            origin: Position {
                line: self.origin.line + line,
                ..self.origin
            },
            width: self.width,
            height: self.height - line as u16,
        };
        (rectangle1, rectangle2)
    }

    pub fn move_up(&self, offset: usize) -> Rectangle {
        Rectangle {
            origin: Position {
                line: self.origin.line - offset,
                ..self.origin
            },
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
}

#[cfg(test)]
mod test_rectangle {

    use crate::position::Position;
    use crate::rectangle::Border;
    use crate::screen::Dimension;

    use super::BorderDirection::*;
    use super::Rectangle;

    mod generate_tall {

        use std::collections::HashSet;

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

        #[quickcheck]
        fn qc_rectangles_and_borders_area_equals_dimension_area(
            count: Count,
            dimension: Dimension,
        ) -> bool {
            let (rectangles, borders) = Rectangle::generate_tall(count.0, dimension);
            let rectangles_area: usize = rectangles.iter().map(|r| r.area()).sum();
            let borders_area: usize = borders.iter().map(|b| b.area(&dimension)).sum();
            let dimension_area = dimension.area();

            rectangles_area + borders_area == dimension_area
        }

        #[quickcheck]
        fn qc_all_dimension_positions_are_filled_perfectly(
            count: Count,
            dimension: Dimension,
        ) -> bool {
            let (rectangles, borders) = Rectangle::generate_tall(count.0, dimension);

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
        fn qc_no_rectangles_and_borders_overlapped(count: Count, dimension: Dimension) -> bool {
            let (rectangles, borders) = Rectangle::generate_tall(count.0, dimension);

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
        fn split_vertically_at() {
            let rectangle = Rectangle {
                origin: Position { line: 0, column: 0 },
                width: 100,
                height: 100,
            };
            let (rectangle1, rectangle2) = rectangle.split_vertically_at(50);
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
