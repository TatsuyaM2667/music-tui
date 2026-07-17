package main

// Dual-color Braille renderer: each cell gets BOTH foreground and background color.
// Foreground = average color of "lit" dots (luminance above threshold)
// Background = average color of "unlit" dots (luminance below threshold)
// This doubles the effective color richness compared to single-color Braille.
//
// Output layout: 12 bytes per cell
//   [0..3]  u32 LE  braille codepoint
//   [4..6]  u8 x3   foreground RGB
//   [7..9]  u8 x3   background RGB
//   [10..11] padding (zeroed)

@export
generate_braille_cells :: proc "c" (
    in_pixels: [^]u8,
    in_width: u32,
    in_height: u32,
    out_cells: [^]u8,
    target_width: u32,
    target_height: u32,
) {
    if target_width == 0 || target_height == 0 do return
    if in_width == 0 || in_height == 0 do return

    out_pixel_width := target_width * 2
    out_pixel_height := target_height * 4

    scale_x := f32(out_pixel_width) / f32(in_width)
    scale_y := f32(out_pixel_height) / f32(in_height)
    scale := scale_x if scale_x < scale_y else scale_y

    drawn_width := u32(f32(in_width) * scale)
    drawn_height := u32(f32(in_height) * scale)

    off_x := (out_pixel_width - drawn_width) / 2
    off_y := (out_pixel_height - drawn_height) / 2

    dot_flags := [4][2]u32{
        {0x01, 0x08},
        {0x02, 0x10},
        {0x04, 0x20},
        {0x40, 0x80},
    }

    for cy in 0..<target_height {
        for cx in 0..<target_width {
            braille_char : u32 = 0x2800

            // Foreground (lit dots) accumulators
            fg_r : u32 = 0
            fg_g : u32 = 0
            fg_b : u32 = 0
            fg_count : u32 = 0

            // Background (unlit dots) accumulators
            bg_r : u32 = 0
            bg_g : u32 = 0
            bg_b : u32 = 0
            bg_count : u32 = 0

            // First pass: compute average luminance for adaptive threshold
            lum_sum : f32 = 0.0
            pixel_count : u32 = 0
            for dy in 0..<4 {
                for dx in 0..<2 {
                    px := cx * 2 + u32(dx)
                    py := cy * 4 + u32(dy)
                    if px >= off_x && px < off_x + drawn_width && py >= off_y && py < off_y + drawn_height {
                        sx := u32(f32(px - off_x) / scale)
                        sy := u32(f32(py - off_y) / scale)
                        csx := sx if sx < in_width else in_width - 1
                        csy := sy if sy < in_height else in_height - 1
                        idx := (csy * in_width + csx) * 3
                        r := in_pixels[idx]
                        g := in_pixels[idx + 1]
                        b := in_pixels[idx + 2]
                        lum_sum += 0.299 * f32(r) + 0.587 * f32(g) + 0.114 * f32(b)
                        pixel_count += 1
                    }
                }
            }

            // Adaptive threshold: use mean luminance of the cell
            // Clamp to [30, 200] to avoid degenerate cases
            threshold : f32 = 80.0
            if pixel_count > 0 {
                avg_lum := lum_sum / f32(pixel_count)
                threshold = avg_lum
                if threshold < 30.0 do threshold = 30.0
                if threshold > 200.0 do threshold = 200.0
            }

            // Second pass: classify dots and accumulate colors
            for dy in 0..<4 {
                for dx in 0..<2 {
                    px := cx * 2 + u32(dx)
                    py := cy * 4 + u32(dy)

                    if px >= off_x && px < off_x + drawn_width && py >= off_y && py < off_y + drawn_height {
                        sx := u32(f32(px - off_x) / scale)
                        sy := u32(f32(py - off_y) / scale)
                        csx := sx if sx < in_width else in_width - 1
                        csy := sy if sy < in_height else in_height - 1

                        idx := (csy * in_width + csx) * 3
                        r := in_pixels[idx]
                        g := in_pixels[idx + 1]
                        b := in_pixels[idx + 2]

                        lum := 0.299 * f32(r) + 0.587 * f32(g) + 0.114 * f32(b)
                        if lum > threshold {
                            braille_char |= dot_flags[dy][dx]
                            fg_r += u32(r)
                            fg_g += u32(g)
                            fg_b += u32(b)
                            fg_count += 1
                        } else {
                            bg_r += u32(r)
                            bg_g += u32(g)
                            bg_b += u32(b)
                            bg_count += 1
                        }
                    }
                }
            }

            // Compute final colors
            final_fg_r : u8 = 0
            final_fg_g : u8 = 0
            final_fg_b : u8 = 0
            if fg_count > 0 {
                final_fg_r = u8(fg_r / fg_count)
                final_fg_g = u8(fg_g / fg_count)
                final_fg_b = u8(fg_b / fg_count)
            }

            final_bg_r : u8 = 0
            final_bg_g : u8 = 0
            final_bg_b : u8 = 0
            if bg_count > 0 {
                final_bg_r = u8(bg_r / bg_count)
                final_bg_g = u8(bg_g / bg_count)
                final_bg_b = u8(bg_b / bg_count)
            }

            // 12 bytes per cell
            out_idx := (cy * target_width + cx) * 12

            // Braille codepoint (LE u32)
            out_cells[out_idx + 0] = u8(braille_char & 0xFF)
            out_cells[out_idx + 1] = u8((braille_char >> 8) & 0xFF)
            out_cells[out_idx + 2] = u8((braille_char >> 16) & 0xFF)
            out_cells[out_idx + 3] = u8((braille_char >> 24) & 0xFF)

            // Foreground RGB
            out_cells[out_idx + 4] = final_fg_r
            out_cells[out_idx + 5] = final_fg_g
            out_cells[out_idx + 6] = final_fg_b

            // Background RGB
            out_cells[out_idx + 7] = final_bg_r
            out_cells[out_idx + 8] = final_bg_g
            out_cells[out_idx + 9] = final_bg_b

            // Padding
            out_cells[out_idx + 10] = 0
            out_cells[out_idx + 11] = 0
        }
    }
}
