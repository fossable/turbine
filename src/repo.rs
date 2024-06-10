use crate::currency::Address;
use anyhow::{bail, Result};
use base64::prelude::*;
use chrono::{DateTime, Utc};
use git2::{Commit, Oid, Repository, Revwalk, Sort};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use tracing::{debug, info, instrument, trace};

#[derive(Debug)]
pub struct Contributor {
    pub address: Address,
    pub last_payout: Option<DateTime<Utc>>,
    /// The user's GPG public key ID which must remain constant
    pub key_id: String,

    /// Paid commits
    pub commits: Vec<Oid>,
}

impl Contributor {
    #[instrument(ret)]
    pub fn compute_payout(&self, commit_id: Oid) -> u64 {
        // TODO
        1
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

    /// Last time we refreshed
    last_refresh: Option<DateTime<Utc>>,

    pub contributors: Vec<Contributor>,
}

impl std::fmt::Debug for TurbineRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.tmp.fmt(f)
    }
}

/// Get the key ID of the public key that corresponds to the private key that
/// signed this commit.
#[instrument(ret, level = "trace")]
fn get_public_key_id(commit: &Commit) -> Result<String> {
    if let Some(header) = commit.raw_header() {
        if let Some((_, gpgsig)) = header.split_once("gpgsig") {
            let mut signature_base64 = String::new();
            for line in gpgsig.lines() {
                let line = line.trim();
                if line.starts_with("-----BEGIN") {
                    continue;
                } else if line.starts_with("=") {
                    // Ascii armor checksum means we're done
                    break;
                } else {
                    signature_base64.push_str(&line);
                }
            }

            let decoded = BASE64_STANDARD.decode(signature_base64)?;
            return Ok(hex::encode(&decoded[12..32]));
        }
    }
    bail!("Failed to get GPG public key ID");
}

impl TurbineRepo {
    pub fn new(remote: &str, branch: &str) -> Result<Self> {
        let tmp = tempfile::tempdir()?;

        debug!(remote = remote, dest = ?tmp.path(), "Cloning repository");
        let container = Repository::clone(&remote, tmp.path())?;
        let mut repo = Self {
            branch: branch.into(),
            tmp,
            container,
            last: None,
            contributors: vec![],
            last_refresh: None,
        };

        repo.refresh()?;
        Ok(repo)
    }

    /// Verify a commit's GPG signature and return its key ID.
    #[instrument(ret, level = "trace")]
    fn verify_signature(&self, commit: &Commit) -> Result<String> {
        // Receive the public key first
        Command::new("gpg")
            .arg("--keyserver")
            .arg("hkp://keys.gnupg.net")
            .arg("--recv-keys")
            .arg(get_public_key_id(&commit)?)
            .spawn()?
            .wait()?;

        // TODO verify the signature ourselves (gpgme?)
        if Command::new("git")
            .arg("verify-commit")
            .arg(commit.id().to_string())
            .current_dir(self.tmp.path())
            .spawn()?
            .wait()?
            .success()
        {
            Ok(get_public_key_id(&commit)?)
        } else {
            bail!("Failed to verify signature");
        }
    }

    pub fn refresh(&mut self) -> Result<()> {
        // Always fetch the repo first
        debug!("Fetching upstream repo");
        self.container
            .find_remote("origin")?
            .fetch(&[self.branch.clone()], None, None)?;

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

                // Check for GPG signature
                if let Some(header) = commit.raw_header() {
                    if !header.contains("gpgsig") {
                        continue;
                    }
                }

                if let Ok(key_id) = self.verify_signature(&commit) {
                    if let Some(message) = commit.message() {
                        #[cfg(feature = "monero")]
                        if let Some((_, address)) = message.split_once("XMR") {
                            let address = address.trim().to_string();
                            if let Some(contributor) = self
                                .contributors
                                .iter_mut()
                                .find(|contributor| contributor.key_id == key_id)
                            {
                                debug!(
                                    old = ?contributor.address,
                                    new = ?address,
                                    "Updating contributor address"
                                );
                                contributor.address = Address::XMR(address);
                            } else {
                                let contributor = Contributor {
                                    address: Address::XMR(address),
                                    last_payout: None,
                                    key_id,
                                    commits: Vec::new(),
                                };

                                info!(contributor = ?contributor, "Adding new contributor");
                                self.contributors.push(contributor);
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

                    // Check for GPG signature
                    if let Some(header) = commit.raw_header() {
                        if !header.contains("gpgsig") {
                            continue;
                        }
                    }

                    if let Ok(key_id) = self.verify_signature(&commit) {
                        if let Some(contributor) = self
                            .contributors
                            .iter_mut()
                            .find(|contributor| contributor.key_id == key_id)
                        {
                            info!(contributor = ?contributor, commit = ?commit, "Found new paid commit");
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

        self.last_refresh = Some(Utc::now());
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
