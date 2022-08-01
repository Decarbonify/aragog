#![allow(clippy::option_if_let_else)]
use arangors::document::options::{InsertOptions, RemoveOptions, UpdateOptions};

#[derive(Clone, Debug)]
/// Struct defining some options for database `write` operations (create,
/// update, delete)
pub struct OperationOptions {
    /// Defines if aragog should wait for the operation to be written on disk
    ///
    /// If set on `true` the requests might be slower. By default, the
    /// collection behavior is picked
    pub wait_for_sync: Option<bool>,
    /// Defines if aragog should ignore the `ArangoDB` document revision system
    /// (`_rev` field)
    ///
    /// If set on `false` the requests might be slower. By default, `true` is
    /// used as it is the default `ArangoDB` behaviour
    pub ignore_revs: bool,
    /// Defines if the operation should ignore [`Record`] hooks. By default set
    /// to `true`
    ///
    /// [`Record`]: crate::Record
    pub ignore_hooks: bool,
}

impl OperationOptions {
    /// Sets the `wait_for_sync` value
    #[inline]
    #[must_use]
    pub const fn wait_for_sync(mut self, value: bool) -> Self {
        self.wait_for_sync = Some(value);
        self
    }

    /// Sets the `ignore_revs` value
    #[inline]
    #[must_use]
    pub const fn ignore_revs(mut self, value: bool) -> Self {
        self.ignore_revs = value;
        self
    }

    /// Sets the `ignore_hooks` value
    #[inline]
    #[must_use]
    pub const fn ignore_hooks(mut self, value: bool) -> Self {
        self.ignore_hooks = value;
        self
    }
}

impl Default for OperationOptions {
    fn default() -> Self {
        Self {
            wait_for_sync: None, // We keep it at None to use the collection value
            ignore_revs: true,
            ignore_hooks: false,
        }
    }
}

impl From<OperationOptions> for InsertOptions {
    fn from(option: OperationOptions) -> Self {
        let builder = Self::builder()
            .return_new(true) // TODO: allow customization on this option
            .return_old(false)
            .silent(false);
        if let Some(value) = option.wait_for_sync {
            builder.wait_for_sync(value).build()
        } else {
            builder.build()
        }
    }
}

impl From<OperationOptions> for UpdateOptions {
    fn from(option: OperationOptions) -> Self {
        let builder = Self::builder()
            .keep_null(true)
            .ignore_revs(option.ignore_revs)
            .return_new(true) // TODO: allow customization on this option
            .return_old(false)
            .silent(false);
        if let Some(value) = option.wait_for_sync {
            builder.wait_for_sync(value).build()
        } else {
            builder.build()
        }
    }
}

impl From<OperationOptions> for RemoveOptions {
    fn from(option: OperationOptions) -> Self {
        let builder = Self::builder().return_old(false).silent(true); // On deletion we don't need meta data
        if let Some(value) = option.wait_for_sync {
            builder.wait_for_sync(value).build()
        } else {
            builder.build()
        }
    }
}
