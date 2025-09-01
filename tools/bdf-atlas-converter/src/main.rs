mod args;

use crate::args::Args;
use bdf_parser::{Encoding, Font};
use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use eg_font_converter::{FontConverter, MonoFontOutput};
use embedded_graphics::geometry::Size;
use embedded_graphics::mono_font::{DecorationDimensions, MonoFont};
use embedded_graphics_unicodefonts::atlas::NamedUnicodeBlock;
use std::ffi::OsStr;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

const DEFAULT_BLOCKS: &[NamedUnicodeBlock] = &[
    NamedUnicodeBlock::Ascii,
    NamedUnicodeBlock::Latin1,
    NamedUnicodeBlock::BoxDrawing,
    NamedUnicodeBlock::BlockElements,
    NamedUnicodeBlock::Miscellaneous,
    NamedUnicodeBlock::BraillePatterns,
    NamedUnicodeBlock::CurrencySymbols,
    NamedUnicodeBlock::SymbolsAndArrows,
];

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    if args.info {
        cmd_font_info(args)
    } else {
        cmd_convert_font(args)
    }
}

struct GlyphLayout {
    ranges: Vec<RangeInclusive<u32>>,
    singles: Vec<u32>,
    skipped_chars_count: u32,
}

fn cmd_font_info(args: Args) -> Result<()> {
    let font_content = std::fs::read_to_string(&args.input)?;
    let font = Font::parse(&font_content)?;

    let mut glyphs: Vec<u32> = iter_glyphs(&font).collect();
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

fn print_glyph_summary(
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

fn layout_glyphs(
    mut glyphs: Vec<u32>,
    max_gap_threshold: u32,
    min_range_length: usize,
) -> GlyphLayout {
    if glyphs.is_empty() {
        return GlyphLayout {
            ranges: Vec::new(),
            singles: Vec::new(),
            skipped_chars_count: 0,
        };
    }

    glyphs.sort_unstable();
    glyphs.dedup();

    let mut ranges = Vec::new();
    let mut singles = Vec::new();
    let mut skipped_chars_count = 0;

    let mut range_start = glyphs[0];
    let mut range_end = glyphs[0];

    for &glyph in &glyphs[1..] {
        if glyph <= range_end + 1 + max_gap_threshold {
            // Count characters we're skipping in the gap
            if glyph > range_end + 1 {
                skipped_chars_count += glyph - range_end - 1;
            }
            // Extend current range
            range_end = glyph;
        } else {
            // End current range and decide if it's a range or singles
            let range_length = (range_end - range_start + 1) as usize;
            if range_length >= min_range_length {
                ranges.push(range_start..=range_end);
            } else {
                // Add individual characters
                for code in range_start..=range_end {
                    singles.push(code);
                }
            }

            // Start new range
            range_start = glyph;
            range_end = glyph;
        }
    }

    // Handle the last range
    let range_length = (range_end - range_start + 1) as usize;
    if range_length >= min_range_length {
        ranges.push(range_start..=range_end);
    } else {
        for code in range_start..=range_end {
            singles.push(code);
        }
    }

    GlyphLayout {
        ranges,
        singles,
        skipped_chars_count,
    }
}

fn cmd_convert_font(args: Args) -> Result<()> {
    let output_path = if let Some(path) = args.output {
        path
    } else {
        args.input
            .file_stem()
            .and_then(OsStr::to_str)
            .map(normalize_font_name)
            .map(PathBuf::from)
            .ok_or_else(|| eyre!("Invalid input filename"))?
    };

    // Scan the font to get available glyphs
    let font_content = std::fs::read_to_string(&args.input)?;
    let font = Font::parse(&font_content)?;

    let available_glyphs: std::collections::HashSet<u32> = font
        .glyphs
        .iter()
        .map(|g| match g.encoding {
            Encoding::Standard(v) => v,
            Encoding::NonStandard(v) => v,
            Encoding::Unspecified => panic!("Unspecified encoding: {:?}", g),
        })
        .collect();

    // Get requested ranges (default blocks + command line ranges)
    let requested_ranges: Vec<RangeInclusive<char>> = DEFAULT_BLOCKS
        .iter()
        .map(|b| b.range())
        .chain(args.ranges)
        .collect();

    // Filter requested ranges to only include available glyphs
    let mut filtered_glyphs: Vec<u32> = Vec::new();
    for range in requested_ranges {
        for code_point in (*range.start() as u32)..=(*range.end() as u32) {
            if available_glyphs.contains(&code_point) {
                filtered_glyphs.push(code_point);
            }
        }
    }

    filtered_glyphs.sort_unstable();
    filtered_glyphs.dedup();

    let total_filtered_glyphs = filtered_glyphs.len();

    // Use layout algorithm to organize filtered glyphs into optimal ranges and singles
    let glyph_layout: GlyphLayout =
        layout_glyphs(filtered_glyphs, args.gap_threshold, args.min_range_length);

    // Convert to character ranges for font conversion
    let blocks = into_blocks(&glyph_layout);

    let mut basename = args
        .input
        .file_stem()
        .and_then(OsStr::to_str)
        .map(normalize_font_name)
        .expect("input filename is valid UTF-8");

    // Append suffix if provided
    if let Some(suffix) = &args.suffix {
        basename.push_str(suffix);
    }

    let mapping_string = generate_ranges_string(&blocks);
    let font_output = convert_bdf(&basename, &args.input, blocks)?;
    let atlas_src = rust_font_atlas(&basename, &font_output, &mapping_string)?;

    save_font(
        basename,
        &output_path,
        atlas_src,
        font_output,
        args.save_png,
    )?;

    // Print summary of what was generated
    println!("\nFont Generation Summary:");
    print_glyph_summary(
        &args.input,
        Some(&output_path),
        total_filtered_glyphs,
        &glyph_layout,
    );

    Ok(())
}

fn into_blocks(glyph_layout: &GlyphLayout) -> Vec<RangeInclusive<char>> {
    let mut blocks: Vec<RangeInclusive<char>> = glyph_layout
        .ranges
        .iter()
        .map(|r| {
            let start_char = char::from_u32(*r.start()).unwrap_or('\u{FFFD}');
            let end_char = char::from_u32(*r.end()).unwrap_or('\u{FFFD}');
            start_char..=end_char
        })
        .collect();

    // Add individual characters as single-character ranges
    for &glyph_code in &glyph_layout.singles {
        if let Some(ch) = char::from_u32(glyph_code) {
            blocks.push(ch..=ch);
        }
    }

    blocks.sort_unstable_by_key(|b| *b.start());
    blocks
}

fn save_font(
    name: String,
    output_path: &PathBuf,
    atlas_src: String,
    font_output: MonoFontOutput,
    save_png: bool,
) -> Result<()> {
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
    }

    // save the .data file + standard .rs file
    font_output.save(output_path)?;

    // save the atlas .rs file
    let atlas_file_path = output_path.join(format!("{name}_atlas.rs"));
    std::fs::write(&atlas_file_path, atlas_src)?;

    // save PNG if requested
    if save_png {
        let png_path = output_path.join(format!("{name}.png"));
        font_output
            .save_png(&png_path)
            .map_err(|e| eyre!("Failed to save PNG: {}", e))?;
    }

    Ok(())
}

fn convert_bdf(
    basename: &str,
    input: &Path,
    blocks: Vec<RangeInclusive<char>>,
) -> Result<MonoFontOutput> {
    let name = basename.to_ascii_uppercase();
    let mut converter = FontConverter::with_file(input, &name);
    for block in blocks {
        converter = converter.glyphs(block);
    }

    converter
        .missing_glyph_substitute(' ')
        .convert_mono_font()
        .map_err(|e| eyre!("{}", e))
}

fn generate_ranges_string(blocks: &[RangeInclusive<char>]) -> String {
    let mut result = String::new();

    for range in blocks {
        result.push('\0'); // Range start marker
        result.push(*range.start()); // Range start character
        result.push(*range.end()); // Range end character
    }

    result
}

fn rust_font_atlas(
    font_basename: &str,
    font_output: &MonoFontOutput,
    mapping_string: &str,
) -> Result<String> {
    let MonoFont {
        character_size:
            Size {
                width: character_width,
                height: character_height,
            },
        character_spacing,
        baseline,
        underline:
            DecorationDimensions {
                offset: underline_offset,
                height: underline_height,
            },
        strikethrough:
            DecorationDimensions {
                offset: strikethrough_offset,
                height: strikethrough_height,
            },
        ..
    } = font_output.as_font();

    let content = format!(
        r#"use crate::atlas::FontAtlas;

/// **Danger**: leaking [`FontAtlas<'static>`] for the lifetime of the program
pub fn {font_basename}_atlas() -> ::embedded_graphics::mono_font::MonoFont<'static> {{
    let atlas = FontAtlas::from({mapping_string:?})
        .leak();

    ::embedded_graphics::mono_font::MonoFont {{
        image: ::embedded_graphics::image::ImageRaw::new(
            include_bytes!("{font_basename}.data"),
            {texture_width}u32,
        ),
        glyph_mapping: atlas,
        character_size: ::embedded_graphics::geometry::Size::new({character_width}u32, {character_height}u32),
        character_spacing: {character_spacing}u32,
        baseline: {baseline}u32,
        underline: ::embedded_graphics::mono_font::DecorationDimensions::new({underline_offset}u32, {underline_height}u32),
        strikethrough: ::embedded_graphics::mono_font::DecorationDimensions::new({strikethrough_offset}u32, {strikethrough_height}u32),
    }}
}}"#,
        texture_width = character_width * 16
    );

    Ok(content)
}

fn normalize_font_name(filename: &str) -> String {
    let mut name = filename.to_string();

    if name.ends_with('B') {
        name.pop();
        name.push_str("_bold");
    } else if name.ends_with('O') {
        name.pop();
        name.push_str("_italic");
    }

    format!("mono_{}", name)
}

fn iter_glyphs(font: &Font) -> impl Iterator<Item = u32> + '_ {
    font.glyphs.iter().filter_map(|g| match g.encoding {
        Encoding::Standard(v) => Some(v),
        Encoding::NonStandard(v) => Some(v),
        Encoding::Unspecified => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_glyphs_no_gaps() {
        let glyphs = vec![10, 11, 12, 15, 16, 17, 18, 19, 20, 21, 22];
        let layout = layout_glyphs(glyphs, 0, 4);

        // Range 10-12 is only 3 chars, below min_range_length=4, so should be singles
        // Range 15-22 is 8 chars, meets min_range_length=4, so should be a range
        assert_eq!(layout.ranges.len(), 1);
        assert_eq!(layout.ranges[0], 15..=22);
        assert_eq!(layout.singles.len(), 3);
        assert!(layout.singles.contains(&10));
        assert!(layout.singles.contains(&11));
        assert!(layout.singles.contains(&12));
        assert_eq!(layout.skipped_chars_count, 0);
    }

    #[test]
    fn test_layout_glyphs_with_gaps() {
        let glyphs = vec![10, 12, 14, 16, 18, 20, 22];
        let layout = layout_glyphs(glyphs, 1, 4);

        // With gap_threshold=1, should bridge single gaps: 10,12,14,16,18,20,22
        assert_eq!(layout.ranges.len(), 1);
        assert_eq!(layout.ranges[0], 10..=22);
        assert_eq!(layout.singles.len(), 0);
        // Should skip: 11, 13, 15, 17, 19, 21 = 6 chars
        assert_eq!(layout.skipped_chars_count, 6);
    }

    #[test]
    fn test_layout_glyphs_min_range_length() {
        let glyphs = vec![10, 11, 12, 20, 21, 30];
        let layout = layout_glyphs(glyphs, 0, 4);

        // Ranges 10-12 and 20-21 are too short (< 4), should be singles
        assert_eq!(layout.ranges.len(), 0);
        assert_eq!(layout.singles.len(), 6);
        assert!(layout.singles.contains(&10));
        assert!(layout.singles.contains(&11));
        assert!(layout.singles.contains(&12));
        assert!(layout.singles.contains(&20));
        assert!(layout.singles.contains(&21));
        assert!(layout.singles.contains(&30));
    }

    #[test]
    fn test_layout_glyphs_mixed_ranges_singles() {
        let glyphs = vec![10, 11, 12, 13, 14, 15, 16, 17, 25, 26, 35];
        let layout = layout_glyphs(glyphs, 0, 4);

        // Should have one range 10-17 and singles 25, 26, 35
        assert_eq!(layout.ranges.len(), 1);
        assert_eq!(layout.ranges[0], 10..=17);
        assert_eq!(layout.singles.len(), 3);
        assert!(layout.singles.contains(&25));
        assert!(layout.singles.contains(&26));
        assert!(layout.singles.contains(&35));
    }

    #[test]
    fn test_layout_glyphs_large_gaps() {
        let glyphs = vec![10, 11, 12, 20, 21, 22];
        let layout = layout_glyphs(glyphs, 5, 3);

        // Gap between 12 and 20 is 7, but threshold is 5, so no bridge
        assert_eq!(layout.ranges.len(), 2);
        assert_eq!(layout.ranges[0], 10..=12);
        assert_eq!(layout.ranges[1], 20..=22);
        assert_eq!(layout.skipped_chars_count, 0);
    }

    #[test]
    fn test_layout_glyphs_bridge_large_gaps() {
        let glyphs = vec![10, 11, 12, 20, 21, 22];
        let layout = layout_glyphs(glyphs, 10, 3);

        // Gap between 12 and 20 is 7, threshold is 10, so should bridge
        assert_eq!(layout.ranges.len(), 1);
        assert_eq!(layout.ranges[0], 10..=22);
        assert_eq!(layout.singles.len(), 0);
        // Should skip chars: 13, 14, 15, 16, 17, 18, 19 = 7 chars
        assert_eq!(layout.skipped_chars_count, 7);
    }

    #[test]
    fn test_layout_glyphs_empty() {
        let glyphs = vec![];
        let layout = layout_glyphs(glyphs, 1, 8);

        assert_eq!(layout.ranges.len(), 0);
        assert_eq!(layout.singles.len(), 0);
        assert_eq!(layout.skipped_chars_count, 0);
    }

    #[test]
    fn test_layout_glyphs_single_char() {
        let glyphs = vec![42];
        let layout = layout_glyphs(glyphs, 1, 8);

        assert_eq!(layout.ranges.len(), 0);
        assert_eq!(layout.singles.len(), 1);
        assert_eq!(layout.singles[0], 42);
        assert_eq!(layout.skipped_chars_count, 0);
    }

    #[test]
    fn test_layout_glyphs_deduplication() {
        let glyphs = vec![10, 10, 11, 11, 12, 12, 13, 13];
        let layout = layout_glyphs(glyphs, 0, 4);

        // Should deduplicate and create one range
        assert_eq!(layout.ranges.len(), 1);
        assert_eq!(layout.ranges[0], 10..=13);
        assert_eq!(layout.singles.len(), 0);
    }
}
