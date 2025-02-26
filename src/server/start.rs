use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

use crate::server::chat::{run_chat_response, ChatArgs};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub file_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
}

async fn chat_handler(chat_req: web::Json<ChatRequest>) -> impl Responder {
    let args = ChatArgs {
        prompt: Some(chat_req.prompt.clone()),
        no_db: true, // Always disable DB loading.
        file_content: chat_req.file_content.clone(),
    };

    run_chat_response(args).await
}

async fn ping_handler() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

pub async fn start_server() -> std::io::Result<()> {
    println!("Starting backend server on http://127.0.0.1:8080");
    HttpServer::new(|| {
        App::new()
            .wrap(Cors::permissive())
            .route("/ping", web::get().to(ping_handler))
            .route("/chat", web::post().to(chat_handler))
    })
    .workers(4) // Ensure multi-threaded workers.
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
