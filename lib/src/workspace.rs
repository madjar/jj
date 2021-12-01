// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

use crate::repo::{ReadonlyRepo, RepoLoader};
use crate::settings::UserSettings;
use crate::working_copy::WorkingCopy;

#[derive(Error, Debug, PartialEq)]
pub enum WorkspaceInitError {
    #[error("The destination repo ({0}) already exists")]
    DestinationExists(PathBuf),
}

#[derive(Error, Debug, PartialEq)]
pub enum WorkspaceLoadError {
    #[error("There is no Jujutsu repo in {0}")]
    NoWorkspaceHere(PathBuf),
}

/// Represents a workspace, i.e. what's typically the .jj/ directory and its
/// parent.
pub struct Workspace {
    // Path to the workspace root (typically the parent of a .jj/ directory), which is where
    // working copy files live.
    workspace_root: PathBuf,
    repo_loader: RepoLoader,
    working_copy: WorkingCopy,
}

fn create_jj_dir(workspace_root: &Path) -> Result<PathBuf, WorkspaceInitError> {
    let jj_dir = workspace_root.join(".jj");
    if jj_dir.exists() {
        Err(WorkspaceInitError::DestinationExists(jj_dir))
    } else {
        std::fs::create_dir(&jj_dir).unwrap();
        Ok(jj_dir)
    }
}

fn init_working_copy(
    repo: &Arc<ReadonlyRepo>,
    workspace_root: &Path,
    jj_dir: &Path,
) -> WorkingCopy {
    let mut working_copy = WorkingCopy::init(
        repo.store().clone(),
        workspace_root.to_path_buf(),
        jj_dir.join("working_copy"),
    );
    let checkout_commit = repo.store().get_commit(repo.view().checkout()).unwrap();
    working_copy
        .check_out(checkout_commit)
        .expect("failed to check out root commit");
    working_copy
}

impl Workspace {
    pub fn init_local(
        user_settings: &UserSettings,
        workspace_root: PathBuf,
    ) -> Result<(Self, Arc<ReadonlyRepo>), WorkspaceInitError> {
        let jj_dir = create_jj_dir(&workspace_root)?;
        let repo = ReadonlyRepo::init_local(user_settings, jj_dir.clone());
        let working_copy = init_working_copy(&repo, &workspace_root, &jj_dir);
        let repo_loader = repo.loader();
        let workspace = Workspace {
            workspace_root,
            repo_loader,
            working_copy,
        };
        Ok((workspace, repo))
    }

    pub fn init_internal_git(
        user_settings: &UserSettings,
        workspace_root: PathBuf,
    ) -> Result<(Self, Arc<ReadonlyRepo>), WorkspaceInitError> {
        let jj_dir = create_jj_dir(&workspace_root)?;
        let repo = ReadonlyRepo::init_internal_git(user_settings, jj_dir.clone());
        let working_copy = init_working_copy(&repo, &workspace_root, &jj_dir);
        let repo_loader = repo.loader();
        let workspace = Workspace {
            workspace_root,
            repo_loader,
            working_copy,
        };
        Ok((workspace, repo))
    }

    pub fn init_external_git(
        user_settings: &UserSettings,
        workspace_root: PathBuf,
        git_repo_path: PathBuf,
    ) -> Result<(Self, Arc<ReadonlyRepo>), WorkspaceInitError> {
        let jj_dir = create_jj_dir(&workspace_root)?;
        let repo = ReadonlyRepo::init_external_git(user_settings, jj_dir.clone(), git_repo_path);
        let working_copy = init_working_copy(&repo, &workspace_root, &jj_dir);
        let repo_loader = repo.loader();
        let workspace = Workspace {
            workspace_root,
            repo_loader,
            working_copy,
        };
        Ok((workspace, repo))
    }

    pub fn load(
        user_settings: &UserSettings,
        workspace_path: PathBuf,
    ) -> Result<Self, WorkspaceLoadError> {
        let repo_path = find_repo_dir(&workspace_path)
            .ok_or(WorkspaceLoadError::NoWorkspaceHere(workspace_path))?;
        let workspace_root = repo_path.parent().unwrap().to_owned();
        let repo_loader = RepoLoader::init(user_settings, repo_path);
        let working_copy_state_path = repo_loader.repo_path().join("working_copy");
        let working_copy = WorkingCopy::load(
            repo_loader.store().clone(),
            workspace_root.clone(),
            working_copy_state_path,
        );
        Ok(Self {
            workspace_root,
            repo_loader,
            working_copy,
        })
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub fn repo_path(&self) -> &PathBuf {
        self.repo_loader.repo_path()
    }

    pub fn repo_loader(&self) -> &RepoLoader {
        &self.repo_loader
    }

    pub fn working_copy(&self) -> &WorkingCopy {
        &self.working_copy
    }

    pub fn working_copy_mut(&mut self) -> &mut WorkingCopy {
        &mut self.working_copy
    }
}

fn find_repo_dir(mut workspace_root: &Path) -> Option<PathBuf> {
    loop {
        let repo_path = workspace_root.join(".jj");
        if repo_path.is_dir() {
            return Some(repo_path);
        }
        if let Some(wc_dir_parent) = workspace_root.parent() {
            workspace_root = wc_dir_parent;
        } else {
            return None;
        }
    }
}