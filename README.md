# WAVS Rust Gui Example

This builds off the [WAVS Foundry Template](https://github.com/Lay3rLabs/wavs-foundry-template) and adds a GUI to the example.

Some [code is shared](./shared/) between the [Component](./components/eth-price-oracle/) and the [Frontend](./frontend/)

You have a choice of using Metamask, Anvil, or a Mnemonic for your wallet (via dropdown in the GUI)

# Getting started

First follow the [WAVS Foundry Template README](https://github.com/Lay3rLabs/wavs-foundry-template/blob/main/README.md)

In short, assuming you have all the tools installed, you can run:

```bash
1. make build
2. (in another terminal) make start-all
3. make deploy-contracts
4. TRIGGER_EVENT="NewTrigger(bytes)" make deploy-service
```

After you have the server running, contracts deployed, and service created (this must all be done first!), start the gui with:

```bash
make frontend-dev
```

You can then open http://127.0.0.1:8080/ in your browser.

# Setting up Metamask with Anvil

You'll need to add Anvil to Metamask manually. Settings will look something like this:

- Network Name: `Anvil`
- Default RPC URL: `localhost:8545`
- ChainID: `31337`
- Currency Symbol: `ETH`
- Block Explorer URL: (leave empty)

And don't forget to add a Metamask account with the Anvil private key:

```
mnemonic: test test test test test test test test test test test junk
private key: ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```
