use crate::{
    api::{ApiState, Error},
    llm::context::Context,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use strfmt::strfmt;

/// An API query request payload.
#[derive(Deserialize)]
pub struct RequestPayload {
    query: String,
}

/// Handles API query requests.
#[axum::debug_handler]
pub async fn process_query(
    State(state): State<Arc<ApiState>>,
    WithRejection(Json(request), _): WithRejection<Json<RequestPayload>, Error>,
) -> Result<impl IntoResponse, Error> {
    let model: String = state.db.config_value("llm_model").await?;

    let context = serde_json::to_string(&Context::new()).unwrap();
    let mut vars = HashMap::new();
    vars.insert("context".to_owned(), context);
    vars.insert("query".to_owned(), request.query);
    let query: String = state.db.config_value("query_text").await?;
    let query = strfmt(&query, &vars).unwrap();

    let tools: Vec<_> = state.tools.values().map(|t| t.metadata()).collect();
    let call = state.llm.derive_tool_call(model, tools, query).await?;
    dbg!(&call);
    if let Some(call) = call {
        let (_, _) = (call.name, call.params);
    }

    Ok(StatusCode::OK)
}
