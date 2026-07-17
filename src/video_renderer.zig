const std = @import("std");

/// RGBピクセルバッファ(in_pixels)をターゲットのターミナルセルサイズにリサイズし、
/// 前景色(上半分)と背景色(下半分)のRGB値を抽出する。
/// 
/// `in_pixels`: 元画像のRGB8バッファ
/// `in_width`: 元画像の幅 (ピクセル単位)
/// `in_height`: 元画像の高さ (ピクセル単位)
/// `out_cells`: 出力バッファ。1セルあたり6バイト (R_fg, G_fg, B_fg, R_bg, G_bg, B_bg)。
///              長さは target_width * target_height * 6 であること。
/// `target_width`: 出力ターミナルエリアの幅 (セル数)
/// `target_height`: 出力ターミナルエリアの高さ (セル数)
export fn generate_terminal_cells(
    in_pixels: [*]const u8,
    in_width: u32,
    in_height: u32,
    out_cells: [*]u8,
    target_width: u32,
    target_height: u32,
) void {
    if (target_width == 0 or target_height == 0) return;
    if (in_width == 0 or in_height == 0) return;

    // 出力ピクセルサイズ: 幅 = target_width, 高さ = target_height * 2 (1セル = 縦2ピクセル)
    const out_pixel_width = target_width;
    const out_pixel_height = target_height * 2;

    const x_ratio: f32 = @as(f32, @floatFromInt(in_width)) / @as(f32, @floatFromInt(out_pixel_width));
    const y_ratio: f32 = @as(f32, @floatFromInt(in_height)) / @as(f32, @floatFromInt(out_pixel_height));

    var cy: u32 = 0;
    while (cy < target_height) : (cy += 1) {
        // セルの上半分 (Foreground)
        const py_top: u32 = cy * 2;
        const src_y_top: u32 = @intFromFloat(@as(f32, @floatFromInt(py_top)) * y_ratio);
        const clamped_src_y_top = if (src_y_top >= in_height) in_height - 1 else src_y_top;

        // セルの下半分 (Background)
        const py_bot: u32 = cy * 2 + 1;
        const src_y_bot: u32 = @intFromFloat(@as(f32, @floatFromInt(py_bot)) * y_ratio);
        const clamped_src_y_bot = if (src_y_bot >= in_height) in_height - 1 else src_y_bot;

        var cx: u32 = 0;
        while (cx < target_width) : (cx += 1) {
            const src_x: u32 = @intFromFloat(@as(f32, @floatFromInt(cx)) * x_ratio);
            const clamped_src_x = if (src_x >= in_width) in_width - 1 else src_x;

            const idx_top = (clamped_src_y_top * in_width + clamped_src_x) * 3;
            const r_fg = in_pixels[idx_top];
            const g_fg = in_pixels[idx_top + 1];
            const b_fg = in_pixels[idx_top + 2];

            const idx_bot = (clamped_src_y_bot * in_width + clamped_src_x) * 3;
            const r_bg = in_pixels[idx_bot];
            const g_bg = in_pixels[idx_bot + 1];
            const b_bg = in_pixels[idx_bot + 2];

            const out_idx = (cy * target_width + cx) * 6;
            out_cells[out_idx] = r_fg;
            out_cells[out_idx + 1] = g_fg;
            out_cells[out_idx + 2] = b_fg;
            out_cells[out_idx + 3] = r_bg;
            out_cells[out_idx + 4] = g_bg;
            out_cells[out_idx + 5] = b_bg;
        }
    }
}
