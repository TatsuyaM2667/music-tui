extern "C" {
    fn generate_braille_cells(
        in_pixels: *const u8,
        in_width: u32,
        in_height: u32,
        out_cells: *mut u8,
        target_width: u32,
        target_height: u32,
    );
}

pub struct OdinVideoFrame {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>, // 12 bytes per cell: u32(char) + fg(r,g,b) + bg(r,g,b) + pad(2)
}

pub fn render_raw_rgb_to_cells(pixels: &[u8], in_width: u16, in_height: u16, target_width: u16, target_height: u16) -> Option<OdinVideoFrame> {
    if target_width == 0 || target_height == 0 || in_width == 0 || in_height == 0 {
        return None;
    }
    if pixels.len() < (in_width as usize) * (in_height as usize) * 3 {
        return None;
    }

    let capacity = (target_width as usize) * (target_height as usize) * 12;
    let mut out_buffer = vec![0u8; capacity];

    unsafe {
        generate_braille_cells(
            pixels.as_ptr(),
            in_width as u32,
            in_height as u32,
            out_buffer.as_mut_ptr(),
            target_width as u32,
            target_height as u32,
        );
    }

    Some(OdinVideoFrame {
        width: target_width,
        height: target_height,
        data: out_buffer,
    })
}
