//! Collect statistics about what is being painted.

use crate::*;

/// Size of the elements in a vector/array.
#[derive(Clone, Copy, PartialEq)]
enum ElementSize {
    Unknown,
    Homogeneous(usize),
    Heterogenous,
}

impl Default for ElementSize {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Aggregate information about a bunch of allocations.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct AllocInfo {
    element_size: ElementSize,
    num_allocs: usize,
    num_elements: usize,
    num_bytes: usize,
}

impl<T> From<&[T]> for AllocInfo {
    fn from(slice: &[T]) -> Self {
        Self::from_slice(slice)
    }
}

impl std::ops::Add for AllocInfo {
    type Output = AllocInfo;
    fn add(self, rhs: AllocInfo) -> AllocInfo {
        use ElementSize::{Heterogenous, Homogeneous, Unknown};
        let element_size = match (self.element_size, rhs.element_size) {
            (Heterogenous, _) | (_, Heterogenous) => Heterogenous,
            (Unknown, other) | (other, Unknown) => other,
            (Homogeneous(lhs), Homogeneous(rhs)) if lhs == rhs => Homogeneous(lhs),
            _ => Heterogenous,
        };

        AllocInfo {
            element_size,
            num_allocs: self.num_allocs + rhs.num_allocs,
            num_elements: self.num_elements + rhs.num_elements,
            num_bytes: self.num_bytes + rhs.num_bytes,
        }
    }
}

impl std::ops::AddAssign for AllocInfo {
    fn add_assign(&mut self, rhs: AllocInfo) {
        *self = *self + rhs;
    }
}

impl AllocInfo {
    // pub fn from_shape(shape: &Shape) -> Self {
    //     match shape {
    //         Shape::Noop
    //         Shape::Vec(shapes) => Self::from_shapes(shapes)
    //         | Shape::Circle { .. }
    //         | Shape::LineSegment { .. }
    //         | Shape::Rect { .. } => Self::default(),
    //         Shape::Path { points, .. } => Self::from_slice(points),
    //         Shape::Text { galley, .. } => Self::from_galley(galley),
    //         Shape::Triangles(triangles) => Self::from_triangles(triangles),
    //     }
    // }

    pub fn from_galley(galley: &Galley) -> Self {
        Self::from_slice(galley.text.as_bytes()) + Self::from_slice(&galley.rows)
    }

    pub fn from_triangles(triangles: &Triangles) -> Self {
        Self::from_slice(&triangles.indices) + Self::from_slice(&triangles.vertices)
    }

    pub fn from_slice<T>(slice: &[T]) -> Self {
        use std::mem::size_of;
        let element_size = size_of::<T>();
        Self {
            element_size: ElementSize::Homogeneous(element_size),
            num_allocs: 1,
            num_elements: slice.len(),
            num_bytes: slice.len() * element_size,
        }
    }

    pub fn num_elements(&self) -> usize {
        assert!(self.element_size != ElementSize::Heterogenous);
        self.num_elements
    }
    pub fn num_allocs(&self) -> usize {
        self.num_allocs
    }
    pub fn num_bytes(&self) -> usize {
        self.num_bytes
    }

    pub fn megabytes(&self) -> String {
        megabytes(self.num_bytes())
    }

    pub fn format(&self, what: &str) -> String {
        if self.num_allocs() == 0 {
            format!("{:6} {:12}", 0, what)
        } else if self.num_allocs() == 1 {
            format!(
                "{:6} {:12}  {}       1 allocation",
                self.num_elements,
                what,
                self.megabytes()
            )
        } else if self.element_size != ElementSize::Heterogenous {
            format!(
                "{:6} {:12}  {}     {:3} allocations",
                self.num_elements(),
                what,
                self.megabytes(),
                self.num_allocs()
            )
        } else {
            format!(
                "{:6} {:12}  {}     {:3} allocations",
                "",
                what,
                self.megabytes(),
                self.num_allocs()
            )
        }
    }
}

/// Collected allocation statistics for shapes and meshes.
#[derive(Clone, Copy, Default)]
pub struct PaintStats {
    pub shapes: AllocInfo,
    pub shape_text: AllocInfo,
    pub shape_path: AllocInfo,
    pub shape_mesh: AllocInfo,
    pub shape_vec: AllocInfo,

    /// Number of separate clip rectangles
    pub jobs: AllocInfo,
    pub vertices: AllocInfo,
    pub indices: AllocInfo,
}

impl PaintStats {
    pub fn from_shapes(shapes: &[ClippedShape]) -> Self {
        let mut stats = Self::default();
        stats.shape_path.element_size = ElementSize::Heterogenous; // nicer display later
        stats.shape_vec.element_size = ElementSize::Heterogenous; // nicer display later

        stats.shapes = AllocInfo::from_slice(shapes);
        for ClippedShape(_, shape) in shapes {
            stats.add(shape);
        }
        stats
    }

    fn add(&mut self, shape: &Shape) {
        match shape {
            Shape::Vec(shapes) => {
                // self += PaintStats::from_shapes(&shapes); // TODO
                self.shapes += AllocInfo::from_slice(shapes);
                self.shape_vec += AllocInfo::from_slice(shapes);
                for shape in shapes {
                    self.add(shape);
                }
            }
            Shape::Noop | Shape::Circle { .. } | Shape::LineSegment { .. } | Shape::Rect { .. } => {
                Default::default()
            }
            Shape::Path { points, .. } => {
                self.shape_path += AllocInfo::from_slice(points);
            }
            Shape::Text { galley, .. } => {
                self.shape_text += AllocInfo::from_galley(galley);
            }
            Shape::Triangles(triangles) => {
                self.shape_mesh += AllocInfo::from_triangles(triangles);
            }
        }
    }

    pub fn with_paint_jobs(mut self, paint_jobs: &[crate::PaintJob]) -> Self {
        self.jobs += AllocInfo::from_slice(paint_jobs);
        for (_, indices) in paint_jobs {
            self.vertices += AllocInfo::from_slice(&indices.vertices);
            self.indices += AllocInfo::from_slice(&indices.indices);
        }
        self
    }

    // pub fn total(&self) -> AllocInfo {
    //     self.shapes
    //         + self.shape_text
    //         + self.shape_path
    //         + self.shape_mesh
    //         + self.jobs
    //         + self.vertices
    //         + self.indices
    // }
}

fn megabytes(size: usize) -> String {
    format!("{:.2} MB", size as f64 / 1e6)
}
