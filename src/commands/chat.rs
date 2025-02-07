use anyhow::Result;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use dotenvy::dotenv;
use std::env;
use std::path::Path;

use crate::commands::save::establish_connection;
use crate::schema::{file_content, metadata};

use mistralrs::{
    IsqType, PagedAttentionMetaBuilder, Response, TextMessageRole, TextMessages, TextModelBuilder,
};

// ==================================================
// CLI args
// ==================================================
#[derive(Debug)]
pub struct ChatArgs {
    pub prompt: Option<String>,
    pub model_id: Option<String>,
    pub no_db: bool,
}

// =============================================
// Helper function: Load all Markdown data from DB
// =============================================
fn load_all_markdown_data() -> Result<Vec<(String, String)>, DieselError> {
    // 1) Load environment to read LILA_OUTPUT_PATH
    dotenv().ok(); // This loads .env if found

    // 2) Grab the base folder from the .env variable
    let base_path = env::var("LILA_OUTPUT_PATH").map_err(|_| DieselError::NotFound)?;

    // 3) Build the path to db/lila.db
    let db_path = Path::new(&base_path).join("lila.db");
    let db_path_str = db_path.to_string_lossy();

    // 4) Establish connection using existing function
    let mut conn = establish_connection(&db_path_str);

    // 5) Perform join on both tables -> (file_path, content)
    let rows = metadata::table
        .inner_join(file_content::table.on(file_content::id.eq(metadata::id)))
        .select((metadata::file_path, file_content::content))
        .load::<(String, String)>(&mut conn)?;

    Ok(rows)
}

// =============================================
// Main entry point for Chat
// =============================================
#[tokio::main]
pub async fn run_chat(args: ChatArgs) -> Result<()> {
    // Conditionally load DB data
    let db_content = if !args.no_db {
        // If user didn't disable DB, load data
        match load_all_markdown_data() {
            Ok(data) => {
                // Join them into a single string
                data.into_iter()
                    .map(|(file_path, content)| format!("File: {}\n{}", file_path, content))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
            Err(e) => {
                eprintln!("Failed to load Markdown data: {:?}", e);
                return Err(e.into());
            }
        }
    } else {
        // No DB was requested
        String::new()
    };

    let prompt = match &args.prompt {
        Some(p) => p,
        None => anyhow::bail!("No prompt provided. Cannot run chat."),
    };

    let model_id = args.model_id.clone().unwrap_or_else(|| {
        std::env::var("LILA_AI_MODEL")
            .unwrap_or_else(|_| "microsoft/Phi-3.5-mini-instruct".to_string())
    });
    println!("Using model={model_id}");

    let model = TextModelBuilder::new(model_id)
        .with_isq(IsqType::Q8_0)
        .with_logging()
        .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
        .build()
        .await?;

    let messages = TextMessages::new()
        .add_message(
            TextMessageRole::System,
            "
            You are an AI agent with a specialty in programming.
            You do not provide information outside of this scope.
            If a question is not about programming, respond with, 'I can't assist you with that, sorry!'.
            Here are some Markdown documents from the DB. Use them to answer questions.
            ",
        )
        .add_message(TextMessageRole::System, &db_content)
        .add_message(TextMessageRole::User, prompt);

    let mut stream = model.stream_chat_request(messages).await?;

    while let Some(chunk) = stream.next().await {
        if let Response::Chunk(chunk) = chunk {
            print!("{}", chunk.choices[0].delta.content);
        }
    }

    Ok(())
}
