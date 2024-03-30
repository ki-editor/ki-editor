use crate::{
    app::{Dispatch, Dispatches},
    components::{dropdown::DropdownItem, suggestive_editor::Info},
};

pub struct Command {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    dispatch: Dispatch,
}
impl Command {
    pub fn dispatch(&self) -> Dispatch {
        self.dispatch.clone()
    }

    pub fn matches(&self, name: &str) -> bool {
        self.aliases.contains(&name) || self.name == name
    }

    pub fn to_dropdown_items(&self) -> Vec<DropdownItem> {
        [
            DropdownItem::new(self.name.to_string()).set_info(Some(Info::new(
                "Description".to_string(),
                self.description.to_string(),
            ))),
        ]
        .into_iter()
        .chain(self.aliases.iter().map(|alias| {
            DropdownItem::new(alias.to_string()).set_info(Some(Info::new(
                "Description".to_string(),
                self.description.to_string(),
            )))
        }))
        .map(|item| item.set_dispatches(Dispatches::one(self.dispatch.clone())))
        .collect()
    }
}

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.matches(name))
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "quit-all",
        aliases: &["qa"],
        description: "Quit the editor",
        dispatch: Dispatch::QuitAll,
    },
    Command {
        name: "write-quit-all",
        aliases: &["wqa"],
        description: "Save all buffers and quite the editor",
        dispatch: Dispatch::SaveQuitAll,
    },
];
