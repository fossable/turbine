use git2::{Repository, Oid, Commit};
use crate::currency::Address;
use std::error::Error;

/// 
pub struct TurbineRepo {
    /// Underlying git repository
    container: Repository,

    /// ID of the last commit we parsed
    last: Option<Oid>,
}

impl TurbineRepo {

    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            container: Repository::open(todo!())?,
            last: None,
        })
    }

    pub fn find_paid_commits(&mut self) -> Result<Vec<PaidCommit>, Box<dyn Error>> {
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
                        },
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
                        Err(_) => {},
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
    commit: Box<Commit>,
}

impl PaidCommit {

    pub fn try_parse(commit: Commit) -> Result<Self, Box<dyn Error>> {
        match commit.message() {
            Some(message) => {
                for line in message.split("\n") {
                    match line.split_once(":") {
                        Some((currency, rest)) => {
                            match Address::try_parse(currency.trim(), rest.trim()) {
                                Some(address) => {
                                    return Ok(Self {
                                        address,
                                        commit: Box::new(commit),
                                    });
                                }
                                None => (),
                            }
                        },
                        None => (),
                    }
                }
            }
            None => {
                debug!("Encountered invalid UTF-8 commit message");
            }
        }
        return Err("No currency line found".into());
    }
}
