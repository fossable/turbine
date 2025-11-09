#[cfg(feature = "monero")]
use crate::api::PaidCommit;
use crate::currency::Address;
use anyhow::{bail, Result};
use base64::prelude::*;
use cached::proc_macro::once;
use chrono::{DateTime, Utc};
use git2::{Commit, Oid, Repository, Sort};
use std::{path::PathBuf, process::Command, time::Duration};
use tempfile::TempDir;
use tracing::{debug, info, instrument};

#[derive(Debug)]
pub struct Contributor {
    pub address: Address,
    /// Paid commits
    pub commits: Vec<Oid>,
    /// The user's GPG public key ID which must remain constant
    pub key_id: String,

    pub last_payout: Option<DateTime<Utc>>,

    /// The contributor's public name from git history
    pub name: String,
}

impl Contributor {
    /// Compute payout for a specific commit using logarithmic growth formula:
    /// payout(n) = base × (1 + ln(n + 1))
    /// where n is the commit index (0-based)
    #[instrument(ret)]
    pub fn compute_payout(&self, commit_id: Oid, base_payout: u64, max_payout_cap: Option<u64>) -> u64 {
        // Find the index of this commit in the contributor's commit list
        let commit_index = self.commits.iter().position(|&id| id == commit_id)
            .unwrap_or(0);

        // Apply logarithmic growth formula: base × (1 + ln(n + 1))
        let multiplier = 1.0 + ((commit_index + 1) as f64).ln();
        let payout = (base_payout as f64 * multiplier) as u64;

        // Apply cap if configured
        match max_payout_cap {
            Some(cap) => payout.min(cap),
            None => payout,
        }
    }
}

///
pub struct TurbineRepo {
    /// Branch to track
    pub branch: String,

    /// Underlying git repository
    container: Repository,

    pub contributors: Vec<Contributor>,

    /// ID of the last commit we parsed
    pub last: Option<Oid>,

    /// Last time we refreshed
    pub last_refresh: Option<DateTime<Utc>>,

    /// Remote URI
    pub remote: String,

    /// Cloned repo directory
    tmp: TempDir,
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

/// Receive the public key for the given commit.
#[once(time = "36000", result = true)]
fn import_public_key(commit: &Commit) -> Result<()> {
    Command::new("gpg")
        .arg("--keyserver")
        .arg(std::env::var("TURBINE_GPG_KEYSERVER").unwrap_or("hkp://keyserver.ubuntu.com".into()))
        .arg("--recv-keys")
        .arg(get_public_key_id(&commit)?)
        .spawn()?
        .wait()?;

    Ok(())
}

/// Verify a commit's GPG signature and return its key ID.
#[once(result = true)]
fn verify_signature(repo: PathBuf, commit: &Commit) -> Result<String> {
    // Receive the public key first
    import_public_key(commit)?;

    // TODO verify the signature ourselves (gpgme?)
    if Command::new("git")
        .arg("verify-commit")
        .arg(commit.id().to_string())
        .current_dir(repo)
        .spawn()?
        .wait()?
        .success()
    {
        Ok(get_public_key_id(&commit)?)
    } else {
        bail!("Failed to verify signature");
    }
}

impl TurbineRepo {
    pub fn new(remote: &str, branch: &str) -> Result<Self> {
        let tmp = tempfile::tempdir()?;

        debug!(remote = remote, dest = ?tmp.path(), "Cloning repository");
        let container = Repository::clone(&remote, tmp.path())?;
        let mut repo = Self {
            branch: branch.into(),
            container,
            contributors: vec![],
            last: None,
            last_refresh: None,
            remote: remote.to_string(),
            tmp,
        };

        repo.refresh()?;
        Ok(repo)
    }

    #[cfg(feature = "monero")]
    #[instrument(skip(self), ret)]
    pub fn find_monero_transaction(
        &self,
        transfer: &monero_rpc::GotTransfer,
    ) -> Result<PaidCommit> {
        if let Some(contributor) = self
            .contributors
            .iter()
            .find(|contributor| contributor.address == Address::XMR(transfer.address.to_string()))
        {
            Ok(PaidCommit {
                amount: format!("{}", transfer.amount.as_xmr()),
                timestamp: transfer.timestamp.timestamp() as u64,
                contributor_name: contributor.name.clone(),
            })
        } else {
            bail!("Transaction not found");
        }
    }

    /// Get all signed commits (whether their signatures are valid or not).
    pub fn get_signed_commits(&self) -> Result<Vec<Oid>> {
        let mut revwalk = self.container.revwalk()?;
        let branch = self
            .container
            .find_branch(&self.branch, git2::BranchType::Local)?;
        let branch_ref = branch.into_reference();

        revwalk.push(branch_ref.target().unwrap())?;

        let mut commits = Vec::new();
        loop {
            if let Some(next) = revwalk.next() {
                let commit = self.container.find_commit(next?)?;

                // Check for GPG signature
                if let Some(header) = commit.raw_header() {
                    if header.contains("gpgsig") {
                        commits.push(commit.id());
                    }
                }
            } else {
                break;
            }
        }

        Ok(commits)
    }

    pub fn refresh(&mut self) -> Result<()> {
        // Always fetch the repo first
        debug!("Fetching upstream repo");
        self.container
            .find_remote("origin")?
            .fetch(&[self.branch.clone()], None, None)?;

        let mut revwalk = self.container.revwalk()?;
        revwalk.set_sorting(Sort::REVERSE)?;

        let start = if let Some(last) = self.last {
            last
        } else {
            let branch = self
                .container
                .find_branch(&self.branch, git2::BranchType::Local)?;
            let branch_ref = branch.into_reference();

            branch_ref.target().unwrap()
        };
        revwalk.push(start)?;

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

                if let Ok(key_id) = verify_signature(self.tmp.path().to_path_buf(), &commit) {
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
                                    name: commit.author().name().unwrap_or("<invalid>").to_string(),
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
        revwalk.push(start)?;

        // Find paid commits
        debug!("Searching for new paid commits");
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

                    if let Ok(key_id) = verify_signature(self.tmp.path().to_path_buf(), &commit) {
                        if let Some(contributor) = self
                            .contributors
                            .iter_mut()
                            .find(|contributor| contributor.key_id == key_id)
                        {
                            info!(contributor = ?contributor, commit = ?commit, "Found new paid commit");
                            contributor.commits.push(commit.id());
                        } else {
                            debug!(
                                key_id = key_id,
                                "Found signed commit, but no contributor is registered"
                            );
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
        debug!(repo = ?self, "Refreshed repository");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test contributor with N commits
    fn create_test_contributor(num_commits: usize) -> Contributor {
        let commits: Vec<Oid> = (0..num_commits)
            .map(|i| {
                // Create fake OIDs for testing
                Oid::from_bytes(&[i as u8; 20]).unwrap()
            })
            .collect();

        Contributor {
            address: Address::XMR("test_address".to_string()),
            commits,
            key_id: "test_key".to_string(),
            last_payout: None,
            name: "Test Contributor".to_string(),
        }
    }

    #[test]
    fn test_first_commit_gets_base_amount() {
        let contributor = create_test_contributor(1);
        let base_payout = 1_000_000_000; // 0.001 XMR
        let commit_id = contributor.commits[0];

        let payout = contributor.compute_payout(commit_id, base_payout, None);

        // First commit (index 0): base × (1 + ln(1)) = base × 1
        assert_eq!(payout, base_payout);
    }

    #[test]
    fn test_logarithmic_growth() {
        let contributor = create_test_contributor(10);
        let base_payout = 1_000_000_000; // 0.001 XMR

        // Test various commits to verify logarithmic growth
        let commit_1 = contributor.commits[0]; // index 0
        let commit_2 = contributor.commits[1]; // index 1
        let commit_10 = contributor.commits[9]; // index 9

        let payout_1 = contributor.compute_payout(commit_1, base_payout, None);
        let payout_2 = contributor.compute_payout(commit_2, base_payout, None);
        let payout_10 = contributor.compute_payout(commit_10, base_payout, None);

        // First commit: base × (1 + ln(1)) = base × 1
        assert_eq!(payout_1, base_payout);

        // Second commit: base × (1 + ln(2)) ≈ base × 1.693
        let expected_2 = (base_payout as f64 * (1.0 + 2.0_f64.ln())) as u64;
        assert_eq!(payout_2, expected_2);
        assert!(payout_2 > payout_1, "Second commit should pay more than first");

        // Tenth commit: base × (1 + ln(10)) ≈ base × 3.303
        let expected_10 = (base_payout as f64 * (1.0 + 10.0_f64.ln())) as u64;
        assert_eq!(payout_10, expected_10);
        assert!(payout_10 > payout_2, "Tenth commit should pay more than second");

        // Verify logarithmic property: growth slows down
        let growth_1_to_2 = payout_2 - payout_1;
        let growth_2_to_10 = payout_10 - payout_2;
        let rate_1_to_2 = growth_1_to_2 as f64 / 1.0; // per commit
        let rate_2_to_10 = growth_2_to_10 as f64 / 8.0; // per commit
        assert!(
            rate_2_to_10 < rate_1_to_2,
            "Growth rate should slow down (logarithmic property)"
        );
    }

    #[test]
    fn test_payout_cap_is_respected() {
        let contributor = create_test_contributor(100);
        let base_payout = 1_000_000_000; // 0.001 XMR
        let cap = 3_000_000_000; // 0.003 XMR

        // Test a later commit that would exceed the cap
        let commit_100 = contributor.commits[99]; // index 99

        let payout_uncapped = contributor.compute_payout(commit_100, base_payout, None);
        let payout_capped = contributor.compute_payout(commit_100, base_payout, Some(cap));

        // Verify uncapped payout would exceed cap
        assert!(payout_uncapped > cap, "Uncapped payout should exceed the cap");

        // Verify capped payout respects the cap
        assert_eq!(payout_capped, cap, "Capped payout should equal the cap");
    }

    #[test]
    fn test_payout_cap_not_applied_when_below() {
        let contributor = create_test_contributor(3);
        let base_payout = 1_000_000_000; // 0.001 XMR
        let cap = 10_000_000_000; // 0.01 XMR (much higher than early payouts)

        let commit_1 = contributor.commits[0];
        let payout = contributor.compute_payout(commit_1, base_payout, Some(cap));

        // Cap should not affect early commits
        assert_eq!(payout, base_payout);
    }

    #[test]
    fn test_different_base_amounts() {
        let contributor = create_test_contributor(5);
        let commit_1 = contributor.commits[0];

        // Test with different base amounts
        let small_base = 100_000_000; // 0.0001 XMR
        let large_base = 10_000_000_000; // 0.01 XMR

        let payout_small = contributor.compute_payout(commit_1, small_base, None);
        let payout_large = contributor.compute_payout(commit_1, large_base, None);

        assert_eq!(payout_small, small_base);
        assert_eq!(payout_large, large_base);
        assert_eq!(payout_large / payout_small, 100);
    }

    #[test]
    fn test_commit_not_in_list_defaults_to_index_zero() {
        let contributor = create_test_contributor(5);
        let base_payout = 1_000_000_000;

        // Create a commit ID that's not in the contributor's list
        let fake_commit = Oid::from_bytes(&[99; 20]).unwrap();

        let payout = contributor.compute_payout(fake_commit, base_payout, None);

        // Should default to index 0 (first commit amount)
        assert_eq!(payout, base_payout);
    }

    #[test]
    fn test_realistic_payout_progression() {
        let contributor = create_test_contributor(20);
        let base_payout = 1_000_000_000; // 0.001 XMR = 1e9 piconero

        // Calculate expected values for documentation
        let test_cases = vec![
            (0, 1_000_000_000),   // Commit 1: base × 1
            (1, 1_693_147_180),   // Commit 2: base × 1.693
            (4, 2_609_437_912),   // Commit 5: base × 2.609
            (9, 3_302_585_092),   // Commit 10: base × 3.303
            (19, 3_995_732_273),  // Commit 20: base × 3.996
        ];

        for (index, expected) in test_cases {
            let commit = contributor.commits[index];
            let payout = contributor.compute_payout(commit, base_payout, None);
            let diff = if payout > expected {
                payout - expected
            } else {
                expected - payout
            };
            assert!(
                diff < 100, // Allow small rounding differences
                "Commit {} payout {} should be close to expected {}",
                index + 1,
                payout,
                expected
            );
        }
    }
}
