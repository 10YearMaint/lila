use actix_web::HttpResponse;
use mistralrs::{
    IsqType, PagedAttentionMetaBuilder, Response, TextMessageRole, TextMessages, TextModelBuilder,
};
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::task;
use toml::Value as TomlValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
}

/// CLI arguments for the chat command.
#[derive(Debug, Deserialize)]
pub struct ChatArgs {
    pub prompt: Option<String>,
    pub no_db: bool,
    pub file_content: Option<String>,
}

/// Runs the chat command and returns an HttpResponse with the AI response in JSON.
pub async fn run_chat_response(args: ChatArgs) -> HttpResponse {
    // We'll spawn a blocking task so we don't tie up the async threads.
    let response_text = task::spawn_blocking(move || {
        // Log the received prompt and file.
        println!(
            "Processing chat request: prompt = {:?}, file_content is {} bytes",
            args.prompt,
            args.file_content.as_ref().map_or(0, |c| c.len())
        );

        // Build an inner runtime for blocking I/O
        let rt_inner = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        rt_inner.block_on(async {
            // -------------------------------------------------------------
            // 1. Get the "file_content" if provided.
            // -------------------------------------------------------------
            let context_content = match &args.file_content {
                Some(s) => s.clone(),
                None => String::new(),
            };

            // -------------------------------------------------------------
            // 2. Parse Lila.toml from the project root (optional).
            // -------------------------------------------------------------
            let lila_toml_path = "Lila.toml";
            let mut project_info = String::from("No [project] info found.");
            let mut development_info = String::from("No [development] info found.");
            let mut dependencies_info = String::from("No [dependencies] info found.");
            let mut compliance_info = String::from("No [compliance] info found.");
            let mut code_of_conduct = String::from("No code_of_conduct found.");

            if let Ok(lila_content) = fs::read_to_string(lila_toml_path) {
                if let Ok(toml_value) = toml::from_str::<TomlValue>(&lila_content) {
                    if let Some(val) = toml_value.get("project") {
                        project_info = format!("{:#?}", val);
                    }
                    if let Some(val) = toml_value.get("development") {
                        development_info = format!("{:#?}", val);
                    }
                    if let Some(val) = toml_value.get("dependencies") {
                        dependencies_info = format!("{:#?}", val);
                    }
                    if let Some(val) = toml_value.get("compliance") {
                        compliance_info = format!("{:#?}", val);
                    }
                    if let Some(ai_guidance) = toml_value.get("ai_guidance") {
                        if let Some(coc) = ai_guidance.get("code_of_conduct") {
                            if let Some(coc_str) = coc.as_str() {
                                code_of_conduct = coc_str.to_string();
                            }
                        }
                    }
                }
            }

            // -------------------------------------------------------------
            // 3. Extract prompt or bail if missing.
            // -------------------------------------------------------------
            let prompt = match &args.prompt {
                Some(p) => p.clone(),
                None => {
                    return format!("No prompt provided");
                }
            };

            // -------------------------------------------------------------
            // 4. Build/select your Mistral model.
            // -------------------------------------------------------------
            let model_id = std::env::var("LILA_AI_MODEL")
                .unwrap_or_else(|_| "microsoft/Phi-3.5-mini-instruct".to_string());
            println!("Using model={}", model_id);

            let model = match TextModelBuilder::new(model_id)
                .with_isq(IsqType::Q8_0)
                .with_logging()
                .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())
            {
                Ok(builder) => match builder.build().await {
                    Ok(m) => m,
                    Err(e) => {
                        println!("Error building model: {:?}", e);
                        return format!("Error building model: {:?}", e);
                    }
                },
                Err(e) => {
                    println!("Error creating model builder: {:?}", e);
                    return format!("Error creating model builder: {:?}", e);
                }
            };

            // -------------------------------------------------------------
            // 5. Construct the system message + the context
            // -------------------------------------------------------------
            let mut system_msg = if !context_content.is_empty() {
                "You are an AI agent with a specialty in programming.
                 You do not provide information outside of this scope.
                 If a question is not about programming, respond with, 'I can't assist you with that, sorry!'.
                 Below is some Markdown file content. Use it to answer the user's question."
                    .to_string()
            } else {
                "You are an AI agent with a specialty in programming.
                 You do not provide information outside of this scope.
                 If a question is not about programming, respond with, 'I can't assist you with that, sorry!'.
                 No additional context was provided."
                    .to_string()
            };

            // Append Lila.toml sections
            system_msg.push_str("\n---\n**Project**:\n");
            system_msg.push_str(&project_info);
            system_msg.push_str("\n\n**Development**:\n");
            system_msg.push_str(&development_info);
            system_msg.push_str("\n\n**Dependencies**:\n");
            system_msg.push_str(&dependencies_info);
            system_msg.push_str("\n\n**Compliance**:\n");
            system_msg.push_str(&compliance_info);
            system_msg.push_str("\n\n**AI Guidance Code of Conduct**:\n");
            system_msg.push_str(&code_of_conduct);
            system_msg.push_str("\n---\n");

            // -------------------------------------------------------------
            // 6. Build conversation (system + user).
            // -------------------------------------------------------------
            let messages = TextMessages::new()
                .add_message(TextMessageRole::System, &system_msg)
                .add_message(TextMessageRole::System, &context_content)
                .add_message(TextMessageRole::User, &prompt);

            // -------------------------------------------------------------
            // 7. Stream the AI response
            // -------------------------------------------------------------
            let mut stream = match model.stream_chat_request(messages).await {
                Ok(s) => s,
                Err(e) => {
                    println!("Error during stream: {:?}", e);
                    return format!("Error during stream: {:?}", e);
                }
            };

            let mut accumulated_response = String::new();
            while let Some(chunk) = stream.next().await {
                if let Response::Chunk(chunk) = chunk {
                    accumulated_response.push_str(&chunk.choices[0].delta.content);
                }
            }

            accumulated_response
        })
    })
    .await
    .unwrap_or_else(|e| format!("Error during spawn_blocking: {:?}", e));

    HttpResponse::Ok().json(ChatResponse {
        response: response_text,
    })
}
