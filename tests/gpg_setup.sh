#!/usr/bin/env bash
## Setup GPG for commit signing.

set -e -x

# Generate a new keypair for commit signing
{
	cat <<-EOF >/tmp/keygen-parameters
		Key-Type: RSA
		Key-Length: 2048
		Subkey-Type: RSA
		Subkey-Length: 2048
		Name-Real: Test Contributor
		Name-Email: test@example.com
		Expire-Date: 1d
		%no-protection
		%commit
	EOF

	gpg --batch --generate-key /tmp/keygen-parameters

	key_id=$(gpg --list-keys --with-colons test@example.com | grep fpr | head -1 | cut -d: -f10)
}

# Send it to the keyserver
gpg --keyserver hkp://keyserver.ubuntu.com --send-keys "${key_id}"

# Configure commit signing
git config --system user.name "Test Contributor"
git config --system user.email "test@example.com"
git config --system user.signingkey "${key_id}"
git config --system --add safe.directory '*'
