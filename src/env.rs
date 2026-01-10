pub fn parse_env<T: Clone>(
    env_name: &'static str,
    choices: &[T],
    to_string: fn(&T) -> &str,
    default: T,
) -> T {
    let Ok(user_value) = std::env::var(env_name) else {
        return default;
    };
    choices
        .iter()
        .find(|choice| to_string(choice).to_uppercase() == user_value.to_uppercase())
        .unwrap_or_else(|| {
            let choice_names: Vec<String> = choices
                .iter()
                .map(|choice| format!("  * {}", to_string(choice)))
                .collect();
            let choice_list = choice_names.join("\n");
            panic!(
                "
{user_value:?} was not found. Please update your {env_name} environment variable.

Available choices:
{choice_list}"
            );
        })
        .clone()
}
