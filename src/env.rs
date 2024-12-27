pub(crate) fn parse_env<T: Clone>(
    env_name: &'static str,
    choices: &[T],
    to_string: fn(&T) -> &str,
    default: T,
) -> T {
    let Ok(user_value) = std::env::var(env_name) else {
        return default;
    };
    choices
        .into_iter()
        .find(|choice| to_string(choice) == user_value)
        .unwrap_or_else(|| {
            let choice_names: Vec<String> = choices
                .iter()
                .map(|choice| format!("  * {}", to_string(choice)))
                .collect();
            let choice_list = choice_names.join("\n");
            panic!(
                "
{:?} was not found. Please update your {env_name} environment variable.

Available choices:
{}",
                user_value, choice_list
            );
        })
        .clone()
}
