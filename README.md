<p align="center">
	<img src="https://raw.githubusercontent.com/fossable/turbine/master/.github/images/turbine-256.png" />
</p>

![License](https://img.shields.io/github/license/fossable/turbine)
![Build](https://github.com/fossable/turbine/actions/workflows/test.yml/badge.svg)
![GitHub repo size](https://img.shields.io/github/repo-size/fossable/turbine)
![Stars](https://img.shields.io/github/stars/fossable/turbine?style=social)

<hr>

**turbine** distributes cryptocurrency to contributors of git repos.

### Beyond bug bounties

Issue bounties can be counterproductive as they discourage collaboration and
often attract minimum-quality solutions.

Instead of rewarding specific contributions like bug bounties, **turbine**
reduces unfriendly competition by rewarding _all_ contributions. Multiple
authors can even work on the same issue and all get paid independently.

### Award amount scales over time rather than with change complexity

Although a complex change might deserve a larger award than a minor typo fix,
**turbine** doesn't factor the magnitude of changes. Instead, a developer's
award amount starts out small and increases over time as they make more
contributions.

This incentivizes contributors to stick around and break their changes up into
smaller chunks (which is often good for other reasons too).

It's up to the discretion of the person that merges PRs to make sure
contributors aren't unfairly boosting their rewards. _In the unbelievable event
that such an injustice occurs_, maintainers can cancel payouts or ban
contributors.

### What if the `turbine` owner steals the project's funds?

Since `turbine` is self-hosted, the crypto wallet is fully under control of the
project owner. We have to trust them not to misuse funds deposited in `turbine`.
Just like we also have to trust them not to include a backdoor in the software
(for example).

### Funding directly impacts development

When you fund a project's `turbine`, those funds directly support further
development of that project. It's entirely up to the maintainer what issues get
worked on, unlike the bug bounty model.

## Using `turbine` as a funder

First, you need to find a repository that's hosting a `turbine`. It's important
to use a legitimate instance (see two paragraphs above), so make sure a
reference to the `turbine` URL is found somewhere in the official project.

The `turbine` homepage has the crypto wallet address that allows you to add
funds. When you add funds to a `turbine`, it goes into the project's pool. As
contributions are made to the project's git repo, `turbine` will automatically
disperse funds to committers.

## Using `turbine` as a contributor

All contributor commits must be GPG signed (because otherwise someone could
impersonate your name/email in git history).

### Generate a GPG keypair

If you don't already have a GPG keypair, generate a new one:

```sh
gpg --full-generate-key
```

Make sure to use the same email address as your git config:
`git config user.email`.

### Setup commit signing

Turn on commit signing globally (or on a per-repo basis):

```sh
git config --global commit.gpgsign true
git config --global user.signingkey <public key ID>
```

### Send your public key to a keyserver

To allow `turbine` to find your public key and verify commits, upload it to this
keyserver:

```sh
gpg --keyserver hkp://keyserver.ubuntu.com --send-keys <public key ID>
```

### Commit your payment address

Register your payment address with a signed commit message so `turbine` knows
who to pay. If you ever update your GPG key or wallet address, this commit can
be made multiple times and the last one takes effect.

```sh
git commit --allow-empty -m "turbine: XMR <wallet address>"
```

### Start contributing!

Contribute as normal and `turbine` will pay you automatically.

## Running your own `turbine`

`turbine` is fully dockerized and requires no persistent state. It reads the
entire git history on startup and checks the crypto ledger to determine what
commits have already been paid out.

### Monero

So far `turbine` only supports Monero, but other currencies can be supported
later.

```sh
docker run \
    -p 80:80 \
    -e MONERO_WALLET_ADDRESS=478zp7VkvbFXdFJ7areyaxj5b2AbBrRmGezoJAiQJtT4f5nD1DEYtg7EGVcrnXzTdgci6Q5WdTKRo3veEY3itRnnDDUxmFh \
    -e MONERO_WALLET_SPENDKEY=81cb36d45e311ad4102f77753048e0236f5df7a0ec83e3607a0b534caddc2f0a \
    -e MONERO_WALLET_VIEWKEY=96f6b0d2417000e63ad36987407b3be31434172e8d5fbd0e79aaee3cc0065609 \
    fossable/turbine \
        --stagenet \
        --repo <repo clone URL> \
        --branch master \
        --monero-block-height 565903 \
        --monero-wallet-password 1234
```
