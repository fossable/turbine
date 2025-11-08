#!/usr/bin/env bash
## Spawn end to end tests on example repos

set -e -x

# Generate new monero wallet for test contributor
{
	monero-wallet-cli --stagenet --create-address-file --password 5678 --mnemonic-language English --generate-new-wallet /wallets/contributor_wallet --command exit

	contributor_address=$(monero-wallet-cli --stagenet --wallet-file /wallets/contributor_wallet --password 5678 --command address | grep '^0.*Primary address' | awk '{print $2}')
}

# Create test repo
(
	mkdir /repo && cd /repo || return
	git init
	git commit --allow-empty -m "Initial commit"
	git commit -S --allow-empty -m "turbine: XMR ${contributor_address}"
	git commit -S --allow-empty -m "Another signed commit"
	git commit --allow-empty -m "Another unsigned commit"
)

# Spawn turbine
RUST_LOG=debug turbine serve \
	--stagenet \
	--repo file:///repo \
	--branch master \
	--monero-block-height "$(cat /wallets/pool_height)" \
	--monero-wallet-path /wallets/pool_wallet \
	--monero-wallet-password 1234 &

# Wait for API server to be ready
for i in {1..30}; do
	if curl -s http://localhost:80/ >/dev/null 2>&1; then
		break
	fi
	sleep "$i"
done

# Refresh immediately
curl -X POST http://localhost:80/refresh

# Check the UI to verify payouts appear
# curl -s http://localhost:80/ | grep -A 5 "commit" || echo "No payouts found in UI"

# The 'docker run' invocation just shows logs forever. Use 'docker exec' to get
# a debugging session within the container.
wait
