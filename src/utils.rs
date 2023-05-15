pub fn find_previous<T>(
    mut iter: impl Iterator<Item = T>,
    set_last_match_predicate: impl Fn(&T, &Option<T>) -> bool,
    break_predicate: impl Fn(&T) -> bool,
) -> Option<T> {
    let mut last_match = None;
    while let Some(match_) = iter.next() {
        if break_predicate(&match_) {
            break;
        }

        if set_last_match_predicate(&match_, &last_match) {
            last_match = Some(match_);
        }
    }
    last_match
}
