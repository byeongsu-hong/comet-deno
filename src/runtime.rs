use std::{borrow::Cow, env, fmt::Display, rc::Rc, sync::Arc};

use deno_core::{error::AnyError, resolve_path, Extension, JsRuntime, OpDecl, RuntimeOptions};
use tendermint_proto::abci::Event;
use tokio::sync::Mutex;

use crate::{
    loader,
    runtime_ops::{
        op_ctx_emit, op_ctx_get_request, op_ctx_get_sender, op_ctx_respond, op_kv_get, op_kv_set,
    },
    store::Store,
};

pub enum RuntimeMode {
    Query,
    Execute,
}

impl RuntimeMode {
    pub fn assert_query(&self) -> Result<(), AnyError> {
        if !matches!(self, RuntimeMode::Query) {
            return Err(AnyError::msg("expected query mode"));
        }
        Ok(())
    }

    pub fn assert_execute(&self) -> Result<(), AnyError> {
        if !matches!(self, RuntimeMode::Execute) {
            return Err(AnyError::msg("expected execute mode"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum RuntimeRunResult {
    Query(serde_json::Value),
    Execute(Vec<Event>),
}

impl Display for RuntimeRunResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeRunResult::Query(res) => write!(f, "query: {}", res),
            RuntimeRunResult::Execute(events) => write!(f, "execute: {:?}", events),
        }
    }
}

pub struct OpStateContext {
    pub(crate) mode: RuntimeMode,
    pub(crate) store: Arc<Mutex<dyn Store>>,
    pub(crate) sender: String,
    pub(crate) events: Vec<Event>,
    pub(crate) request: serde_json::Value,
    pub(crate) response: Option<serde_json::Value>,
}

pub const OP_DECL: &[OpDecl] = &[
    op_kv_set(),
    op_kv_get(),
    op_ctx_emit(),
    op_ctx_respond(),
    op_ctx_get_sender(),
    op_ctx_get_request(),
];

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

pub fn init_runtime() -> JsRuntime {
    JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(loader::TsModuleLoader)),
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        extensions: vec![Extension {
            name: "deno-kv-ops-ext",
            ops: Cow::Borrowed(OP_DECL),
            ..Default::default()
        }],
        ..Default::default()
    })
}

pub async fn run(
    store: Arc<Mutex<dyn Store>>,
    mode: RuntimeMode,
    sender: &str,
    request: serde_json::Value,
    file_path: &str,
) -> Result<RuntimeRunResult, AnyError> {
    let mut runtime = init_runtime();

    runtime.op_state().borrow_mut().put(OpStateContext {
        mode,
        store,
        sender: sender.to_string(),
        events: vec![],
        request,
        response: None,
    });

    let main_module = resolve_path(file_path, env::current_dir()?.as_path())?;
    let module_id = runtime.load_main_es_module(&main_module).await?;
    let result = runtime.mod_evaluate(module_id);
    runtime.run_event_loop(Default::default()).await?;
    result.await?;

    let ctx = runtime.op_state().borrow_mut().take::<OpStateContext>();
    if ctx.response.is_none() && matches!(ctx.mode, RuntimeMode::Query) {
        return Err(AnyError::msg("respond not called"));
    }

    match ctx.mode {
        RuntimeMode::Query => Ok(RuntimeRunResult::Query(ctx.response.expect("no response"))),
        RuntimeMode::Execute => Ok(RuntimeRunResult::Execute(ctx.events)),
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use runtime::{run, RuntimeMode};
    use serde_json::json;
    use store::{MemoryStore, Store};
    use tokio::sync::Mutex;

    use crate::*;

    #[tokio::test]
    async fn test_runtime() {
        let store: Arc<Mutex<dyn Store>> = Arc::new(Mutex::new(MemoryStore::new()));

        let res = run(
            Arc::clone(&store),
            RuntimeMode::Execute,
            "<sender>",
            json!({"key": "hello", "value": "world"}),
            "../scripts/kv-set.execute.ts",
        )
        .await
        .unwrap();
        println!("{:?}", res);

        let res = run(
            Arc::clone(&store),
            RuntimeMode::Query,
            "<sender>",
            json!({"key": "hello"}),
            "../scripts/kv-get.query.ts",
        )
        .await
        .unwrap();
        println!("{:?}", res);
    }
}
