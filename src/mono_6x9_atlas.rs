use crate::atlas::FontAtlas;

/// **Danger**: leaking [`FontAtlas<'static>`] for the lifetime of the program
pub fn mono_6x9_atlas() -> ::embedded_graphics::mono_font::MonoFont<'static> {
    let atlas = FontAtlas::from("\0 \u{7f}\0\u{a0}ÿ\0₠\u{20cf}\0─╿\0▀▟\0☀⛿\0⠀⣿\0⬀⯿")
        .leak();

    ::embedded_graphics::mono_font::MonoFont {
        image: ::embedded_graphics::image::ImageRaw::new(
            include_bytes!("raw/mono_6x9.data"),
            0u32,
        ),
        glyph_mapping: atlas,
        character_size: ::embedded_graphics::geometry::Size::new(6u32, 9u32),
        character_spacing: 0u32,
        baseline: 6u32,
        underline: ::embedded_graphics::mono_font::DecorationDimensions::new(8u32, 1u32),
        strikethrough: ::embedded_graphics::mono_font::DecorationDimensions::new(4u32, 1u32),
    }
}