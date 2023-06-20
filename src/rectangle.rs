use tree_sitter::Point;

use crate::screen::Dimension;

#[derive(Debug, PartialEq, Eq, Default, Clone)]
// A struct to represent a rectangle with origin, width and height
pub struct Rectangle {
    pub origin: Point,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, PartialEq, Eq, Clone)]
// A struct to represent a border with direction, start and length
pub struct Border {
    pub direction: BorderDirection,
    pub start: Point,
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
                origin: Point {
                    column: self.origin.column + (width1 as usize) + 1,
                    ..self.origin
                },
                width: width2,
                ..*self
            };
            // Create a vertical border between the two rectangles
            let border = Border {
                direction: BorderDirection::Vertical,
                start: Point {
                    column: self.origin.column + (width1 as usize),
                    row: self.origin.row,
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
                origin: Point {
                    row: self.origin.row + height1 as usize + 1,
                    ..self.origin
                },
                height: height2,
                ..*self
            };
            // Create a horizontal border between the two rectangles
            let border = Border {
                direction: BorderDirection::Horizontal,
                start: Point {
                    row: self.origin.row + height1 as usize,
                    column: self.origin.column,
                },
            };
            (rectangle1, rectangle2, border)
        }
    }

    // A method to generate a vector of rectangles and a vector of borders based on bspwm tiling algorithm
    pub fn generate(count: usize, dimension: Dimension) -> (Vec<Rectangle>, Vec<Border>) {
        // Create an empty vector to store the rectangles
        let mut rectangles = Vec::new();

        // Create an empty vector to store the borders
        let mut borders = Vec::new();

        // Create a root rectangle that covers the whole screen
        let root = Rectangle {
            origin: Point { row: 0, column: 0 },
            width: dimension.width,
            height: dimension.height,
        };

        // Push the root rectangle to the vector
        rectangles.push(root);

        // Loop through the count and split the last rectangle in the vector
        for _ in 0..count - 1 {
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

    /// Split the rectangle vertically at the given row.
    pub fn split_vertically_at(&self, row: usize) -> (Rectangle, Rectangle) {
        let rectangle1 = Rectangle {
            origin: self.origin,
            width: self.width,
            height: row as u16,
        };
        let rectangle2 = Rectangle {
            origin: Point {
                row: self.origin.row + row,
                ..self.origin
            },
            width: self.width,
            height: self.height - row as u16,
        };
        (rectangle1, rectangle2)
    }

    pub fn move_up(&self, offset: usize) -> Rectangle {
        Rectangle {
            origin: Point {
                row: self.origin.row - offset,
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
}

#[cfg(test)]
mod test_rectangle {
    use tree_sitter::Point;

    use crate::rectangle::Border;
    use crate::screen::Dimension;

    use super::BorderDirection::*;
    use super::Rectangle;

    #[test]
    fn split_vertically_at() {
        let rectangle = Rectangle {
            origin: Point { row: 0, column: 0 },
            width: 100,
            height: 100,
        };
        let (rectangle1, rectangle2) = rectangle.split_vertically_at(50);
        assert_eq!(
            rectangle1,
            Rectangle {
                origin: Point { row: 0, column: 0 },
                width: 100,
                height: 50,
            }
        );
        assert_eq!(
            rectangle2,
            Rectangle {
                origin: Point { row: 50, column: 0 },
                width: 100,
                height: 50,
            }
        );
    }

    #[test]
    fn generate_same_height_and_width() {
        let (rectangles, borders) = Rectangle::generate(
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
                    start: Point { row: 50, column: 0 }
                },
                Border {
                    direction: Horizontal,
                    start: Point { row: 75, column: 0 }
                },
                Border {
                    direction: Vertical,
                    start: Point {
                        row: 76,
                        column: 50
                    }
                }
            ]
        );

        assert_eq!(
            rectangles,
            vec![
                Rectangle {
                    origin: Point { row: 0, column: 0 },
                    width: 100,
                    height: 50
                },
                Rectangle {
                    origin: Point { row: 51, column: 0 },
                    width: 100,
                    height: 24
                },
                Rectangle {
                    origin: Point { row: 76, column: 0 },
                    width: 50,
                    height: 24
                },
                Rectangle {
                    origin: Point {
                        row: 76,
                        column: 51
                    },
                    width: 49,
                    height: 24
                }
            ]
        );
    }
}
