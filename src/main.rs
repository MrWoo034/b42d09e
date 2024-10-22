use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    // Use individual member locks to help avoid dead lock conditions
    pub db: Arc<RwLock<HashMap<String, Movie>>>,
    pub cache: Arc<RwLock<HashMap<String, Movie>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Movie {
    pub id: String,
    pub name: String,
    pub year: u16,
    pub was_good: bool,
}

async fn get_movie(
    State(state): State<AppState>,
    Path(movie_id): Path<String>,
) -> Result<Json<Movie>, StatusCode> {
    // Scope so we don't hold the read lock
    {
        let locked_cache = state.cache.read().await;
        if let Some(res) = locked_cache.get(&movie_id) {
            return Ok(Json(res.clone()));
        }
    }

    let locked_db = state.db.read().await;
    if let Some(movie) = locked_db.get(&movie_id) {
        // Scope for lock
        {
            let mut locked_cache = state.cache.write().await;
            locked_cache.insert(movie_id, movie.clone());
        }
        println!("Found Movie: {:?}", movie);
        Ok(Json(movie.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn add_movie(State(state): State<AppState>, Json(movie): Json<Movie>) -> StatusCode {
    let mut locked_self = state.db.write().await;
    if locked_self.insert(movie.id.clone(), movie).is_some() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    }
}

// Create Axum server with the following endpoints:
// 1. GET /movie/{id} - This should return back a movie given the id
// 2. POST /movie - this should save movie in a DB (HashMap<String, Movie>). This movie will be sent
// via a JSON payload.
// As a bonus: implement a caching layer so we don't need to make expensive "DB" lookups, etc.
#[tokio::main]
async fn main() {
    let state = AppState {
        db: Arc::new(RwLock::new(HashMap::default())),
        cache: Arc::new(RwLock::new(HashMap::default())),
    };
    let app = Router::new()
        .route("/movie/:movie_id", get(get_movie))
        .route("/movie", post(add_movie))
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
