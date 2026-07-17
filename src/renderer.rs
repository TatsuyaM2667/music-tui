use image::{DynamicImage, GenericImageView};

extern "C" {
    fn generate_terminal_cells(
        in_pixels: *const u8,
        in_width: u32,
        in_height: u32,
        out_cells: *mut u8,
        target_width: u32,
        target_height: u32,
    );
}

pub struct ZigVideoFrame {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

pub fn render_image_to_cells(img: &DynamicImage, target_width: u16, target_height: u16) -> Option<ZigVideoFrame> {
    if target_width == 0 || target_height == 0 {
        return None;
    }

    let in_width = img.width();
    let in_height = img.height();
    if in_width == 0 || in_height == 0 {
        return None;
    }

    let rgb = img.to_rgb8();
    let pixels = rgb.as_raw();

    let capacity = (target_width as usize) * (target_height as usize) * 6;
    let mut out_buffer = vec![0u8; capacity];

    unsafe {
        generate_terminal_cells(
            pixels.as_ptr(),
            in_width,
            in_height,
            out_buffer.as_mut_ptr(),
            target_width as u32,
            target_height as u32,
        );
    }

    Some(ZigVideoFrame {
        width: target_width,
        height: target_height,
        data: out_buffer,
    })
}

pub fn render_raw_rgb_to_cells(pixels: &[u8], in_width: u16, in_height: u16, target_width: u16, target_height: u16) -> Option<ZigVideoFrame> {
    if target_width == 0 || target_height == 0 || in_width == 0 || in_height == 0 {
        return None;
    }
    if pixels.len() < (in_width as usize) * (in_height as usize) * 3 {
        return None;
    }

    let capacity = (target_width as usize) * (target_height as usize) * 6;
    let mut out_buffer = vec![0u8; capacity];

    unsafe {
        generate_terminal_cells(
            pixels.as_ptr(),
            in_width as u32,
            in_height as u32,
            out_buffer.as_mut_ptr(),
            target_width as u32,
            target_height as u32,
        );
    }

    Some(ZigVideoFrame {
        width: target_width,
        height: target_height,
        data: out_buffer,
    })
}
