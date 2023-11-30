//! # GitSync
//!
//! A custom git implementation in rust.
//!
//! In the future this will be extended to also work as a secondary git
//! repoitory in the same git folder.  The idea is to run git and gitsync
//! simultaneously in the same folder, creating to repos.  .git and .gitsync.
//! git will be obviously work just like normall. gitsync will add a secondary
//! repo with autocommits every n minutes, as well as auto push pull to a
//! special remote repo. That way it can be used to autosync between my
//! computers, without something like Dropbox.

// #![deny(missing_docs)]
// TODO reenable missing docs as error
// TODO change out anyhow for thiserror

#[cfg(test)]
pub mod test_utils;

mod object;
pub use object::{Object, ObjectType};
mod repository;
pub use repository::Repository;
