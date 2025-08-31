pub const MONO_9X18_BOLD: ::embedded_graphics::mono_font::MonoFont = ::embedded_graphics::mono_font::MonoFont {
    image: ::embedded_graphics::image::ImageRaw::new(
        include_bytes!("raw/mono_9x18_bold.data"),
        144u32,
    ),
    glyph_mapping: &::embedded_graphics::mono_font::mapping::StrGlyphMapping::new(
        "\0 \u{7f}\0\u{a0}ÿ\0₠\u{20cf}\0─▟\0☀⛿\0⠀⣿\0⬀⯿",
        31usize,
    ),
    character_size: ::embedded_graphics::geometry::Size::new(9u32, 18u32),
    character_spacing: 0u32,
    baseline: 13u32,
    underline: ::embedded_graphics::mono_font::DecorationDimensions::new(15u32, 1u32),
    strikethrough: ::embedded_graphics::mono_font::DecorationDimensions::new(9u32, 1u32),
};
