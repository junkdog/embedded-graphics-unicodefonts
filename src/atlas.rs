use alloc::vec::Vec;
use core::ops;
use embedded_graphics::mono_font::mapping::GlyphMapping;

const ASCII_OFFSET: usize = 0x20;

impl GlyphMapping for FontAtlasData {
    fn index(&self, c: char) -> usize {
        self.find(c).unwrap_or(0)
    }
}

/// Named Unicode blocks with predefined ranges for font generation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamedUnicodeBlock {
    Ascii,            // 0x0020..=0x007F
    Latin1,           // 0x00A0..=0x00FF
    BlockElements,    // 0x2580..=0x259F
    BoxDrawing,       // 0x2500..=0x257F
    Miscellaneous,    // 0x2600..=0x26FF
    BraillePatterns,  // 0x2800..=0x28FF
    CurrencySymbols,  // 0x20A0..=0x20CF
    Dingbats,         // 0x2700..=0x27BF
    SymbolsAndArrows, // 0x2B00..=0x2BFF
    TransportAndMap,  // 0x01F680..=0x01F6FF
    Pictographs,      // 0x01F300..=0x01F5FF
}

/// Font atlas containing Unicode blocks and additional symbols for glyph lookup
pub struct FontAtlasData {
    blocks: Vec<UnicodeBlock>,
    other_symbols: Vec<char>, // sorted
}

impl NamedUnicodeBlock {
    /// Returns all available Unicode blocks
    pub const fn all() -> &'static [Self] {
        &[
            Self::Ascii,
            Self::Latin1,
            Self::BlockElements,
            Self::BoxDrawing,
            Self::Miscellaneous,
            Self::BraillePatterns,
            Self::CurrencySymbols,
            Self::Dingbats,
            Self::SymbolsAndArrows,
        ]
    }

    /// Returns the Unicode range for this block
    pub const fn range(&self) -> ops::RangeInclusive<char> {
        match self {
            Self::Ascii            => '\u{0020}'..='\u{007F}',
            Self::Latin1           => '\u{00A0}'..='\u{00FF}',
            Self::BoxDrawing       => '\u{2500}'..='\u{257F}',
            Self::BlockElements    => '\u{2580}'..='\u{259F}',
            Self::Miscellaneous    => '\u{2600}'..='\u{26FF}',
            Self::BraillePatterns  => '\u{2800}'..='\u{28FF}',
            Self::CurrencySymbols  => '\u{20A0}'..='\u{20CF}',
            Self::Dingbats         => '\u{2700}'..='\u{27BF}',
            Self::SymbolsAndArrows => '\u{2B00}'..='\u{2BFF}',
            Self::TransportAndMap  => '\u{1F680}'..='\u{1F6FF}',
            Self::Pictographs      => '\u{1F300}'..='\u{1F5FF}'
        }
    }

    /// Returns the number of characters in this block
    pub const fn len(&self) -> usize {
        let r = self.range();
        *r.end() as usize - *r.start() as usize + 1
    }

    /// Checks if the symbol is within this block's range
    pub fn contains(&self, symbol: char) -> bool {
        self.range().contains(&symbol)
    }
}


impl FontAtlasData {
    /// Creates a new font atlas from Unicode blocks and additional symbols
    pub fn new(blocks: &[NamedUnicodeBlock], other_symbols: &[char]) -> Self {
        let mut other_symbols = other_symbols.to_vec();
        other_symbols.sort_unstable();
        other_symbols.dedup();

        let unicode_blocks = into_unicode_blocks(&blocks);

        Self {
            blocks: unicode_blocks,
            other_symbols,
        }
    }

    /// Returns total number of glyphs in the atlas
    pub fn len(&self) -> usize {
        let block_len: usize = self
            .blocks
            .last()
            .map(|b| b.len() + b.base_offset as usize)
            .unwrap_or(0);

        block_len + self.other_symbols.len()
    }

    /// Iterates over all characters in the atlas
    pub fn iter(&self) -> impl Iterator<Item=char> + '_ {
        self.blocks.iter().flat_map(|b| b.iter()).chain(self.other_symbols.iter().copied())
    }

    /// Checks if the atlas contains the given symbol
    pub fn contains(&self, symbol: char) -> bool {
        self.blocks.iter().any(|b| b.contains(symbol))
            || self.other_symbols.binary_search(&symbol).is_ok()
    }

    /// Finds the glyph index for a character, returns None if not found
    pub fn find(&self, symbol: char) -> Option<usize> {
        if symbol.is_ascii() {
            return Some(symbol as usize - ASCII_OFFSET);
        }

        self.blocks
            .iter()
            .skip(1) // skip ASCII block, handled above
            .find_map(|b| b.try_index(symbol))
            .or_else(|| self.index_of_other_symbol(symbol))
    }

    fn index_of_other_symbol(&self, symbol: char) -> Option<usize> {
        match self.other_symbols.binary_search(&symbol) {
            Ok(idx) => {
                let other_symbols_offset: usize = self
                    .blocks
                    .last()
                    .map(|b| b.base_offset as usize + b.len())
                    .unwrap_or(0);

                Some(other_symbols_offset + idx)
            }
            Err(_) => None,
        }
    }
}

/// A Unicode block with base offset for glyph indexing
#[derive(Debug, Clone)]
pub struct UnicodeBlock {
    base_offset: u16,
    range: ops::RangeInclusive<char>,
}

impl UnicodeBlock {
    pub fn contains(&self, symbol: char) -> bool {
        self.range.contains(&symbol)
    }

    /// Returns the glyph index for the symbol if it's in this block
    pub fn try_index(&self, symbol: char) -> Option<usize> {
        if self.range.contains(&symbol) {
            let offset = symbol as usize - *self.range.start() as usize;
            Some(self.base_offset as usize + offset)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        *self.range.end() as usize - *self.range.start() as usize + 1
    }

    /// Iterates over all characters in the atlas
    pub fn iter(&self) -> impl Iterator<Item=char> + '_ {
        self.range.clone().into_iter()
    }
}

fn into_unicode_blocks(blocks: &[NamedUnicodeBlock]) -> Vec<UnicodeBlock> {
    let mut base_offset = 0u16;
    let mut result = Vec::with_capacity(blocks.len());

    let mut blocks = blocks.to_vec();
    blocks.sort();
    blocks.dedup();

    for block in blocks {
        let range = block.range();
        result.push(UnicodeBlock {
            base_offset,
            range: range.clone(),
        });

        base_offset += u16::try_from(block.len())
            .expect("char will always fit into u16 as all defined blocks are within BMP");
    }
    result
}
