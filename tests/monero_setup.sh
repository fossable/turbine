#!/usr/bin/env bash
## Setup Monero

set -e -x

mkdir /wallets

# Generate new monero wallet for turbine
{
	monero-wallet-cli --stagenet --create-address-file --password 1234 --mnemonic-language English --generate-new-wallet /wallets/pool_wallet --command exit

	# Save address and block height
	monero-wallet-cli --stagenet --wallet-file /wallets/pool_wallet --password 1234 --command address | grep '^0.*Primary address' | awk '{print $2}' >/wallets/pool_address
	monero-wallet-cli --stagenet --wallet-file /wallets/pool_wallet --password 1234 --daemon-address stagenet.xmr-tw.org:38081 --command bc_height | tail -1 >/wallets/pool_height
}

echo "Fund this wallet from a stagenet faucet: $(cat /wallets/pool_address)"
