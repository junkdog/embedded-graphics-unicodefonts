mod args;

use crate::args::Args;
use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use eg_font_converter::{FontConverter, MonoFontOutput};
use embedded_graphics::geometry::Size;
use embedded_graphics::mono_font::{DecorationDimensions, MonoFont};
use embedded_graphics_unicodefonts::atlas::NamedUnicodeBlock;
use std::ffi::OsStr;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use bdf_parser::{Encoding, Font};

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

    let mut glyphs: Vec<u32> = font.glyphs
        .iter()
        .map(|g| match g.encoding {
            Encoding::Standard(v) => v,
            Encoding::NonStandard(v) => v,
            Encoding::Unspecified => panic!("Unspecified encoding: {:?}", g),
        })
        .collect();
    glyphs.sort_unstable();
    
    let total_glyphs = glyphs.len();

    // join glyphs into ranges
    let glyph_layout: GlyphLayout = layout_glyphs(glyphs, args.gap_threshold, args.min_range_length);

    println!("Font Information: {}", args.input.display());
    println!("Font Properties:");
    println!("  Name: {}", font.metadata.name);
    println!("  Bounding box size: {}x{}", font.metadata.bounding_box.size.x, font.metadata.bounding_box.size.y);
    println!("  Bounding box offset: ({}, {})", font.metadata.bounding_box.offset.x, font.metadata.bounding_box.offset.y);
    println!("  Point size: {}", font.metadata.point_size);
    println!("  Resolution: {}x{} dpi", font.metadata.resolution.x, font.metadata.resolution.y);
    println!();
    
    println!("Character Coverage:");
    println!("  Total glyphs: {}", total_glyphs);
    println!("  Ranges: {}", glyph_layout.ranges.len());
    println!("  Gaps (wasted): {}", glyph_layout.skipped_chars_count);
    println!("  Single characters: {}", glyph_layout.singles.len());
    println!();
    
    if !glyph_layout.ranges.is_empty() {
        let total_range_chars: u32 = glyph_layout.ranges.iter()
            .map(|r| r.end() - r.start() + 1)
            .sum();
        println!("Unicode Ranges ({} chars total):", total_range_chars);
        
        // First range has no gap info
        if let Some(first_range) = glyph_layout.ranges.first() {
            let start_char = char::from_u32(*first_range.start()).unwrap_or('�');
            let end_char = char::from_u32(*first_range.end()).unwrap_or('�');
            println!("  U+{:04X}..U+{:04X} ({} chars) '{}' to '{}'", 
                first_range.start(), first_range.end(), 
                first_range.end() - first_range.start() + 1,
                start_char, end_char);
        }
        
        // Subsequent ranges show gap from previous range
        for window in glyph_layout.ranges.windows(2) {
            let prev_range = &window[0];
            let curr_range = &window[1];
            let gap = curr_range.start() - prev_range.end() - 1;
            
            let start_char = char::from_u32(*curr_range.start()).unwrap_or('�');
            let end_char = char::from_u32(*curr_range.end()).unwrap_or('�');
            
            println!("  U+{:04X}..U+{:04X} ({} chars) '{}' to '{}' (gap: {} chars)", 
                curr_range.start(), curr_range.end(), 
                curr_range.end() - curr_range.start() + 1,
                start_char, end_char, gap);
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

fn layout_glyphs(
    mut glyphs: Vec<u32>,
    max_gap_threshold: u32,
    min_range_length: usize
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
    
    GlyphLayout { ranges, singles, skipped_chars_count }
}

fn cmd_convert_font(args: Args) -> Result<()> {
    let output_path = if let Some(path) = args.output {
        path
    } else {
        args.input.file_stem()
            .and_then(OsStr::to_str)
            .map(normalize_font_name)
            .map(PathBuf::from)
            .ok_or_else(|| eyre!("Invalid input filename"))?
    };

    let mut blocks = DEFAULT_BLOCKS
        .iter()
        .map(|b| b.range())
        .chain(args.ranges)
        .collect::<Vec<RangeInclusive<_>>>();

    blocks.sort_unstable_by_key(|b| *b.start());

    let basename = args.input
        .file_stem()
        .and_then(OsStr::to_str)
        .map(normalize_font_name)
        .expect("input filename is valid UTF-8");

    let mapping_string = generate_ranges_string(&blocks);
    let font_output = convert_bdf(&basename, &args.input, blocks)?;
    let atlas_src = rust_font_atlas(&basename, &font_output, &mapping_string)?;

    save_font(basename, &output_path, atlas_src, font_output)?;

    Ok(())
}

fn save_font(
    name: String,
    output_path: &PathBuf,
    atlas_src: String,
    font_output: MonoFontOutput,
) -> Result<()> {
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
    }

    // save the .data file + standard .rs file
    font_output.save(output_path)?;

    // save the atlas .rs file
    let atlas_file_path = output_path.join(format!("{name}_atlas.rs"));
    std::fs::write(&atlas_file_path, atlas_src)?;

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
