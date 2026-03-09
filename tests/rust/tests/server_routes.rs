use axum::body::Body;
use axum::http::{Request, StatusCode};
use composer_server::{build_app, AppState};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    let db = composer_db::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    db.run_migrations().await.unwrap();
    let (event_bus, persist_rx) = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(Arc::new(db), event_bus.clone(), persist_rx);
    let state = Arc::new(AppState {
        services,
        event_bus,
    });
    let default_origins = composer_config::ComposerConfig::default().cors.origins;
    build_app(state, &default_origins)
}

async fn body_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// --- Health ---

#[tokio::test]
async fn health_check() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["status"], "ok");
}

// --- Tasks ---

#[tokio::test]
async fn task_list_empty() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn task_create() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Test Task"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["title"], "Test Task");
    assert_eq!(json["status"], "backlog");
}

#[tokio::test]
async fn task_create_invalid_body() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    // Missing required field "title" → 422 Unprocessable Entity from Axum
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn task_get_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/tasks/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn task_create_get_update_delete() {
    let app = setup_app().await;

    // Create
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"CRUD Task"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let task = body_json(resp.into_body()).await;
    let task_id = task["id"].as_str().unwrap();

    // Get
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/tasks/{}", task_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/tasks/{}", task_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Updated Task"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated = body_json(resp.into_body()).await;
    assert_eq!(updated["title"], "Updated Task");

    // Delete
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}", task_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify deleted
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/tasks/{}", task_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn task_move() {
    let app = setup_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Move Me"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let task = body_json(resp.into_body()).await;
    let task_id = task["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/move", task_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"status":"done"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let moved = body_json(resp.into_body()).await;
    assert_eq!(moved["status"], "done");
}

#[tokio::test]
async fn task_assign() {
    let app = setup_app().await;

    // Create agent first
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let agent = body_json(resp.into_body()).await;
    let agent_id = agent["id"].as_str().unwrap();

    // Create task
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Assign Me"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let task = body_json(resp.into_body()).await;
    let task_id = task["id"].as_str().unwrap();

    // Assign
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/assign", task_id))
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"agent_id":"{}"}}"#, agent_id)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let assigned = body_json(resp.into_body()).await;
    assert_eq!(assigned["assigned_agent_id"], agent_id);
}

#[tokio::test]
async fn task_create_with_auto_approve() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"With Auto Approve"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["title"], "With Auto Approve");
    assert_eq!(json["auto_approve"], true);
}

#[tokio::test]
async fn task_start_requires_agent_and_project() {
    let app = setup_app().await;

    // Create task without agent/project
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"No Config"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let task = body_json(resp.into_body()).await;
    let task_id = task["id"].as_str().unwrap();

    // Try to start — should fail with 400 (no agent assigned)
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/start", task_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn task_list_sessions_for_nonexistent() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/tasks/00000000-0000-0000-0000-000000000000/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- Agents ---

#[tokio::test]
async fn agent_list_empty() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn agent_create_and_get() {
    let app = setup_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"My Agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let agent = body_json(resp.into_body()).await;
    assert_eq!(agent["name"], "My Agent");
    let agent_id = agent["id"].as_str().unwrap();

    // Get
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/agents/{}", agent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn agent_get_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn agent_delete() {
    let app = setup_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Del Agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let agent = body_json(resp.into_body()).await;
    let agent_id = agent["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/agents/{}", agent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify deleted
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/agents/{}", agent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn agent_health_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents/00000000-0000-0000-0000-000000000000/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_server_error() || resp.status().is_client_error());
}

// --- Sessions ---

/// Setup app + return a DB handle for direct test data seeding.
async fn setup_app_with_db() -> (axum::Router, Arc<composer_db::Database>) {
    let db = Arc::new(
        composer_db::Database::connect("sqlite::memory:")
            .await
            .unwrap(),
    );
    db.run_migrations().await.unwrap();
    let (event_bus, persist_rx) = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(db.clone(), event_bus.clone(), persist_rx);
    let state = Arc::new(AppState {
        services,
        event_bus,
    });
    let default_origins = composer_config::ComposerConfig::default().cors.origins;
    (build_app(state, &default_origins), db)
}

#[tokio::test]
async fn session_get_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/sessions/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn session_logs_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/sessions/00000000-0000-0000-0000-000000000000/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Logs endpoint returns 200 with a PaginatedSessionLogs containing an empty list
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["logs"].as_array().unwrap().is_empty());
    assert_eq!(json["has_more"], false);
    assert_eq!(json["total_count"], 0);
}

// --- Session input ---

#[tokio::test]
async fn session_input_nonexistent_session_returns_error() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/00000000-0000-0000-0000-000000000000/input")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_server_error());
}

#[tokio::test]
async fn session_input_missing_message_field_returns_422() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/00000000-0000-0000-0000-000000000000/input")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    // Missing required field "message" → 422 Unprocessable Entity
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn session_input_on_non_running_session_returns_error() {
    let (app, db) = setup_app_with_db().await;

    // Seed an agent and a session with "paused" status directly in DB
    let agent = composer_db::models::agent::create(
        &db.pool, "TestAgent", &composer_api_types::AgentType::ClaudeCode, None,
    ).await.unwrap();
    let agent_id = agent.id.to_string();

    let session_id = uuid::Uuid::new_v4().to_string();
    composer_db::models::session::create_with_status(
        &db.pool,
        &session_id,
        &agent_id,
        None,
        None,
        "test prompt",
        &composer_api_types::SessionStatus::Paused,
        None,
    )
    .await
    .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{}/input", session_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_server_error());
}

#[tokio::test]
async fn session_input_on_completed_session_returns_error() {
    let (app, db) = setup_app_with_db().await;

    let agent = composer_db::models::agent::create(
        &db.pool, "TestAgent", &composer_api_types::AgentType::ClaudeCode, None,
    ).await.unwrap();
    let agent_id = agent.id.to_string();

    let session_id = uuid::Uuid::new_v4().to_string();
    composer_db::models::session::create_with_status(
        &db.pool,
        &session_id,
        &agent_id,
        None,
        None,
        "test prompt",
        &composer_api_types::SessionStatus::Completed,
        None,
    )
    .await
    .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{}/input", session_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_server_error());
}

#[tokio::test]
async fn session_input_on_running_session_without_process_returns_error() {
    let (app, db) = setup_app_with_db().await;

    let agent = composer_db::models::agent::create(
        &db.pool, "TestAgent", &composer_api_types::AgentType::ClaudeCode, None,
    ).await.unwrap();
    let agent_id = agent.id.to_string();

    let session_id = uuid::Uuid::new_v4().to_string();
    composer_db::models::session::create_with_status(
        &db.pool,
        &session_id,
        &agent_id,
        None,
        None,
        "test prompt",
        &composer_api_types::SessionStatus::Running,
        None,
    )
    .await
    .unwrap();

    // Session is Running in DB but no real process exists.
    // send_input will fail because no process is in the DashMap.
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/sessions/{}/input", session_id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_server_error());
}

// --- Error response shape ---

#[tokio::test]
async fn error_response_has_error_field() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/tasks/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
    let json = body_json(resp.into_body()).await;
    assert!(json.get("error").is_some());
}
