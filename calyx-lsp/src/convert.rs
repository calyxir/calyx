//! Both lsp_types and tree_sitter use Point and Range types to represent
//! a position in a document, or a range in a document. This module contains
//! some definitions to make converting between them more ergonomic.

use tower_lsp::lsp_types as lspt;
use tree_sitter as ts;

/// Crate local Point representing a location in a document
#[derive(Clone, Debug, PartialEq)]
pub struct Point(ts::Point);

#[allow(unused)]
impl Point {
    pub fn row(&self) -> usize {
        self.0.row
    }

    pub fn column(&self) -> usize {
        self.0.column
    }

    pub fn new(row: usize, column: usize) -> Self {
        Self(ts::Point { row, column })
    }

    pub fn zero() -> Self {
        Self(ts::Point { row: 0, column: 0 })
    }
}

impl From<Point> for ts::Point {
    fn from(value: Point) -> Self {
        value.0
    }
}

impl From<ts::Point> for Point {
    fn from(value: ts::Point) -> Self {
        Point(value)
    }
}

impl From<Point> for lspt::Position {
    fn from(value: Point) -> Self {
        lspt::Position::new(value.0.row as u32, value.0.column as u32)
    }
}

impl From<lspt::Position> for Point {
    fn from(value: lspt::Position) -> Self {
        Point(ts::Point {
            row: value.line as usize,
            column: value.character as usize,
        })
    }
}

impl PartialOrd<Point> for Point {
    fn partial_cmp(&self, other: &Point) -> Option<std::cmp::Ordering> {
        match self.row().cmp(&other.row()) {
            x @ std::cmp::Ordering::Less => Some(x),
            std::cmp::Ordering::Equal => {
                self.column().partial_cmp(&other.column())
            }
            x @ std::cmp::Ordering::Greater => Some(x),
        }
    }
}

/// Crate local Range representing a region between two points
#[derive(Debug)]
pub struct Range {
    start: Point,
    end: Point,
}

#[allow(unused)]
impl Range {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    pub fn zero() -> Self {
        Self {
            start: Point::zero(),
            end: Point::zero(),
        }
    }
}

impl<'a> From<ts::Node<'a>> for Range {
    fn from(value: ts::Node) -> Self {
        Range {
            start: value.start_position().into(),
            end: value.end_position().into(),
        }
    }
}

impl From<Range> for lspt::Range {
    fn from(val: Range) -> Self {
        lspt::Range::new(val.start.into(), val.end.into())
    }
}

impl From<lspt::Range> for Range {
    fn from(value: lspt::Range) -> Self {
        Range {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

impl From<ts::Range> for Range {
    fn from(value: ts::Range) -> Self {
        Range {
            start: value.start_point.into(),
            end: value.end_point.into(),
        }
    }
}

pub trait Contains<T> {
    fn contains(&self, other: T) -> bool;
}

impl Contains<&Point> for Range {
    fn contains(&self, other: &Point) -> bool {
        &self.start <= other && other < &self.end
    }
}

impl<'a> Contains<&Point> for Vec<ts::Node<'a>> {
    fn contains(&self, other: &Point) -> bool {
        self.iter().any(|n| Range::from(*n).contains(other))
    }
}
