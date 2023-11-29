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

use anyhow::{bail, Context, Result};
use ini::Ini;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Repository {
    worktree_path: Box<Path>,
    git_dir_path: Box<Path>,
    config: Ini,
    // TODO config
}

impl Repository {
    pub fn new(path: impl Into<Box<Path>>) -> Result<Self> {
        let path = path.into();
        let mut git_dir_path = PathBuf::from(path.as_ref().clone());
        git_dir_path.push(".git");

        if !path.exists() {
            bail!("Git worktree directory does not exist")
        }

        if !git_dir_path.exists() {
            bail!("Git worktree directory does not exist")
        }

        let config_path = git_dir_path.join("config");

        let config = Ini::load_from_file(config_path).context("failed to load config.")?;
        if config
            .section(Some("core"))
            .context("No core section in config")?
            .get("repositoryformatversion")
            .context("no repositoryformatversion in core")?
            .parse::<i32>()
            .context("could not parse repository format")?
            != 0
        {
            bail!("invalid repository format. Only 0 is supported");
        }

        Ok(Self {
            worktree_path: path,
            git_dir_path: git_dir_path.into_boxed_path(),
            config,
        })
    }

    pub fn new_from_ref(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().clone();
        return Self::new(path);
    }

    pub fn create_at(path: impl Into<Box<Path>>) -> Result<Self> {
        let worktree_path: Box<Path> = path.into();
        let mut git_dir_path = worktree_path.clone().into_path_buf();
        git_dir_path.push(".git");
        let git_dir_path: Box<Path> = git_dir_path.into();

        // Verify that no repo exists at path
        if worktree_path.exists() {
            if !worktree_path.is_dir() {
                bail!("{worktree_path:?} already exists and is not a directory");
            }
            if git_dir_path.exists() {
                let mut dir_iter =
                    fs::read_dir(&git_dir_path).context("could not open .git dir")?;
                if dir_iter.next().is_some() {
                    bail!("{git_dir_path:?} is not empty")
                }
            }
        } else {
            fs::create_dir_all(&worktree_path).context("could not create workdir at {path:?}")?;
        }

        if !git_dir_path.exists() {
            fs::create_dir(&git_dir_path).context("could not create dir at {path:?}")?;
        }

        let repo = Self {
            worktree_path,
            git_dir_path,
            config: Self::default_config(),
        };

        repo.dir("branches", true).context("create new repo")?;
        repo.dir("objects", true).context("create new repo")?;
        repo.dir("refs/heads", true).context("create new repo")?;
        repo.dir("refs/tags", true).context("create new repo")?;

        repo.config()
            .write_to_file(repo.path("config"))
            .context("failed to write config")?;

        let mut open_opts = File::options();
        open_opts.create_new(true).write(true);

        writeln!(
            repo.file("description", &open_opts, true)
                .context("create new repo")?,
            "Unnamed repository; edit this file 'description' to name the repository."
        )
        .context("create new repo: description")?;

        writeln!(
            repo.file("HEAD", &open_opts, true)
                .context("create new repo")?,
            "ref: refs/heads/main"
        )
        .context("create new repo: HEAD")?;

        Ok(repo)
    }

    fn default_config() -> Ini {
        let mut config = Ini::new();
        config
            .with_section(Some("core"))
            .set("repositoryformatversion", "0")
            .set("filemode", "false")
            .set("bare", "false");
        config
    }

    pub fn worktree_root(&self) -> &Path {
        &self.worktree_path
    }

    pub fn gitdir_root(&self) -> &Path {
        &self.git_dir_path
    }

    pub fn config(&self) -> &Ini {
        &self.config
    }

    fn path(&self, path: impl AsRef<Path>) -> PathBuf {
        assert!(path.as_ref().is_relative());
        let mut res = self.git_dir_path.clone().into_path_buf();
        res.push(path);
        res
    }

    fn dir(&self, path: impl AsRef<Path>, mkdir: bool) -> Result<PathBuf> {
        let path = self.path(path);
        if path.exists() {
            if !path.is_dir() {
                bail!("expected dir found file")
            } else {
                Ok(path)
            }
        } else {
            if mkdir {
                fs::create_dir_all(&path).context("failed to create directory")?;
                Ok(path)
            } else {
                bail!("directory not found")
            }
        }
    }

    fn file(
        &self,
        path: impl AsRef<Path>,
        open_opts: &OpenOptions,
        create_parent: bool,
    ) -> Result<File> {
        let path = self.path(path);

        if create_parent {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).context("failed to create parent dir")?;
            } else {
                // parent dir is root. I don't think this can ever happen
                // unless maybe if a repo is created at the root of a drive,
                // although that should only count as a root on windows. Not sure
            }
        }

        open_opts.open(path).context("failed to open {path:?}")
    }

    pub fn worktree_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let mut res = self.worktree_path.clone().into_path_buf();
        res.push(path);
        res
    }
}

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod test {
    use test_dir::DirBuilder;

    use crate::test_utils::{existing_test_repo, test_dir};
    use crate::Repository;

    #[test]
    fn open_repository() {
        let test_dir = existing_test_repo("valid_empty");
        let repo = Repository::new(test_dir.root());
        println!("{repo:?}")
    }

    #[test]
    fn create_repository() {
        let repo_path = test_dir("create_repo");

        let repo = Repository::create_at(repo_path.root()).expect("could not create repo");

        drop(repo);
        Repository::new(repo_path.root()).expect("could not open nearly created repo");
    }
}
