use crate::{
    app::Dispatch,
    lsp::{completion::CompletionItem, documentation::Documentation},
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

    pub fn to_completion_items(&self) -> Vec<CompletionItem> {
        [CompletionItem::from_label(self.name.to_string())
            .set_documentation(Some(Documentation::new(self.description)))]
        .into_iter()
        .chain(self.aliases.iter().map(|alias| {
            CompletionItem::from_label(alias.to_string())
                .set_documentation(Some(Documentation::new(self.description)))
        }))
        .collect()
    }
}

pub fn find(name: &str) -> Option<&'static Command> {
    commands().iter().find(|c| c.matches(name))
}

pub const fn commands() -> &'static [Command] {
    &[
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
    ]
}
