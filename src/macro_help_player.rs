use crate::components::keymap_legend::KeymapLegendConfig;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use event::KeyEvent;

#[derive(Clone, Debug, PartialEq)]
pub struct MacroHelpPlayer {
    description: String,
    all_keys: Vec<KeyEvent>,
    keymap_config: KeymapLegendConfig,
    current_step: usize,
    pub(crate) press_key_and_update: bool,
}

impl MacroHelpPlayer {
    pub fn new(
        description: String,
        all_keys: Vec<KeyEvent>,
        keymap_config: KeymapLegendConfig,
    ) -> Self {
        Self {
            description,
            all_keys,
            keymap_config,
            current_step: 0,
            press_key_and_update: false,
        }
    }

    pub fn for_press_phase(&self) -> Self {
        Self {
            press_key_and_update: true,
            ..self.clone()
        }
    }

    pub fn for_render_phase(&self) -> Self {
        Self {
            current_step: self.current_step + 1,
            press_key_and_update: false,
            ..self.clone()
        }
    }

    pub fn get_current_key(&self) -> Option<KeyEvent> {
        self.all_keys.get(self.current_step).cloned()
    }

    pub fn is_finished(&self) -> bool {
        self.current_step >= self.all_keys.len()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn current_step(&self) -> usize {
        self.current_step
    }

    pub fn total_steps(&self) -> usize {
        self.all_keys.len()
    }

    pub fn render_help_display(&self, available_width: u16, available_height: u16) -> String {
        let remaining_keys = &self.all_keys[self.current_step..];
        let descriptions: Vec<String> = remaining_keys
            .iter()
            .map(|key| {
                self.keymap_config
                    .keymaps()
                    .get(key)
                    .map_or_else(|| "[Unknown]".to_string(), |km| km.description.clone())
            })
            .collect();
        let key_strings: Vec<String> = remaining_keys
            .iter()
            .map(|key| format!("{:?}", key))
            .collect();

        let mut all_tables_output = String::new();
        let mut processed_steps = 0;
        let mut lines_used: u16 = 0;

        while processed_steps < remaining_keys.len() && lines_used < available_height {
            let mut current_width: u16 = 1;
            let mut highlighted_keys = Vec::new();
            let mut highlighted_descs = Vec::new();

            for i in processed_steps..remaining_keys.len() {
                let key_str = &key_strings[i];
                let desc = &descriptions[i];
                let col_width = std::cmp::max(key_str.len(), desc.len()) as u16 + 3;
                if current_width + col_width > available_width && !highlighted_keys.is_empty() {
                    break;
                }
                current_width += col_width;
                highlighted_keys.push(key_str.clone());
                highlighted_descs.push(desc.clone());
                processed_steps += 1;
            }

            if !highlighted_keys.is_empty() {
                if lines_used.saturating_add(5) > available_height {
                    break;
                }

                let mut table = Table::new();
                table
                    .add_row(highlighted_keys)
                    .add_row(highlighted_descs)
                    .load_preset(UTF8_FULL)
                    .apply_modifier(UTF8_ROUND_CORNERS);

                if !all_tables_output.is_empty() {
                    all_tables_output.push('\n');
                }
                all_tables_output.push_str(&table.to_string());
                lines_used += 5;
            } else {
                break;
            }
        }

        if processed_steps < remaining_keys.len() {
            all_tables_output.push_str(&format!(
                "\n... and {} more steps.",
                remaining_keys.len() - processed_steps
            ));
        }

        all_tables_output
    }
}
