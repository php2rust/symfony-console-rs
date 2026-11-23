//! Port of Symfony\Component\Console\Helper\TableStyle.php.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/TableStyle.php

use crate::exception::{ConsoleError, Result};

/// `STR_PAD_LEFT`, `STR_PAD_RIGHT`, `STR_PAD_BOTH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadType {
    Left,
    Right,
    Both,
}

impl PadType {
    /// The PHP constant's value.
    pub fn value(self) -> i64 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Both => 2,
        }
    }

    pub fn from_value(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Both),
            _ => None,
        }
    }
}

/// `TableStyle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStyle {
    padding_char: String,
    horizontal_outside_border_char: String,
    horizontal_inside_border_char: String,
    vertical_outside_border_char: String,
    vertical_inside_border_char: String,
    crossing_char: String,
    crossing_top_right_char: String,
    crossing_top_mid_char: String,
    crossing_top_left_char: String,
    crossing_mid_right_char: String,
    crossing_bottom_right_char: String,
    crossing_bottom_mid_char: String,
    crossing_bottom_left_char: String,
    crossing_mid_left_char: String,
    crossing_top_left_bottom_char: String,
    crossing_top_mid_bottom_char: String,
    crossing_top_right_bottom_char: String,
    header_title_format: String,
    footer_title_format: String,
    cell_header_format: String,
    cell_row_format: String,
    cell_row_content_format: String,
    border_format: String,
    display_outside_border: bool,
    pad_type: PadType,
}

impl Default for TableStyle {
    fn default() -> Self {
        let plus = || "+".to_owned();
        Self {
            padding_char: " ".to_owned(),
            horizontal_outside_border_char: "-".to_owned(),
            horizontal_inside_border_char: "-".to_owned(),
            vertical_outside_border_char: "|".to_owned(),
            vertical_inside_border_char: "|".to_owned(),
            crossing_char: plus(),
            crossing_top_right_char: plus(),
            crossing_top_mid_char: plus(),
            crossing_top_left_char: plus(),
            crossing_mid_right_char: plus(),
            crossing_bottom_right_char: plus(),
            crossing_bottom_mid_char: plus(),
            crossing_bottom_left_char: plus(),
            crossing_mid_left_char: plus(),
            crossing_top_left_bottom_char: plus(),
            crossing_top_mid_bottom_char: plus(),
            crossing_top_right_bottom_char: plus(),
            header_title_format: "<fg=black;bg=white;options=bold> %s </>".to_owned(),
            footer_title_format: "<fg=black;bg=white;options=bold> %s </>".to_owned(),
            cell_header_format: "<info>%s</info>".to_owned(),
            cell_row_format: "%s".to_owned(),
            cell_row_content_format: " %s ".to_owned(),
            border_format: "%s".to_owned(),
            display_outside_border: true,
            pad_type: PadType::Right,
        }
    }
}

impl TableStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// `setPaddingChar($paddingChar)`: `!$paddingChar` refuses "" and "0".
    pub fn set_padding_char(&mut self, padding_char: &str) -> Result<&mut Self> {
        if padding_char.is_empty() || padding_char == "0" {
            return Err(ConsoleError::Logic(
                "The padding char must not be empty.".to_owned(),
            ));
        }
        self.padding_char = padding_char.to_owned();
        Ok(self)
    }

    pub fn get_padding_char(&self) -> &str {
        &self.padding_char
    }

    /// `setHorizontalBorderChars($outside, $inside)`.
    pub fn set_horizontal_border_chars(
        &mut self,
        outside: &str,
        inside: Option<&str>,
    ) -> &mut Self {
        self.horizontal_outside_border_char = outside.to_owned();
        self.horizontal_inside_border_char = inside.unwrap_or(outside).to_owned();
        self
    }

    /// `setVerticalBorderChars($outside, $inside)`.
    pub fn set_vertical_border_chars(&mut self, outside: &str, inside: Option<&str>) -> &mut Self {
        self.vertical_outside_border_char = outside.to_owned();
        self.vertical_inside_border_char = inside.unwrap_or(outside).to_owned();
        self
    }

    /// `getBorderChars()`: horizontal outside, vertical outside,
    /// horizontal inside, vertical inside.
    pub fn get_border_chars(&self) -> [&str; 4] {
        [
            &self.horizontal_outside_border_char,
            &self.vertical_outside_border_char,
            &self.horizontal_inside_border_char,
            &self.vertical_inside_border_char,
        ]
    }

    /// `setCrossingChars(...)`: the last three default to `$midLeft`,
    /// `$cross` and `$midRight`.
    #[allow(clippy::too_many_arguments)]
    pub fn set_crossing_chars(
        &mut self,
        cross: &str,
        top_left: &str,
        top_mid: &str,
        top_right: &str,
        mid_right: &str,
        bottom_right: &str,
        bottom_mid: &str,
        bottom_left: &str,
        mid_left: &str,
        top_left_bottom: Option<&str>,
        top_mid_bottom: Option<&str>,
        top_right_bottom: Option<&str>,
    ) -> &mut Self {
        self.crossing_char = cross.to_owned();
        self.crossing_top_left_char = top_left.to_owned();
        self.crossing_top_mid_char = top_mid.to_owned();
        self.crossing_top_right_char = top_right.to_owned();
        self.crossing_mid_right_char = mid_right.to_owned();
        self.crossing_bottom_right_char = bottom_right.to_owned();
        self.crossing_bottom_mid_char = bottom_mid.to_owned();
        self.crossing_bottom_left_char = bottom_left.to_owned();
        self.crossing_mid_left_char = mid_left.to_owned();
        self.crossing_top_left_bottom_char = top_left_bottom.unwrap_or(mid_left).to_owned();
        self.crossing_top_mid_bottom_char = top_mid_bottom.unwrap_or(cross).to_owned();
        self.crossing_top_right_bottom_char = top_right_bottom.unwrap_or(mid_right).to_owned();
        self
    }

    /// `setDefaultCrossingChar($char)`.
    pub fn set_default_crossing_char(&mut self, c: &str) -> &mut Self {
        self.set_crossing_chars(c, c, c, c, c, c, c, c, c, None, None, None)
    }

    pub fn get_crossing_char(&self) -> &str {
        &self.crossing_char
    }

    /// `getCrossingChars()`, in upstream's order.
    pub fn get_crossing_chars(&self) -> [&str; 12] {
        [
            &self.crossing_char,
            &self.crossing_top_left_char,
            &self.crossing_top_mid_char,
            &self.crossing_top_right_char,
            &self.crossing_mid_right_char,
            &self.crossing_bottom_right_char,
            &self.crossing_bottom_mid_char,
            &self.crossing_bottom_left_char,
            &self.crossing_mid_left_char,
            &self.crossing_top_left_bottom_char,
            &self.crossing_top_mid_bottom_char,
            &self.crossing_top_right_bottom_char,
        ]
    }

    pub fn set_cell_header_format(&mut self, format: &str) -> &mut Self {
        self.cell_header_format = format.to_owned();
        self
    }

    pub fn get_cell_header_format(&self) -> &str {
        &self.cell_header_format
    }

    pub fn set_cell_row_format(&mut self, format: &str) -> &mut Self {
        self.cell_row_format = format.to_owned();
        self
    }

    pub fn get_cell_row_format(&self) -> &str {
        &self.cell_row_format
    }

    pub fn set_cell_row_content_format(&mut self, format: &str) -> &mut Self {
        self.cell_row_content_format = format.to_owned();
        self
    }

    pub fn get_cell_row_content_format(&self) -> &str {
        &self.cell_row_content_format
    }

    pub fn set_border_format(&mut self, format: &str) -> &mut Self {
        self.border_format = format.to_owned();
        self
    }

    pub fn get_border_format(&self) -> &str {
        &self.border_format
    }

    /// `setPadType($padType)`.
    pub fn set_pad_type(&mut self, pad_type: i64) -> Result<&mut Self> {
        self.pad_type = PadType::from_value(pad_type).ok_or_else(|| {
            ConsoleError::InvalidArgument(
                "Invalid padding type. Expected one of (STR_PAD_LEFT, STR_PAD_RIGHT, STR_PAD_BOTH)."
                    .to_owned(),
            )
        })?;
        Ok(self)
    }

    pub fn get_pad_type(&self) -> PadType {
        self.pad_type
    }

    pub fn get_header_title_format(&self) -> &str {
        &self.header_title_format
    }

    pub fn set_header_title_format(&mut self, format: &str) -> &mut Self {
        self.header_title_format = format.to_owned();
        self
    }

    pub fn get_footer_title_format(&self) -> &str {
        &self.footer_title_format
    }

    pub fn set_footer_title_format(&mut self, format: &str) -> &mut Self {
        self.footer_title_format = format.to_owned();
        self
    }

    pub fn set_display_outside_border(&mut self, display: bool) -> &mut Self {
        self.display_outside_border = display;
        self
    }

    pub fn display_outside_border(&self) -> bool {
        self.display_outside_border
    }
}
