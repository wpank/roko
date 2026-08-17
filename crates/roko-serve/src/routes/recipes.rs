//! Recipe persistence and pure evaluation API.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use roko_core::{Recipe, RecipeStore};
use serde::Serialize;
use serde_json::Value;

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/recipes", get(list_recipes).post(save_recipe))
        .route("/recipes/{id}", get(get_recipe).delete(delete_recipe))
        .route(
            "/recipes/{id}/evaluate",
            axum::routing::post(evaluate_recipe),
        )
}

#[derive(Debug, Serialize)]
struct RecipeList {
    recipes: Vec<String>,
    total: usize,
}

#[derive(Debug, Serialize)]
struct DeleteResponse {
    id: String,
    deleted: bool,
}

fn store(state: &AppState) -> RecipeStore {
    RecipeStore::new(state.workdir.join(".roko").join("recipes"))
}

async fn list_recipes(State(state): State<Arc<AppState>>) -> Result<Json<RecipeList>, ApiError> {
    let recipes = store(&state).list().map_err(internal)?;
    let total = recipes.len();
    Ok(Json(RecipeList { recipes, total }))
}

async fn get_recipe(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Recipe>, ApiError> {
    store(&state)
        .load(&id)
        .map(Json)
        .map_err(|error| ApiError::not_found(format!("recipe '{id}' not found: {error}")))
}

async fn save_recipe(
    State(state): State<Arc<AppState>>,
    Json(recipe): Json<Recipe>,
) -> Result<(StatusCode, Json<Recipe>), ApiError> {
    let saved = store(&state)
        .save(&recipe)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok((StatusCode::CREATED, Json(saved)))
}

async fn delete_recipe(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let deleted = store(&state).delete(&id).map_err(internal)?;
    if !deleted {
        return Err(ApiError::not_found(format!("recipe '{id}' not found")));
    }
    Ok(Json(DeleteResponse { id, deleted }))
}

async fn evaluate_recipe(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(inputs): Json<HashMap<String, Value>>,
) -> Result<Json<Value>, ApiError> {
    let recipe = store(&state)
        .load(&id)
        .map_err(|error| ApiError::not_found(format!("recipe '{id}' not found: {error}")))?;
    recipe
        .evaluate(&inputs)
        .map(Json)
        .map_err(|error| ApiError::bad_request(error.to_string()))
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;
    use roko_core::config::schema::RokoConfig;

    #[tokio::test]
    async fn empty_store_lists_cleanly() {
        let directory = tempfile::tempdir().unwrap();
        let backend = Arc::from(create_backend("manual", None, None, None).unwrap());
        let state = Arc::new(
            AppState::new(
                directory.path().into(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                backend,
            )
            .unwrap(),
        );
        let response = list_recipes(State(state)).await.unwrap().0;
        assert_eq!(response.total, 0);
        assert_eq!(response.recipes, Vec::<String>::new());
    }
}
