# Soroban-RPC

Soroban-RPC allows you to communicate directly with Soroban via a JSON RPC interface.

For example, you can build an application and have it send a transaction, get ledger and event data or simulate transactions.

## Dependencies
  - [Git](https://git-scm.com/downloads)
  - [Go](https://golang.org/doc/install)
  - [Rust](https://www.rust-lang.org/tools/install)
  - [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## Building Hcnet-Core
Soroban-RPC requires an instance of hcnet-core binary on the same host. This is referred to as the `Captive Core`. 
Since, we are building RPC from source, we recommend considering two approaches to get the hcnet-core binary:
- If saving time is top priority and your development machine is on a linux debian OS, then consider installing the 
testnet release candidates from the [testing repository.](https://apt.hcnet.org/pool/unstable/s/hcnet-core/)
- The recommended option is to compile the core source directly on your machine:
    - Clone the hcnet-core repo:
        ```bash
        git clone https://github.com/hcnet/hcnet-core.git
        cd hcnet-core
        ```
    - Fetch the tags and checkout the testnet release tag:
        ```bash
        git fetch --tags
        git checkout tags/v20.0.0-rc.2.1 -b soroban-testnet-release
        ```
    - Follow the build steps listed in [INSTALL.md](https://github.com/hcnet/hcnet-core/blob/master/INSTALL.md) file for the instructions on building the local binary

## Building Soroban-RPC
- Similar to hcnet-core, we will clone the soroban-tools repo and checkout the testnet release tag:
```bash
git clone https://github.com/hcnet/soroban-tools.git
cd soroban-tools
git fetch --tags
git checkout tags/v20.0.0-rc4 -b soroban-testnet-release
```
- Build soroban-rpc target:
```bash
make build-soroban-rpc
```
This will install and build the required dependencies and generate a `soroban-rpc` binary in the working directory.

## Configuring and Running RPC Server
- Both hcnet-core and soroban-rpc require configuration files to run. 
  - For production, we specifically recommend running Soroban RPC with a TOML configuration file rather than CLI flags. 
  - There is a new subcommand `gen-config-file` which takes all the same arguments as the root command (or no arguments at all), 
  and outputs the resulting config toml file to stdout.
      ```bash
      ./soroban-rpc gen-config-file
      ```
  - Paste the output to a file and save it as `.toml` file in any directory. 
  - Make sure to update the config values to testnet specific ones. You can refer to [Configuring](https://docs.google.com/document/d/1SIbrFWFgju5RAsi6stDyEtgTa78VEt8f3HhqCLoySx4/edit#heading=h.80d1jdtd7ktj) section in the Runbook for specific config settings.
- If everything is set up correctly, then you can run the RPC server with the following command:
```bash
./soroban-rpc --config-path <PATH_TO_THE_RPC_CONFIG_FILE>
```