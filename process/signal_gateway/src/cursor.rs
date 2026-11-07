/// Rotate `items` so the element at `cursor` comes first.
pub fn rotation<T>(items: &[T], cursor: usize) -> Vec<&T> {
    if items.is_empty() {
        return Vec::new();
    }
    let start = cursor % items.len();
    items[start..].iter().chain(items[..start].iter()).collect()
}

/// Next cursor after a full pass.
pub fn advance(cursor: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        cursor.wrapping_add(1) % len
    }
}

/// Wrap a cursor into `[0, len)` after the symbol list changes.
pub fn clamp(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { cursor % len }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_starts_at_cursor() {
        let items = ["a", "b", "c"];
        assert_eq!(rotation(&items, 0), vec![&"a", &"b", &"c"]);
        assert_eq!(rotation(&items, 1), vec![&"b", &"c", &"a"]);
        assert_eq!(rotation(&items, 2), vec![&"c", &"a", &"b"]);
        assert_eq!(rotation(&items, 3), rotation(&items, 0));
        assert_eq!(rotation(&items, 4), rotation(&items, 1));
    }

    #[test]
    fn empty_rotation_is_empty() {
        let items: [&str; 0] = [];
        assert!(rotation(&items, 3).is_empty());
    }

    #[test]
    fn advance_wraps() {
        assert_eq!(advance(0, 3), 1);
        assert_eq!(advance(2, 3), 0);
        assert_eq!(advance(5, 3), 0);
        assert_eq!(advance(5, 0), 0);
        assert_eq!(advance(0, 1), 0);
        assert_eq!(advance(usize::MAX, 3), 0);
    }

    #[test]
    fn clamp_handles_shrinking_lists() {
        assert_eq!(clamp(5, 3), 2);
        assert_eq!(clamp(1, 3), 1);
        assert_eq!(clamp(5, 0), 0);
        assert_eq!(clamp(4, 3), 1);
        assert_eq!(clamp(0, 1), 0);
    }
}
