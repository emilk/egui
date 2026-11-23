use core::iter;
use egui_wgpu::wgpu::{Device, Extent3d, Queue, Texture};
use image::RgbaImage;
use std::sync::mpsc::channel;

use crate::wgpu::WAIT_TIMEOUT;

pub(crate) fn texture_to_image(device: &Device, queue: &Queue, texture: &Texture) -> RgbaImage {
    let buffer_dimensions =
        BufferDimensions::new(texture.width() as usize, texture.height() as usize);

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Texture to bytes output buffer"),
        size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture to bytes encoder"),
    });

    // Copy the data from the texture to the buffer
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(buffer_dimensions.padded_bytes_per_row as u32),
                rows_per_image: None,
            },
        },
        Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        },
    );

    let submission_index = queue.submit(iter::once(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = output_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| drop(sender.send(v)));

    // Poll the device in a blocking manner so that our future resolves.
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission_index),
            timeout: Some(WAIT_TIMEOUT),
        })
        .expect("Failed to poll device");

    receiver.recv().unwrap().unwrap();
    let buffer_slice = output_buffer.slice(..);
    let data = buffer_slice
        .get_mapped_range()
        .expect("Failed to get mapped range");
    let data = buffer_dimensions.unpad(&data);

    RgbaImage::from_raw(texture.width(), texture.height(), data).expect("Failed to create image")
}

struct BufferDimensions {
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }

    /// Copy the rows out of the padded buffer, dropping the padding.
    ///
    /// Copies whole rows: the mapped buffer can be uncached memory, where
    /// reading it byte by byte is slow.
    fn unpad(&self, data: &[u8]) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(self.unpadded_bytes_per_row * self.height);
        for row in data.chunks_exact(self.padded_bytes_per_row) {
            pixels.extend_from_slice(&row[..self.unpadded_bytes_per_row]);
        }
        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::BufferDimensions;

    #[test]
    fn unpad_drops_the_row_padding() {
        // 3 pixels of 4 bytes per row are padded to the copy alignment.
        let dimensions = BufferDimensions::new(3, 2);
        assert!(dimensions.padded_bytes_per_row > dimensions.unpadded_bytes_per_row);

        let data: Vec<u8> = (0..dimensions.padded_bytes_per_row * dimensions.height)
            .map(|i| (i % 251) as u8)
            .collect();

        let expected: Vec<u8> = data
            .chunks_exact(dimensions.padded_bytes_per_row)
            .flat_map(|row| row[..dimensions.unpadded_bytes_per_row].iter().copied())
            .collect();
        assert_eq!(dimensions.unpad(&data), expected);
        assert_eq!(expected.len(), 3 * 4 * 2);
    }
}
