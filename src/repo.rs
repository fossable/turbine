use std::process::{Command, Stdio};

use crate::currency::Address;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use git2::{Commit, Oid, Repository, Revwalk, Sort};
use tempfile::TempDir;
use tracing::{debug, trace};

pub struct Contributor {
    pub address: Address,
    pub last_payout: Option<DateTime<Utc>>,
    /// The user's GPG public key ID which must remain constant
    pub key_id: String,

    /// Paid commits
    pub commits: Vec<Oid>,
}

impl Contributor {
    pub fn compute_payout(&self, commit_id: Oid) -> u64 {
        todo!()
    }
}

///
pub struct TurbineRepo {
    /// The branch to track
    branch: String,

    tmp: TempDir,

    /// Underlying git repository
    container: Repository,

    /// ID of the last commit we parsed
    last: Option<Oid>,

    pub contributors: Vec<Contributor>,
}

impl std::fmt::Debug for TurbineRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.tmp.fmt(f)
    }
}

/// Verify a commit's GPG signature and return its key ID.
fn verify_signature(commit: &Commit) -> Result<String> {
    let output = Command::new("git")
        .arg("verify-commit")
        .arg("--raw")
        .arg(commit.id().to_string())
        .stdout(Stdio::piped())
        .output()?;

    for line in std::str::from_utf8(&output.stdout)?.lines() {
        if line.contains("GOODSIG") {
            return Ok(line.split_whitespace().nth(2).unwrap().into());
        }
    }

    // Get the commit's GPG signature
    // TODO
    // if let Some(header) = commit.raw_header() {
    //     if let Some((_, gpgsig)) = header.split_once("gpgsig") {
    //         // Verify signature
    //         // TODO
    //     }
    // }
    bail!("Failed to verify signature");
}

impl TurbineRepo {
    pub fn new(remote: &str, branch: &str) -> Result<Self> {
        let tmp = tempfile::tempdir()?;

        debug!(remote = remote, dest = ?tmp.path(), "Cloning repository");
        let container = Repository::clone(&remote, tmp.path())?;
        Ok(Self {
            branch: branch.into(),
            tmp,
            container,
            last: None,
            contributors: vec![],
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        // Always fetch the repo first
        // TODO

        let mut revwalk = self.container.revwalk()?;
        revwalk.set_sorting(Sort::REVERSE)?;

        if let Some(last) = self.last {
            revwalk.push(last)?;
        } else {
            let branch = self
                .container
                .find_branch(&self.branch, git2::BranchType::Local)?;
            let branch_ref = branch.into_reference();

            revwalk.push(branch_ref.target().unwrap())?;
        }

        // Search for new contributors and update existing contributors
        debug!("Refreshing contributor table");
        loop {
            if let Some(next) = revwalk.next() {
                let commit = self.container.find_commit(next?)?;

                if let Ok(key_id) = verify_signature(&commit) {
                    if let Some(message) = commit.message() {
                        if let Some((_, address)) = message.split_once("XMR:") {
                            if let Some(contributor) = self
                                .contributors
                                .iter_mut()
                                .find(|contributor| contributor.key_id == key_id)
                            {
                                contributor.address = Address::XMR(address.into());
                            } else {
                                self.contributors.push(Contributor {
                                    address: Address::XMR(address.into()),
                                    last_payout: None,
                                    key_id,
                                    commits: Vec::new(),
                                });
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        revwalk.reset()?;

        // Find paid commits
        loop {
            match revwalk.next() {
                Some(next) => {
                    let commit = self.container.find_commit(next?)?;
                    if let Ok(key_id) = verify_signature(&commit) {
                        if let Some(contributor) = self
                            .contributors
                            .iter_mut()
                            .find(|contributor| contributor.key_id == key_id)
                        {
                            contributor.commits.push(commit.id());
                        }
                    }

                    self.last = Some(commit.id());
                }
                None => {
                    break;
                }
            }
        }
        Ok(())
    }
}

// impl PaidCommit {
//     pub fn try_parse(commit: Commit) -> Result<Self> {
//         match commit.message() {
//             Some(message) => {
//                 for line in message.split("\n") {
//                     match line.split_once(":") {
//                         Some((currency, rest)) => {
//                             match Address::try_parse(currency.trim(), rest.trim()) {
//                                 Some(address) => {
//                                     return Ok(Self {
//                                         address,
//                                         id: commit.id(),
//                                     });
//                                 }
//                                 None => (),
//                             }
//                         }
//                         None => (),
//                     }
//                 }
//             }
//             None => {
//                 debug!("Encountered invalid UTF-8 commit message");
//             }
//         }
//         bail!("No currency line found");
//     }
// }
