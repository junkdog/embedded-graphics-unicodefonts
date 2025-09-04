use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct GlyphLayout {
    pub ranges: Vec<RangeInclusive<u32>>,
    pub singles: Vec<u32>,
    pub skipped_chars_count: u32,
}

pub fn layout_glyphs(
    glyphs: Vec<u32>,
    max_gap_threshold: u32,
    min_range_length: usize,
) -> GlyphLayout {
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

pub fn into_blocks(glyph_layout: &GlyphLayout) -> Vec<RangeInclusive<char>> {
    let blocks = glyph_layout.ranges.iter().map(|r| {
        let start_char = char::from_u32(*r.start()).unwrap_or('\u{FFFD}');
        let end_char = char::from_u32(*r.end()).unwrap_or('\u{FFFD}');
        start_char..=end_char
    });

    let singles = glyph_layout
        .singles
        .iter()
        .copied()
        .flat_map(char::from_u32)
        .map(|c| c..=c);

    // blocks.chain(singles).collect()
    blocks.collect()
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
