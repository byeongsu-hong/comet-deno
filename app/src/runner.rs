use core::str;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use bytes::{Bytes, BytesMut};

use tendermint_abci::Error;
use tendermint_proto::abci::{ExecTxResult, ResponseQuery};
use tokio::sync::Mutex;

use crate::{
    runtime,
    service::MAX_VARINT_LENGTH,
    store::{MemoryStore, Store},
};

#[derive(Debug)]
pub enum RunnerCommand {
    #[allow(dead_code)]
    GetInfo { result_tx: Sender<(i64, Vec<u8>)> },
    Query {
        path: String,
        request: Bytes,
        result_tx: Sender<anyhow::Result<ResponseQuery>>,
    },
    Execute {
        path: String,
        sender: String,
        request: serde_json::Value,
        result_tx: Sender<anyhow::Result<ExecTxResult>>,
    },
    #[allow(dead_code)]
    Commit { result_tx: Sender<(i64, Vec<u8>)> },
}

pub struct Runner {
    rx: Receiver<RunnerCommand>,
    store: Arc<Mutex<dyn Store>>,
    height: i64,
    app_hash: Vec<u8>,
}

impl Runner {
    pub fn new(rx: Receiver<RunnerCommand>) -> Self {
        Self {
            rx,
            height: 0,
            app_hash: vec![0_u8; MAX_VARINT_LENGTH],
            store: Arc::new(Mutex::new(MemoryStore::new())),
        }
    }

    async fn handle_info(&self) -> anyhow::Result<(i64, Vec<u8>)> {
        Ok((self.height, self.app_hash.clone()))
    }

    async fn handle_query(&self, path: String, request: Bytes) -> anyhow::Result<ResponseQuery> {
        tracing::info!(
            "handle_query: path={}, request={}",
            path,
            str::from_utf8(&request)?
        );

        let runtime_res = runtime::run(
            Arc::clone(&self.store),
            runtime::RuntimeMode::Query,
            "<querier>",
            serde_json::from_slice(&request)?,
            &path,
        )
        .await?;

        let runner_res = match runtime_res {
            runtime::RuntimeRunResult::Query(res) => {
                let v = serde_json::to_vec(&res)?;

                ResponseQuery {
                    value: v.into(),
                    height: self.height,
                    ..Default::default()
                }
            }
            _ => panic!("unexpected runtime result"),
        };

        Ok(runner_res)
    }

    async fn handle_execute(
        &mut self,
        path: String,
        sender: String,
        request: serde_json::Value,
    ) -> anyhow::Result<ExecTxResult> {
        tracing::info!(
            "handle_execute: path={}, sender={}, request={}",
            path,
            sender,
            request
        );

        let runtime_res = runtime::run(
            Arc::clone(&self.store),
            runtime::RuntimeMode::Execute,
            &sender,
            request,
            &path,
        )
        .await?;

        let runner_res = match runtime_res {
            runtime::RuntimeRunResult::Execute(events) => ExecTxResult {
                events,
                ..Default::default()
            },
            _ => panic!("unexpected runtime result"),
        };

        Ok(runner_res)
    }

    async fn handle_commit(&mut self) -> anyhow::Result<(i64, Vec<u8>)> {
        // As in the Go-based key/value store, simply encode the number of
        // items as the "app hash"
        let mut app_hash = BytesMut::with_capacity(MAX_VARINT_LENGTH);
        prost::encoding::encode_varint(self.store.lock().await.len().await? as u64, &mut app_hash);
        self.app_hash = app_hash.to_vec();
        self.height += 1;
        Ok((self.height, self.app_hash.clone()))
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let cmd = self.rx.recv().map_err(Error::channel_recv)?;

            match cmd {
                RunnerCommand::GetInfo { result_tx } => {
                    let res = self.handle_info().await?;
                    result_tx.send(res)?
                }
                RunnerCommand::Query {
                    path,
                    request,
                    result_tx,
                } => result_tx.send(self.handle_query(path, request).await)?,
                RunnerCommand::Execute {
                    path,
                    sender,
                    request,
                    result_tx,
                } => result_tx.send(self.handle_execute(path, sender, request).await)?,
                RunnerCommand::Commit { result_tx } => {
                    result_tx.send(self.handle_commit().await?)?
                }
            }
        }
    }
}
