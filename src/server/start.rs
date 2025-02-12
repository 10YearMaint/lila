use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
}

async fn chat_handler(chat_req: web::Json<ChatRequest>) -> impl Responder {
    let prompt = chat_req.prompt.clone();
    let response_text = format!("Mistral says: You said \"{}\"", prompt);
    HttpResponse::Ok().json(ChatResponse {
        response: response_text,
    })
}

pub async fn start_server() -> std::io::Result<()> {
    println!("Starting backend server on http://127.0.0.1:8080");
    HttpServer::new(|| {
        App::new()
            // Enable permissive CORS. For production, tighten this up.
            .wrap(Cors::permissive())
            .route("/chat", web::post().to(chat_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
