//! Port of Symfony\Component\Console\Attribute\AsCommand.php.
//!
//! PHP reads the attribute by reflection when a command is built; here
//! a command states it as a value (`CommandHandler::attribute()`), which a
//! converter writes from the attribute at build time.
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Attribute/AsCommand.php

/// `#[AsCommand]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AsCommand {
    /// The name, with the aliases and the hidden marker folded in
    /// (`"|name|alias"`), as the constructor leaves it.
    pub name: String,
    pub description: Option<String>,
    pub help: Option<String>,
    pub usages: Vec<String>,
}

impl AsCommand {
    /// `new AsCommand($name)`.
    pub fn new(name: &str) -> Self {
        Self::with(name, None, &[], false, None, &[])
    }

    /// `new AsCommand($name, $description, $aliases, $hidden, $help, $usages)`.
    pub fn with(
        name: &str,
        description: Option<&str>,
        aliases: &[&str],
        hidden: bool,
        help: Option<&str>,
        usages: &[&str],
    ) -> Self {
        let name = if !hidden && aliases.is_empty() {
            name.to_owned()
        } else {
            let mut names: Vec<&str> = name.split('|').collect();
            names.extend_from_slice(aliases);
            if hidden && !names[0].is_empty() {
                names.insert(0, "");
            }
            names.join("|")
        };
        Self {
            name,
            description: description.map(str::to_owned),
            help: help.map(str::to_owned),
            usages: usages.iter().map(|u| (*u).to_owned()).collect(),
        }
    }
}
