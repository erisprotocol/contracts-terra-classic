# Eris Amplified Staking Classic

Terra liquid staking derivative. Of the community, by the community, for the community.

The version ([v1.1.1](https://github.com/erisprotocol/contracts-terra-classic/releases/tag/v1.1.1)) of the Eris Amplifier Hub + Token is audited by [SCV Security](https://twitter.com/TerraSCV) ([link](https://github.com/SCV-Security/PublicReports/blob/main/CW/ErisProtocol/Eris%20Protocol%20-%20Amplified%20Staking%20-%20Audit%20Report%20v1.0.pdf)).

A previous version ([v1.0.0-rc0](https://github.com/st4k3h0us3/steak-contracts/releases/tag/v1.0.0-rc0)) of Steak was audited by [SCV Security](https://twitter.com/TerraSCV) ([link](https://github.com/SCV-Security/PublicReports/blob/main/CW/St4k3h0us3/St4k3h0us3%20-%20Steak%20Contracts%20Audit%20Review%20-%20%20v1.0.pdf)).

## Contracts

| Contract                                        | Description                                              |
| ----------------------------------------------- | -------------------------------------------------------- |
| [`eris-staking-hub-classic`](./contracts/hub)   | Manages minting/burning of ampLUNC token and bonded Luna |
| [`eris-stake-token-classic`](./contracts/token) | Modified CW20 token contract                             |

## Deployment

### Mainnet

For contract links see <https://app.erisprotocol.com>

## Building

For interacting with the smart contract clone <https://github.com/erisprotocol/liquid-staking-scripts> into the same parent folder.

## Fork

The initial version of the source code has been forked from the awesome steak liquid staking solution <https://github.com/st4k3h0us3/steak>

## Changes

- Renaming
- added a reward fee for running the protocol
- added a more detailed unbonding query
- Fixed an issue in reconciliation when the expected Luna was correct the unbinding queue items were not marked reconciled
- move scripts to another repository, so that the repo of the smart contracts will not be touched as much <https://github.com/erisprotocol/liquid-staking-scripts>

## License

Contents of this repository are open source under [GNU General Public License v3](./LICENSE) or later.
