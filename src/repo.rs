use crate::currency::Address;
use anyhow::{bail, Result};
use git2::{Commit, Oid, Repository, Revwalk};
use tempfile::TempDir;
use tracing::debug;

pub struct Contributor {
    pub address: Address,
}

///
pub struct TurbineRepo {
    tmp: TempDir,

    /// Underlying git repository
    container: Repository,

    /// ID of the last commit we parsed
    last: Option<Oid>,

    contributors: Vec<Contributor>,
}

impl std::fmt::Debug for TurbineRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.tmp.fmt(f)
    }
}

/// Advance a `Revwalk` past the given `Oid`.
fn fast_forward(revwalk: &mut Revwalk, oid: &Oid) -> Result<bool> {
    loop {
        match revwalk.next() {
            Some(next) => {
                if next?.as_bytes() == oid.as_bytes() {
                    // Move past it
                    revwalk.next();
                    break Ok(true);
                }
            }
            None => {
                // Ran out of commits before encountering "last" which
                // probably means history has been changed
                break Ok(false);
            }
        }
    }
}

impl TurbineRepo {
    pub fn new(remote: &str) -> Result<Self> {
        let tmp = tempfile::tempdir()?;

        debug!(remote = remote, dest = ?tmp.path(), "Cloning repository");
        let container = Repository::clone(&remote, tmp.path())?;
        Ok(Self {
            tmp,
            container,
            last: None,
            contributors: vec![],
        })
    }

    /// Load the latest list of contributors.
    pub fn reload_contributors(&mut self) -> Result<()> {
        let mut revwalk = self.container.revwalk()?;

        // Catch up with the last commit if there is one
        if let Some(last) = self.last.as_ref() {
            if !fast_forward(&mut revwalk, last)? {
                self.contributors = Vec::new();
                revwalk = self.container.revwalk()?;
            }
        }

        // Now find all contributors
        loop {
            if let Some(next) = revwalk.next() {
                let commit = self.container.find_commit(next?)?;
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn find_paid_commits(&mut self) -> Result<Vec<PaidCommit>> {
        // Always fetch the repo first
        // TODO

        let mut revwalk = self.container.revwalk()?;

        // Catch up with the last commit if there is one
        if let Some(last) = self.last.as_ref() {
            if !fast_forward(&mut revwalk, last)? {
                todo!();
            }
        }

        // Find paid commits
        let mut commits: Vec<PaidCommit> = vec![];
        loop {
            match revwalk.next() {
                Some(next) => {
                    let commit = self.container.find_commit(next?)?;
                    match PaidCommit::try_parse(commit) {
                        Ok(paid_commit) => {
                            commits.push(paid_commit);
                        }
                        Err(_) => {}
                    }
                }
                None => {
                    break;
                }
            }
        }
        return Ok(commits);
    }
}

pub struct PaidCommit {
    address: Address,
    id: Oid,
}

impl PaidCommit {
    pub fn try_parse(commit: Commit) -> Result<Self> {
        match commit.message() {
            Some(message) => {
                for line in message.split("\n") {
                    match line.split_once(":") {
                        Some((currency, rest)) => {
                            match Address::try_parse(currency.trim(), rest.trim()) {
                                Some(address) => {
                                    return Ok(Self {
                                        address,
                                        id: commit.id(),
                                    });
                                }
                                None => (),
                            }
                        }
                        None => (),
                    }
                }
            }
            None => {
                debug!("Encountered invalid UTF-8 commit message");
            }
        }
        bail!("No currency line found");
    }
}
