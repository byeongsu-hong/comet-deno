use std::{cell::RefCell, rc::Rc};

use deno_core::{error::AnyError, op2, OpState};
use tendermint_proto::abci::Event;

use crate::runtime::OpStateContext;

#[op2(async)]
#[string]
#[allow(clippy::await_holding_refcell_ref)]
pub(crate) async fn op_kv_set(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
    #[string] value: String,
) -> Result<String, AnyError> {
    let state = state.borrow_mut();
    let ctx: &OpStateContext = state.borrow();

    ctx.mode.assert_execute()?;

    let mut store = ctx.store.lock().await;
    let resp = store.set(key, value.clone()).await?;

    Ok(resp.unwrap_or(value))
}

#[op2(async)]
#[string]
#[allow(clippy::await_holding_refcell_ref)]
pub(crate) async fn op_kv_get(
    state: Rc<RefCell<OpState>>,
    #[string] key: String,
) -> Result<String, AnyError> {
    let state = state.borrow_mut();
    let ctx: &OpStateContext = state.borrow();

    let store = ctx.store.lock().await;
    let res = store.get(key).await?;

    res.ok_or(AnyError::msg("key not found"))
}

#[op2]
pub(crate) fn op_ctx_emit(
    #[state] ctx: &mut OpStateContext,
    #[serde] event: Event,
) -> Result<(), AnyError> {
    ctx.mode.assert_execute()?;

    ctx.events.push(event);

    Ok(())
}

#[op2]
pub(crate) fn op_ctx_respond(
    #[state] ctx: &mut OpStateContext,
    #[serde] response: serde_json::Value,
) -> Result<(), AnyError> {
    ctx.mode.assert_query()?;

    match ctx.response {
        Some(_) => Err(AnyError::msg("respond already called")),
        None => {
            ctx.response = Some(response);
            Ok(())
        }
    }
}

#[op2]
#[string]
pub(crate) fn op_ctx_get_sender(#[state] ctx: &OpStateContext) -> Result<String, AnyError> {
    Ok(ctx.sender.clone())
}

#[op2]
#[serde]
pub(crate) fn op_ctx_get_request(
    #[state] ctx: &OpStateContext,
) -> Result<serde_json::Value, AnyError> {
    Ok(ctx.request.clone())
}
