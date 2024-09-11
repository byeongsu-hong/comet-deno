//! In-memory key/value store ABCI application.

use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender},
};

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tendermint_proto::{
    abci::ExecTxResult,
    v0_38::abci::{
        RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestQuery, ResponseCheckTx,
        ResponseCommit, ResponseFinalizeBlock, ResponseInfo, ResponseQuery,
    },
};
use tracing::{debug, info};

use tendermint_abci::{Application, Error};

use crate::{
    runner::{Runner, RunnerCommand},
    script::load_scripts,
};

pub const MAX_VARINT_LENGTH: usize = 16;

#[derive(Serialize, Deserialize)]
struct Tx {
    pub path: String,
    pub sender: String,
    pub request: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct DenoKVService {
    cmd_tx: Sender<RunnerCommand>,
    scripts_dir: String,
    scripts: HashMap<String, Vec<String>>,
}

impl DenoKVService {
    /// Constructor.
    pub fn new(scripts_dir: &str) -> (Self, Runner) {
        let (cmd_tx, cmd_rx) = channel();
        let scripts = load_scripts(scripts_dir).unwrap();
        (
            Self {
                cmd_tx,
                scripts_dir: scripts_dir.to_string(),
                scripts,
            },
            Runner::new(cmd_rx),
        )
    }

    fn query(&self, req: RequestQuery) -> ResponseQuery {
        if !self
            .scripts
            .get("query")
            .unwrap_or(&vec![])
            .contains(&req.path)
        {
            return ResponseQuery {
                code: 1,
                log: format!("query path {} not supported", req.path),
                ..Default::default()
            };
        }

        let (result_tx, result_rx) = channel();
        channel_send(
            &self.cmd_tx,
            RunnerCommand::Query {
                path: format!("{}/{}.query.ts", self.scripts_dir, req.path),
                request: req.data,
                result_tx,
            },
        )
        .unwrap();

        let resp = channel_recv(&result_rx).unwrap();

        match resp {
            Ok(v) => v,
            Err(err) => ResponseQuery {
                code: 1,
                log: err.to_string(),
                ..Default::default()
            },
        }
    }

    fn execute(&self, tx: Bytes) -> ExecTxResult {
        let tx: Tx = serde_json::from_slice(&tx).unwrap();

        if !self
            .scripts
            .get("execute")
            .unwrap_or(&vec![])
            .contains(&tx.path)
        {
            return ExecTxResult {
                code: 1,
                log: format!("execute path {} not supported", tx.path),
                ..Default::default()
            };
        }

        let (result_tx, result_rx) = channel();
        channel_send(
            &self.cmd_tx,
            RunnerCommand::Execute {
                path: format!("{}/{}.execute.ts", self.scripts_dir, tx.path),
                sender: tx.sender,
                request: tx.request,
                result_tx,
            },
        )
        .unwrap();

        let resp = channel_recv(&result_rx).unwrap();

        match resp {
            Ok(v) => v,
            Err(err) => ExecTxResult {
                code: 1,
                log: err.to_string(),
                ..Default::default()
            },
        }
    }
}

impl Application for DenoKVService {
    fn info(&self, request: RequestInfo) -> ResponseInfo {
        debug!(
            "Got info request. Tendermint version: {}; Block version: {}; P2P version: {}",
            request.version, request.block_version, request.p2p_version
        );

        let (result_tx, result_rx) = channel();
        channel_send(&self.cmd_tx, RunnerCommand::GetInfo { result_tx }).unwrap();
        let (last_block_height, last_block_app_hash) = channel_recv(&result_rx).unwrap();

        ResponseInfo {
            data: "deno-kv-rs".to_string(),
            version: "0.1.0".to_string(),
            app_version: 1,
            last_block_height,
            last_block_app_hash: last_block_app_hash.into(),
        }
    }

    fn query(&self, request: RequestQuery) -> ResponseQuery {
        self.query(request)
    }

    fn check_tx(&self, _request: RequestCheckTx) -> ResponseCheckTx {
        ResponseCheckTx {
            gas_wanted: 1,
            gas_used: 0,
            ..Default::default()
        }
    }

    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let mut tx_results = vec![];
        for tx in request.txs {
            tx_results.push(self.execute(tx));
        }

        ResponseFinalizeBlock {
            tx_results,
            ..Default::default()
        }
    }

    fn commit(&self) -> ResponseCommit {
        let (result_tx, result_rx) = channel();
        channel_send(&self.cmd_tx, RunnerCommand::Commit { result_tx }).unwrap();
        let (height, _) = channel_recv(&result_rx).unwrap();
        info!("Committed height {}", height);
        ResponseCommit {
            retain_height: height - 1,
        }
    }
}

fn channel_send<T>(tx: &Sender<T>, value: T) -> Result<(), Error> {
    tx.send(value).map_err(Error::send)
}

fn channel_recv<T>(rx: &Receiver<T>) -> Result<T, Error> {
    rx.recv().map_err(Error::channel_recv)
}
