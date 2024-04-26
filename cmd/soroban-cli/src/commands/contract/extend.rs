use std::{fmt::Debug, path::Path, str::FromStr};

use clap::{command, Parser};
use soroban_env_host::xdr::{
    Error as XdrError, ExtendFootprintTtlOp, ExtensionPoint, LedgerEntry, LedgerEntryChange,
    LedgerEntryData, LedgerFootprint, Memo, MuxedAccount, Operation, OperationBody, Preconditions,
    SequenceNumber, SorobanResources, SorobanTransactionData, Transaction, TransactionExt,
    TransactionMeta, TransactionMetaV3, TtlEntry, Uint256,
};

use crate::{
    commands::config,
    key,
    rpc::{self, Client},
    wasm, Pwd,
};

const MAX_LEDGERS_TO_EXTEND: u32 = 535_679;

#[derive(Parser, Debug, Clone)]
#[group(skip)]
pub struct Cmd {
    /// Number of ledgers to extend the entries
    #[arg(long, required = true)]
    pub ledgers_to_extend: u32,
    /// Only print the new Time To Live ledger
    #[arg(long)]
    pub ttl_ledger_only: bool,
    #[command(flatten)]
    pub key: key::Args,
    #[command(flatten)]
    pub config: config::Args,
    #[command(flatten)]
    pub fee: crate::fee::Args,
}

impl FromStr for Cmd {
    type Err = clap::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use clap::{CommandFactory, FromArgMatches};
        Self::from_arg_matches_mut(&mut Self::command().get_matches_from(s.split_whitespace()))
    }
}

impl Pwd for Cmd {
    fn set_pwd(&mut self, pwd: &Path) {
        self.config.set_pwd(pwd);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("parsing key {key}: {error}")]
    CannotParseKey {
        key: String,
        error: soroban_spec_tools::Error,
    },
    #[error("parsing XDR key {key}: {error}")]
    CannotParseXdrKey { key: String, error: XdrError },

    #[error(transparent)]
    Config(#[from] config::Error),
    #[error("either `--key` or `--key-xdr` are required")]
    KeyIsRequired,
    #[error("xdr processing error: {0}")]
    Xdr(#[from] XdrError),
    #[error("Ledger entry not found")]
    LedgerEntryNotFound,
    #[error("missing operation result")]
    MissingOperationResult,
    #[error(transparent)]
    Rpc(#[from] rpc::Error),
    #[error(transparent)]
    Wasm(#[from] wasm::Error),
    #[error(transparent)]
    Key(#[from] key::Error),
}

impl Cmd {
    #[allow(clippy::too_many_lines)]
    pub async fn run(&self) -> Result<(), Error> {
        let ttl_ledger = self.run_against_rpc_server().await?;
        if self.ttl_ledger_only {
            println!("{ttl_ledger}");
        } else {
            println!("New ttl ledger: {ttl_ledger}");
        }

        Ok(())
    }

    fn ledgers_to_extend(&self) -> u32 {
        let res = u32::min(self.ledgers_to_extend, MAX_LEDGERS_TO_EXTEND);
        if res < self.ledgers_to_extend {
            tracing::warn!(
                "Ledgers to extend is too large, using max value of {MAX_LEDGERS_TO_EXTEND}"
            );
        }
        res
    }

    async fn run_against_rpc_server(&self) -> Result<u32, Error> {
        let network = self.config.get_network()?;
        tracing::trace!(?network);
        let keys = self.key.parse_keys()?;
        let network = &self.config.get_network()?;
        let client = Client::new(&network.rpc_url)?;
        let key = self.config.key_pair()?;
        let extend_to = self.ledgers_to_extend();

        // Get the account sequence number
        let public_strkey =
            hcnet_strkey::ed25519::PublicKey(key.verifying_key().to_bytes()).to_string();
        let account_details = client.get_account(&public_strkey).await?;
        let sequence: i64 = account_details.seq_num.into();

        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(key.verifying_key().to_bytes())),
            fee: self.fee.fee,
            seq_num: SequenceNumber(sequence + 1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![Operation {
                source_account: None,
                body: OperationBody::ExtendFootprintTtl(ExtendFootprintTtlOp {
                    ext: ExtensionPoint::V0,
                    extend_to,
                }),
            }]
            .try_into()?,
            ext: TransactionExt::V1(SorobanTransactionData {
                ext: ExtensionPoint::V0,
                resources: SorobanResources {
                    footprint: LedgerFootprint {
                        read_only: keys.clone().try_into()?,
                        read_write: vec![].try_into()?,
                    },
                    instructions: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                },
                resource_fee: 0,
            }),
        };

        let (result, meta, events) = client
            .prepare_and_send_transaction(&tx, &key, &[], &network.network_passphrase, None, None)
            .await?;

        tracing::trace!(?result);
        tracing::trace!(?meta);
        if !events.is_empty() {
            tracing::info!("Events:\n {events:#?}");
        }

        // The transaction from core will succeed regardless of whether it actually found & extended
        // the entry, so we have to inspect the result meta to tell if it worked or not.
        let TransactionMeta::V3(TransactionMetaV3 { operations, .. }) = meta else {
            return Err(Error::LedgerEntryNotFound);
        };

        // Simply check if there is exactly one entry here. We only support extending a single
        // entry via this command (which we should fix separately, but).
        if operations.len() == 0 {
            return Err(Error::LedgerEntryNotFound);
        }

        if operations[0].changes.is_empty() {
            let entry = client.get_full_ledger_entries(&keys).await?;
            let extension = entry.entries[0].live_until_ledger_seq;
            if entry.latest_ledger + i64::from(extend_to) < i64::from(extension) {
                return Ok(extension);
            }
        }

        match (&operations[0].changes[0], &operations[0].changes[1]) {
            (
                LedgerEntryChange::State(_),
                LedgerEntryChange::Updated(LedgerEntry {
                    data:
                        LedgerEntryData::Ttl(TtlEntry {
                            live_until_ledger_seq,
                            ..
                        }),
                    ..
                }),
            ) => Ok(*live_until_ledger_seq),
            _ => Err(Error::LedgerEntryNotFound),
        }
    }
}
