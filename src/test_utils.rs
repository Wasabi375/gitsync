use std::{path::PathBuf, time::SystemTime};

use fs_extra::dir::{self, CopyOptions};
use test_dir::{DirBuilder, TestDir};

pub fn test_dir(name: &str) -> TestDir {
    let mut path = PathBuf::from(name);
    if path.exists() {
        path = PathBuf::from(format!("{name}{}", get_sys_time_in_secs()));
    }

    TestDir::current(path.to_str().expect("could not create test dir"))
}

pub fn existing_test_repo(name: &str) -> TestDir {
    let mut existing = PathBuf::from("./test_data");
    existing.push(name);

    assert!(existing.exists() && existing.is_dir());

    let test_dir = test_dir(&format!("existing_{name}"));
    dir::copy(existing, test_dir.root(), &CopyOptions::new())
        .expect("Failed to copy existing repo to temp test location");

    for git_dir in dir::get_dir_content(test_dir.root())
        .expect("Failed to read created test dir")
        .directories
        .iter()
        .filter(|it| it.as_str() == "git-sync-repo")
    {
        let from = PathBuf::from(&git_dir);
        let mut to = from.parent().unwrap().to_path_buf();
        to.push(".git");
        dir::move_dir(from, to, &CopyOptions::new())
            .expect("Faileld to move git-sync-repo to .git in temp test dir");
    }

    test_dir
}

fn get_sys_time_in_secs() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
