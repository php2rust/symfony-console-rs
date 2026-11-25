//! Port of Symfony\Component\Console\Helper\HelperSet.php and
//! HelperInterface.php.
//!
//! A helper is shared (`Arc`), as a PHP object is. `set()` calls the
//! helper's `setHelperSet()`; the M1-B helpers keep nothing from it, and
//! `getHelperSet()` is not ported (`DescriptorHelper` and `QuestionHelper`,
//! which read their set, are M7 work).
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Upstream: symfony/console Helper/HelperSet.php Helper/HelperInterface.php

use std::{any::Any, sync::Arc};

use crate::exception::{ConsoleError, Result};

/// `HelperInterface`.
pub trait HelperInterface: Any + Send + Sync {
    /// `getName()`: the helper's canonical name.
    fn get_name(&self) -> &str;

    /// `setHelperSet($helperSet)`, called when the helper joins a set.
    fn set_helper_set(&self, _helper_set: &HelperSet) {}
}

impl dyn HelperInterface {
    /// The helper as its concrete class, as PHP code calls a helper's own
    /// methods on what `getHelper()` returns.
    pub fn downcast_ref<T: HelperInterface>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}

/// `HelperSet`.
#[derive(Clone, Default)]
pub struct HelperSet {
    helpers: Vec<(String, Arc<dyn HelperInterface>)>,
}

impl HelperSet {
    /// `new HelperSet($helpers)`: each with its alias, or none (an integer
    /// key in PHP).
    pub fn new(helpers: Vec<(Option<String>, Arc<dyn HelperInterface>)>) -> Self {
        let mut set = Self::default();
        for (alias, helper) in helpers {
            set.set(helper, alias.as_deref());
        }
        set
    }

    /// `set($helper, $alias)`.
    #[expect(clippy::needless_pass_by_value, reason = "upstream's signature")]
    pub fn set(&mut self, helper: Arc<dyn HelperInterface>, alias: Option<&str>) {
        let name = helper.get_name().to_owned();
        self.insert(name, Arc::clone(&helper));
        if let Some(alias) = alias {
            self.insert(alias.to_owned(), Arc::clone(&helper));
        }
        helper.set_helper_set(self);
    }

    /// `$this->helpers[$name] = $helper`: an existing key keeps its place.
    fn insert(&mut self, name: String, helper: Arc<dyn HelperInterface>) {
        match self.helpers.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = helper,
            None => self.helpers.push((name, helper)),
        }
    }

    /// `has($name)`.
    pub fn has(&self, name: &str) -> bool {
        self.helpers.iter().any(|(n, _)| n == name)
    }

    /// `get($name)`.
    pub fn get(&self, name: &str) -> Result<Arc<dyn HelperInterface>> {
        self.helpers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| Arc::clone(h))
            .ok_or_else(|| {
                ConsoleError::InvalidArgument(format!("The helper \"{name}\" is not defined."))
            })
    }

    /// `getIterator()`: names (and aliases) with their helpers, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<dyn HelperInterface>)> {
        self.helpers.iter().map(|(n, h)| (n.as_str(), h))
    }
}

impl std::fmt::Debug for HelperSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.helpers.iter().map(|(n, _)| n))
            .finish()
    }
}

/// PHPUnit's `assertEquals` on two sets compares their helpers: the same
/// names, each the same object.
impl PartialEq for HelperSet {
    fn eq(&self, other: &Self) -> bool {
        self.helpers.len() == other.helpers.len()
            && self
                .helpers
                .iter()
                .zip(&other.helpers)
                .all(|((a, x), (b, y))| a == b && Arc::ptr_eq(x, y))
    }
}
