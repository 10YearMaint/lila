use actix_web::HttpResponse;
use anyhow::Result;
use futures::StreamExt;
use mistralrs::{
    IsqType, PagedAttentionMetaBuilder, Response, TextMessageRole, TextMessages, TextModelBuilder,
};
use serde::Serialize;
use std::fs;
use std::path::Path;
use tokio::task;

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
}

/// CLI arguments for the chat command.
#[derive(Debug)]
pub struct ChatArgs {
    pub prompt: Option<String>,
    pub no_db: bool,
    /// Optional: Specify a Markdown file whose content will be used as context.
    pub file: Option<String>,
}

/// Runs the chat command and returns an HttpResponse with the AI response in JSON.
/// This version wraps the blocking operations in spawn_blocking and accumulates the streaming output.
pub async fn run_chat_response(args: ChatArgs) -> HttpResponse {
    let response_text = task::spawn_blocking(move || {
        // Log the received prompt and file.
        println!(
            "Processing chat request: prompt = {:?} and file = {:?}",
            args.prompt, args.file
        );

        // Create an inner multi-threaded runtime for the blocking operations.
        let rt_inner = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        rt_inner.block_on(async {
            // Read context from file if provided.
            let context_content = if let Some(ref file_path) = args.file {
                fs::read_to_string(file_path).unwrap_or_else(|_| String::new())
            } else {
                String::new()
            };

            // Extract prompt; if missing, return an error string.
            let prompt = match &args.prompt {
                Some(p) => p.clone(),
                None => return format!("No prompt provided"),
            };

            // Always ignore any model_id provided by the client.
            let model_id = std::env::var("LILA_AI_MODEL")
                .unwrap_or_else(|_| "microsoft/Phi-3.5-mini-instruct".to_string());
            println!("Using model={}", model_id);

            // Build the model.
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

            // Prepare the system message based on whether a file was provided.
            let system_msg = if args.file.is_some() {
                "You are an AI agent with a specialty in programming.
                 You do not provide information outside of this scope.
                 If a question is not about programming, respond with, 'I can't assist you with that, sorry!'.
                 Below is the content of a specific Markdown file. Use it to answer the user's question."
                    .to_string()
            } else {
                "You are an AI agent with a specialty in programming.
                 You do not provide information outside of this scope.
                 If a question is not about programming, respond with, 'I can't assist you with that, sorry!'.
                 No additional context was provided."
                    .to_string()
            };

            // Build the conversation messages.
            let messages = TextMessages::new()
                .add_message(TextMessageRole::System, &system_msg)
                .add_message(TextMessageRole::System, &context_content)
                .add_message(TextMessageRole::User, &prompt);

            // Attempt to stream the AI response.
            let mut stream = match model.stream_chat_request(messages).await {
                Ok(s) => s,
                Err(e) => {
                    println!("Error during stream: {:?}", e);
                    return format!("Error during stream: {:?}", e);
                }
            };

            // Accumulate all chunks from the stream.
            let mut accumulated_response = String::new();
            while let Some(chunk) = stream.next().await {
                if let Response::Chunk(chunk) = chunk {
                    accumulated_response.push_str(&chunk.choices[0].delta.content);
                }
            }

            // Return the accumulated response.
            accumulated_response
        })
    })
    .await
    .unwrap_or_else(|e| format!("Error during spawn_blocking: {:?}", e));

    HttpResponse::Ok().json(ChatResponse {
        response: response_text,
    })
}
