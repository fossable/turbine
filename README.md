<p align="center">
	<img src="https://raw.githubusercontent.com/fossable/turbine/master/.github/images/turbine-256.png" />
</p>

![License](https://img.shields.io/github/license/fossable/turbine)
![Build](https://github.com/fossable/turbine/actions/workflows/test.yml/badge.svg)
![GitHub repo size](https://img.shields.io/github/repo-size/fossable/turbine)
![Stars](https://img.shields.io/github/stars/fossable/turbine?style=social)
<hr>

**turbine** distributes cryptocurrency to contributors of git repos.

### Beyond bug bounties!

Issue bounties can be counterproductive as they discourage collaboration and
often attract minimum-quality solutions.

Instead of rewarding specific contributions like bug bounties, **turbine**
reduces unfriendly competition by rewarding _all_ contributions. Multiple
authors can even work on the same issue and all get paid independently.

### Award amount scales over time rather than with change complexity

Although a complex change might deserve a larger award than a minor typo fix,
**turbine** doesn't factor the magnitude of changes. Instead, a developer's award
amount starts out small and increases over time as they make more contributions.

This incentivizes contributors to stick around and break their changes up into
smaller chunks (which is a good thing for other reasons too).

It's up to the discretion of the person that merges PRs to make sure contributors
aren't unfairly boosting their rewards. In the event that such an injustice
occurs, maintainers can cancel payouts or ban contributors.

### What if the `turbine` owner steals the project's funds?

Since `turbine` is self-hosted, the crypto wallet is fully under control of the
project owner. We have to trust them not to misuse funds deposited in `turbine`,
just like we have to trust them not to include a backdoor in the software (for example).

## Using `turbine` as a contributor

First, you need to find a repository that's hosting a `turbine`. Here are some examples:

- `https://github.com/fossable/goldboot`

### Generate a GPG keypair

If you don't already have a GPG keypair, generate a new one:

```
gpg --full-generate-key
```

Make sure to use the same email address as your git config: `git config user.email`.

### Setup commit signing

Turn on commit signing globally (or on a per-repo basis):

```
git config --global commit.gpgsign true
git config --global user.signingkey <public key ID>
```

### Send your public key to a keyserver

To allow `turbine` to find your public key and verify commits, upload it to this
keyserver:

```
gpg --keyserver hkp://keys.gnupg.net --send-keys <public key ID>
```

### Commit your payment address

Add your payment address to a signed commit message so `turbine` knows who to pay.
If you ever update your GPG key or wallet address, this commit can be made multiple
times and the last one takes effect.

```
git commit --allow-empty -m "turbine: XMR <wallet address>"
```

### Start contributing!

Contribute as normal and `turbine` will pay you automatically.

## Running your own `turbine`

`turbine` is fully dockerized and requires no persistent state.

### Monero

```
docker run \
	-e MONERO_WALLET_ADDRESS=<address> \
	-e MONERO_WALLET_SPENDKEY=<private key> \
	-e MONERO_WALLET_VIEWKEY=<private key> \
	fossable/turbine \
		--stagenet \
    --repo <repo clone URL> \
    --branch master \
    --monero-block-height <wallet initial block height> \
    --monero-wallet-password 1234
```
