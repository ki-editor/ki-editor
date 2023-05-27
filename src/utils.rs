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

pub fn consolidate_errors<T, E: std::fmt::Debug>(
    message: &str,
    results: Vec<Result<T, E>>,
) -> anyhow::Result<()> {
    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}: {:?}", message, errors))
    }
}
