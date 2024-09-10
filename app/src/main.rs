mod loader;
mod runner;
mod runtime;
mod runtime_ops;
mod script;
mod service;
mod store;

use bytes::Bytes;
use runner::Runner;
use serde_json::json;
use service::DenoKVService;
use tendermint::abci::Event;
use tendermint_abci::{ClientBuilder, Server, ServerBuilder};
use tendermint_proto::abci::{RequestEcho, RequestFinalizeBlock, RequestQuery};

#[tokio::main]
async fn start_server(server: Server<DenoKVService>) -> anyhow::Result<()> {
    let server_addr = server.local_addr();

    tracing::info!("server listening on {:?}", server_addr);

    Ok(server.listen()?)
}

#[tokio::main]
async fn start_runner(mut runner: Runner) -> anyhow::Result<()> {
    let res = runner.run().await;
    match res {
        Ok(_) => tracing::info!("runner finished successfully"),
        Err(e) => tracing::error!("runner error: {:?}", e),
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let (app, runner) = DenoKVService::new("../scripts");

    let server = ServerBuilder::default().bind("127.0.0.1:26658", app)?;
    let server_url = server.local_addr();

    let server_task = std::thread::Builder::new()
        .name("server".to_string())
        .spawn(|| start_server(server))?;

    let runner_task = std::thread::Builder::new()
        .name("runner".to_string())
        .spawn(|| start_runner(runner))?;

    // ======================================== TESTING ======================================== //

    let mut client = ClientBuilder::default().connect(server_url)?;

    // ECHO
    let res = client.echo(RequestEcho {
        message: "hello".to_string(),
    })?;
    assert_eq!(res.message, "hello");

    // SET
    let res = client.finalize_block(RequestFinalizeBlock {
        txs: vec![json!({
            "path": "kv-set",
            "sender": "eddy",
            "request": json!({
                "key": "name",
                "value": "eddy"
            })
        })
        .to_string()
        .into_bytes()
        .into()],
        ..Default::default()
    })?;
    assert_eq!(res.tx_results.len(), 1);
    assert_eq!(res.tx_results[0].events.len(), 1);
    assert_eq!(
        res.tx_results[0].events[0],
        Event::new("kv-set", [("name", "eddy")]).into()
    );

    // GET
    let res = client.query(RequestQuery {
        path: "kv-get".to_string(),
        data: json!({
            "key": "name"
        })
        .to_string()
        .into_bytes()
        .into(),
        ..Default::default()
    })?;
    assert_eq!(
        res.value,
        Bytes::from(json!({"value": "eddy"}).to_string().into_bytes())
    );

    // ======================================== TESTING ======================================== //

    server_task.join().unwrap()?;
    runner_task.join().unwrap()?;

    Ok(())
}
