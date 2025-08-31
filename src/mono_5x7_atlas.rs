use crate::atlas::FontAtlas;

/// **Danger**: leaking [`FontAtlas<'static>`] for the lifetime of the program
pub fn mono_5x7_atlas() -> ::embedded_graphics::mono_font::MonoFont<'static> {
    let atlas = FontAtlas::from("\0 \u{7f}\0\u{a0}ÿ\0₠\u{20cf}\0─╿\0▀▟\0☀⛿\0⠀⣿\0⬀⯿")
        .leak();

    ::embedded_graphics::mono_font::MonoFont {
        image: ::embedded_graphics::image::ImageRaw::new(
            include_bytes!("raw/mono_5x7.data"),
            0u32,
        ),
        glyph_mapping: atlas,
        character_size: ::embedded_graphics::geometry::Size::new(5u32, 7u32),
        character_spacing: 0u32,
        baseline: 5u32,
        underline: ::embedded_graphics::mono_font::DecorationDimensions::new(7u32, 1u32),
        strikethrough: ::embedded_graphics::mono_font::DecorationDimensions::new(3u32, 1u32),
    }
}