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

    let mut blocks = DEFAULT_BLOCKS
        .iter()
        .map(|b| b.range())
        .chain(args.ranges)
        .collect::<Vec<RangeInclusive<_>>>();

    blocks.sort_unstable_by_key(|b| *b.start());

    let basename = args
        .input
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
