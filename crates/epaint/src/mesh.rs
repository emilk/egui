use crate::*;
use emath::*;

/// The 2D vertex type.
///
/// Should be friendly to send to GPU as is.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(not(feature = "unity"))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Vertex {
    /// Logical pixel coordinates (points).
    /// (0,0) is the top left corner of the screen.
    pub pos: Pos2, // 64 bit

    /// Normalized texture coordinates.
    /// (0, 0) is the top left corner of the texture.
    /// (1, 1) is the bottom right corner of the texture.
    pub uv: Pos2, // 64 bit

    /// sRGBA with premultiplied alpha
    pub color: Color32, // 32 bit
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(feature = "unity")]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Vertex {
    /// Logical pixel coordinates (points).
    /// (0,0) is the top left corner of the screen.
    pub pos: Pos2, // 64 bit

    /// sRGBA with premultiplied alpha
    pub color: Color32, // 32 bit

    /// Normalized texture coordinates.
    /// (0, 0) is the top left corner of the texture.
    /// (1, 1) is the bottom right corner of the texture.
    pub uv: Pos2, // 64 bit
}

/// Textured triangles in two dimensions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Mesh {
    /// Draw as triangles (i.e. the length is always multiple of three).
    ///
    /// If you only support 16-bit indices you can use [`Mesh::split_to_u16`].
    ///
    /// egui is NOT consistent with what winding order it uses, so turn off backface culling.
    pub indices: Vec<u32>,

    /// The vertex data indexed by `indices`.
    pub vertices: Vec<Vertex>,

    /// The texture to use when drawing these triangles.
    pub texture_id: TextureId,
    // TODO(emilk): bounding rectangle
}

impl Mesh {
    pub fn with_texture(texture_id: TextureId) -> Self {
        Self {
            texture_id,
            ..Default::default()
        }
    }

    /// Restore to default state, but without freeing memory.
    pub fn clear(&mut self) {
        self.indices.clear();
        self.vertices.clear();
        self.vertices = Default::default();
    }

    pub fn bytes_used(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.vertices.len() * std::mem::size_of::<Vertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }

    /// Are all indices within the bounds of the contained vertices?
    pub fn is_valid(&self) -> bool {
        if let Ok(n) = u32::try_from(self.vertices.len()) {
            self.indices.iter().all(|&i| i < n)
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.vertices.is_empty()
    }

    /// Calculate a bounding rectangle.
    pub fn calc_bounds(&self) -> Rect {
        let mut bounds = Rect::NOTHING;
        for v in &self.vertices {
            bounds.extend_with(v.pos);
        }
        bounds
    }

    /// Append all the indices and vertices of `other` to `self`.
    pub fn append(&mut self, other: Mesh) {
        crate::epaint_assert!(other.is_valid());

        if self.is_empty() {
            *self = other;
        } else {
            self.append_ref(&other);
        }
    }

    /// Append all the indices and vertices of `other` to `self` without
    /// taking ownership.
    pub fn append_ref(&mut self, other: &Mesh) {
        crate::epaint_assert!(other.is_valid());

        if !self.is_empty() {
            assert_eq!(
                self.texture_id, other.texture_id,
                "Can't merge Mesh using different textures"
            );
        } else {
            self.texture_id = other.texture_id;
        }

        let index_offset = self.vertices.len() as u32;
        self.indices
            .extend(other.indices.iter().map(|index| index + index_offset));
        self.vertices.extend(other.vertices.iter());
    }

    #[inline(always)]
    pub fn colored_vertex(&mut self, pos: Pos2, color: Color32) {
        crate::epaint_assert!(self.texture_id == TextureId::default());
        self.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color,
        });
    }

    /// Add a triangle.
    #[inline(always)]
    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    /// Make room for this many additional triangles (will reserve 3x as many indices).
    /// See also `reserve_vertices`.
    #[inline(always)]
    pub fn reserve_triangles(&mut self, additional_triangles: usize) {
        self.indices.reserve(3 * additional_triangles);
    }

    /// Make room for this many additional vertices.
    /// See also `reserve_triangles`.
    #[inline(always)]
    pub fn reserve_vertices(&mut self, additional: usize) {
        self.vertices.reserve(additional);
    }

    /// Rectangle with a texture and color.
    pub fn add_rect_with_uv(&mut self, rect: Rect, uv: Rect, color: Color32) {
        #![allow(clippy::identity_op)]

        let idx = self.vertices.len() as u32;
        self.add_triangle(idx + 0, idx + 1, idx + 2);
        self.add_triangle(idx + 2, idx + 1, idx + 3);

        self.vertices.push(Vertex {
            pos: rect.left_top(),
            uv: uv.left_top(),
            color,
        });
        self.vertices.push(Vertex {
            pos: rect.right_top(),
            uv: uv.right_top(),
            color,
        });
        self.vertices.push(Vertex {
            pos: rect.left_bottom(),
            uv: uv.left_bottom(),
            color,
        });
        self.vertices.push(Vertex {
            pos: rect.right_bottom(),
            uv: uv.right_bottom(),
            color,
        });
    }

    /// Uniformly colored rectangle.
    #[inline(always)]
    pub fn add_colored_rect(&mut self, rect: Rect, color: Color32) {
        crate::epaint_assert!(self.texture_id == TextureId::default());
        self.add_rect_with_uv(rect, [WHITE_UV, WHITE_UV].into(), color);
    }

    /// This is for platforms that only support 16-bit index buffers.
    ///
    /// Splits this mesh into many smaller meshes (if needed)
    /// where the smaller meshes have 16-bit indices.
    pub fn split_to_u16(self) -> Vec<Mesh16> {
        crate::epaint_assert!(self.is_valid());

        const MAX_SIZE: u32 = std::u16::MAX as u32;

        if self.vertices.len() <= MAX_SIZE as usize {
            // Common-case optimization:
            return vec![Mesh16 {
                indices: self.indices.iter().map(|&i| i as u16).collect(),
                vertices: self.vertices,
                texture_id: self.texture_id,
            }];
        }

        let mut output = vec![];
        let mut index_cursor = 0;

        while index_cursor < self.indices.len() {
            let span_start = index_cursor;
            let mut min_vindex = self.indices[index_cursor];
            let mut max_vindex = self.indices[index_cursor];

            while index_cursor < self.indices.len() {
                let (mut new_min, mut new_max) = (min_vindex, max_vindex);
                for i in 0..3 {
                    let idx = self.indices[index_cursor + i];
                    new_min = new_min.min(idx);
                    new_max = new_max.max(idx);
                }

                let new_span_size = new_max - new_min + 1; // plus one, because it is an inclusive range
                if new_span_size <= MAX_SIZE {
                    // Triangle fits
                    min_vindex = new_min;
                    max_vindex = new_max;
                    index_cursor += 3;
                } else {
                    break;
                }
            }

            assert!(
                index_cursor > span_start,
                "One triangle spanned more than {} vertices",
                MAX_SIZE
            );

            let mesh = Mesh16 {
                indices: self.indices[span_start..index_cursor]
                    .iter()
                    .map(|vi| u16::try_from(vi - min_vindex).unwrap())
                    .collect(),
                vertices: self.vertices[(min_vindex as usize)..=(max_vindex as usize)].to_vec(),
                texture_id: self.texture_id,
            };
            crate::epaint_assert!(mesh.is_valid());
            output.push(mesh);
        }
        output
    }

    /// Translate location by this much, in-place
    pub fn translate(&mut self, delta: Vec2) {
        for v in &mut self.vertices {
            v.pos += delta;
        }
    }

    /// Rotate by some angle about an origin, in-place.
    ///
    /// Origin is a position in screen space.
    pub fn rotate(&mut self, rot: Rot2, origin: Pos2) {
        for v in &mut self.vertices {
            v.pos = origin + rot * (v.pos - origin);
        }
    }
}

// ----------------------------------------------------------------------------

/// A version of [`Mesh`] that uses 16-bit indices.
///
/// This is produced by [`Mesh::split_to_u16`] and is meant to be used for legacy render backends.
pub struct Mesh16 {
    /// Draw as triangles (i.e. the length is always multiple of three).
    ///
    /// egui is NOT consistent with what winding order it uses, so turn off backface culling.
    pub indices: Vec<u16>,

    /// The vertex data indexed by `indices`.
    pub vertices: Vec<Vertex>,

    /// The texture to use when drawing these triangles.
    pub texture_id: TextureId,
}

impl Mesh16 {
    /// Are all indices within the bounds of the contained vertices?
    pub fn is_valid(&self) -> bool {
        if let Ok(n) = u16::try_from(self.vertices.len()) {
            self.indices.iter().all(|&i| i < n)
        } else {
            false
        }
    }
}
