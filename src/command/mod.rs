use crate::{
    app::{Dispatch, Dispatches},
    components::{dropdown::DropdownItem, suggestive_editor::Info},
};

pub(crate) struct Command {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    dispatch: Dispatch,
}
impl Command {
    pub(crate) fn dispatch(&self) -> Dispatch {
        self.dispatch.clone()
    }

    pub(crate) fn matches(&self, name: &str) -> bool {
        self.aliases.contains(&name) || self.name == name
    }

    pub(crate) fn to_dropdown_items(&self) -> Vec<DropdownItem> {
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

pub(crate) fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.matches(name))
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "quit-all",
        aliases: &[],
        description: "Quit the editor",
        dispatch: Dispatch::QuitAll,
    },
    Command {
        name: "write-quit-all",
        aliases: &[],
        description: "Save all buffers and quite the editor",
        dispatch: Dispatch::SaveQuitAll,
    },
    Command {
        name: "write-all",
        aliases: &[],
        description: "Save all buffers",
        dispatch: Dispatch::SaveAll,
    },
];
