use crate::args::Args;
use crate::layout::{layout_glyphs, GlyphLayout};
use crate::utils::{extract_glyphs_from_font, load_font};
use color_eyre::Result;

pub fn cmd_font_info(args: Args) -> Result<()> {
    let font = load_font(&args.input)?;

    let mut glyphs = extract_glyphs_from_font(&font);

    // Check if glyphs are in sorted order BEFORE sorting
    let mut is_sorted = true;
    let mut last_ordered_code = None;
    let mut first_out_of_order_code = None;

    for window in glyphs.windows(2) {
        if window[0] > window[1] {
            is_sorted = false;
            last_ordered_code = Some(window[0]);
            first_out_of_order_code = Some(window[1]);
            break;
        }
    }

    println!("Original glyphs are sorted: {}", is_sorted);
    if let (Some(last), Some(first)) = (last_ordered_code, first_out_of_order_code) {
        println!(
            "Last ordered code point: U+{:04X}, First out-of-order: U+{:04X}",
            last, first
        );
    }

    glyphs.sort_unstable();
    let total_glyphs = glyphs.len();

    // join glyphs into ranges
    let glyph_layout: GlyphLayout =
        layout_glyphs(glyphs, args.gap_threshold, args.min_range_length);

    println!("Font Information: {}", args.input.display());
    println!("Font Properties:");
    println!("  Name: {}", font.metadata.name);
    println!(
        "  Bounding box size: {}x{}",
        font.metadata.bounding_box.size.x, font.metadata.bounding_box.size.y
    );
    println!(
        "  Bounding box offset: ({}, {})",
        font.metadata.bounding_box.offset.x, font.metadata.bounding_box.offset.y
    );
    println!("  Point size: {}", font.metadata.point_size);
    println!(
        "  Resolution: {}x{} dpi",
        font.metadata.resolution.x, font.metadata.resolution.y
    );
    println!();

    print_glyph_summary(&args.input, None, total_glyphs, &glyph_layout);
    println!();

    if !glyph_layout.ranges.is_empty() {
        let total_range_chars: u32 = glyph_layout
            .ranges
            .iter()
            .map(|r| r.end() - r.start() + 1)
            .sum();
        println!("Unicode Ranges ({} chars total):", total_range_chars);

        // First range has no gap info
        if let Some(first_range) = glyph_layout.ranges.first() {
            let start_char = char::from_u32(*first_range.start()).unwrap_or('�');
            let end_char = char::from_u32(*first_range.end()).unwrap_or('�');
            println!(
                "  U+{:04X}..U+{:04X} ({} chars) '{}' to '{}'",
                first_range.start(),
                first_range.end(),
                first_range.end() - first_range.start() + 1,
                start_char,
                end_char
            );
        }

        // Subsequent ranges show gap from previous range
        for window in glyph_layout.ranges.windows(2) {
            let prev_range = &window[0];
            let curr_range = &window[1];
            let gap = curr_range.start() - prev_range.end() - 1;

            let start_char = char::from_u32(*curr_range.start()).unwrap_or('�');
            let end_char = char::from_u32(*curr_range.end()).unwrap_or('�');

            println!(
                "  U+{:04X}..U+{:04X} ({} chars) '{}' to '{}' (gap: {} chars)",
                curr_range.start(),
                curr_range.end(),
                curr_range.end() - curr_range.start() + 1,
                start_char,
                end_char,
                gap
            );
        }
        println!();
    }

    if !glyph_layout.singles.is_empty() {
        println!("Individual Characters:");
        for (i, &code) in glyph_layout.singles.iter().enumerate() {
            if i > 0 && i % 8 == 0 {
                println!();
            }
            let ch = char::from_u32(code).unwrap_or('�');
            print!("  U+{:04X}('{}') ", code, ch);
        }
        println!();
    }

    Ok(())
}

pub fn print_glyph_summary(
    input_path: &std::path::Path,
    output_path: Option<&std::path::Path>,
    total_glyphs: usize,
    glyph_layout: &GlyphLayout,
) {
    let total_range_chars: u32 = glyph_layout
        .ranges
        .iter()
        .map(|r| r.end() - r.start() + 1)
        .sum();

    println!("Character Coverage:");
    println!("  Total glyphs: {}", total_glyphs);
    println!(
        "  Ranges: {} ({} chars total)",
        glyph_layout.ranges.len(),
        total_range_chars
    );
    println!("  Gaps (wasted): {}", glyph_layout.skipped_chars_count);
    println!("  Single characters: {}", glyph_layout.singles.len());

    if let Some(output) = output_path {
        println!("  Input: {}", input_path.display());
        println!("  Output: {}", output.display());
    }
}
