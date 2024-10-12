use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{Device, Extent3d, Queue, Texture};
use image::RgbaImage;
use std::iter;
use std::mem::size_of;
use std::sync::mpsc::channel;

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
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
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
    device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission_index));

    receiver.recv().unwrap().unwrap();
    let buffer_slice = output_buffer.slice(..);
    let data = buffer_slice.get_mapped_range();
    let data = data
        .chunks_exact(buffer_dimensions.padded_bytes_per_row)
        .flat_map(|row| row.iter().take(buffer_dimensions.unpadded_bytes_per_row))
        .copied()
        .collect::<Vec<_>>();

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
}
