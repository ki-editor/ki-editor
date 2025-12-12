use itertools::Itertools;
use unicode_width::UnicodeWidthStr;

use crate::utils::distribute_items;

pub(crate) enum FlexLayoutComponent {
    Text(String),
    Spacer,
}

/// Renders a collection of components into a string, distributing available space
/// equally between Spacer components to fill the specified width.
///
/// - `components`: Slice of Component enums to render
/// - `width`: Target width of the rendered output in character cells
///
/// Returns a String formatted according to the layout rules.
pub(crate) fn render_flex_layout(
    width: usize,
    separator: &str,
    components: &[FlexLayoutComponent],
) -> String {
    let width = width
        .saturating_sub(components.len().saturating_sub(1) * UnicodeWidthStr::width(separator));
    // Calculate the fixed width taken by Text components
    let fixed_width: usize = components
        .iter()
        .map(|comp| match comp {
            FlexLayoutComponent::Text(text) => UnicodeWidthStr::width(text.as_str()),
            FlexLayoutComponent::Spacer => 0,
        })
        .sum();

    // Count the number of spacers
    let spacer_count = components
        .iter()
        .filter(|comp| matches!(comp, FlexLayoutComponent::Spacer))
        .count();

    // Calculate remaining width to distribute among spacers
    let remaining_width = width.saturating_sub(fixed_width);

    // Distribute the remaining width among spacers
    let spacer_widths = distribute_items(remaining_width, spacer_count);

    // Create an iterator of spacer widths
    let mut spacer_iter = spacer_widths.into_iter();

    // Build the result by mapping each component to its string representation
    components
        .iter()
        .map(|component| match component {
            FlexLayoutComponent::Text(text) => text.clone(),
            FlexLayoutComponent::Spacer => " ".repeat(spacer_iter.next().unwrap_or(0)),
        })
        .join(separator)
}

#[cfg(test)]
mod test_render_flex_layout {
    use super::*;

    #[test]
    fn test_leading_spacer() {
        let components = [
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("Hello".to_string()),
        ];

        // Width = 10, "Hello" takes 5, so spacer should be 5 spaces
        let result = render_flex_layout(10, "", &components);
        assert_eq!(result, "     Hello");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 10);
    }

    #[test]
    fn test_middle_spacer() {
        let components = [
            FlexLayoutComponent::Text("Left".to_string()),
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("Right".to_string()),
        ];

        // Width = 15, "Left" takes 4, "Right" takes 5, so spacer should be 6 spaces
        let result = render_flex_layout(15, "", &components);
        assert_eq!(result, "Left      Right");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 15);
    }

    #[test]
    fn test_trailing_spacer() {
        let components = [
            FlexLayoutComponent::Text("Hello".to_string()),
            FlexLayoutComponent::Spacer,
        ];

        // Width = 10, "Hello" takes 5, so spacer should be 5 spaces
        let result = render_flex_layout(10, "", &components);
        assert_eq!(result, "Hello     ");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 10);
    }

    #[test]
    fn test_zero_spacer() {
        let components = [
            FlexLayoutComponent::Text("Hello".to_string()),
            FlexLayoutComponent::Text("World".to_string()),
        ];

        // Width = 10, which is exactly the width of "HelloWorld"
        let result = render_flex_layout(10, "", &components);
        assert_eq!(result, "HelloWorld");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 10);
    }

    #[test]
    fn test_multiple_spacers() {
        let components = [
            FlexLayoutComponent::Text("A".to_string()),
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("B".to_string()),
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("C".to_string()),
        ];

        // Width = 9, "A", "B", and "C" each take 1, so 6 spaces distributed between 2 spacers: 3 each
        let result = render_flex_layout(9, "", &components);
        assert_eq!(result, "A   B   C");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 9);

        // Test uneven distribution (10 width = 7 spaces between 2 spacers: should be 3 and 4)
        let result = render_flex_layout(10, "", &components);
        assert_eq!(result, "A   B    C");

        assert_eq!(UnicodeWidthStr::width(result.as_str()), 10);
    }

    #[test]
    fn test_not_enough_width() {
        let components = [
            FlexLayoutComponent::Text("Hello".to_string()),
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("World".to_string()),
        ];

        // Width = 9, which is less than the fixed components (10)
        // Spacer should get 0 width since there's no remaining space
        let result = render_flex_layout(9, "", &components);
        assert_eq!(result, "HelloWorld");

        // With even less width
        let result = render_flex_layout(5, "", &components);
        assert_eq!(result, "HelloWorld");

        // Join all parts to verify total width
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 10); // Still 10 despite requesting 5
    }

    #[test]
    fn test_unicode_characters() {
        let components = [
            FlexLayoutComponent::Text("你好".to_string()),
            FlexLayoutComponent::Spacer,
            FlexLayoutComponent::Text("世界".to_string()),
        ];

        // Each Chinese character typically has width 2, so fixed width is 8
        // With width 12, spacer should be 4 spaces
        let result = render_flex_layout(12, "", &components);
        assert_eq!(result, "你好    世界");

        assert_eq!(UnicodeWidthStr::width(result.as_str()), 12);
    }

    #[test]
    fn test_zero_width() {
        let components = [
            FlexLayoutComponent::Text("Hello".to_string()),
            FlexLayoutComponent::Spacer,
        ];

        // Width = 0, spacers should have 0 width
        let result = render_flex_layout(0, "", &components);
        assert_eq!(result, "Hello");
    }
}
