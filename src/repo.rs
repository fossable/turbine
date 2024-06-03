use crate::currency::Address;
use anyhow::{bail, Result};
use git2::{Commit, Oid, Repository};
use tempfile::TempDir;
use tracing::debug;

///
pub struct TurbineRepo {
    tmp: TempDir,

    /// Underlying git repository
    container: Repository,

    /// ID of the last commit we parsed
    last: Option<Oid>,
}

impl std::fmt::Debug for TurbineRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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
        })
    }

    pub fn find_paid_commits(&mut self) -> Result<Vec<PaidCommit>> {
        // Always fetch the repo first
        // TODO

        let mut revwalk = self.container.revwalk()?;

        // Catch up with the last commit
        match self.last {
            Some(last) => {
                loop {
                    match revwalk.next() {
                        Some(next) => {
                            if next?.as_bytes() == last.as_bytes() {
                                // Move past it
                                revwalk.next();
                                break;
                            }
                        }
                        None => {
                            // Ran out of commits before encountering "last" which
                            // means history has been changed
                            break;
                        }
                    }
                }
            }
            None => {
                // This is the first run and there's nothing to catch up with
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
