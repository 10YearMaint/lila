use anyhow::{Error, Result};
use hf_hub::{api::sync::Api, Repo, RepoType};
use mistralrs::{
    IsqType, PagedAttentionMetaBuilder, Response, TextMessageRole, TextMessages, TextModelBuilder,
};

// ==================================================
// CLI args
// ==================================================
#[derive(Debug)]
pub struct ChatArgs {
    pub cpu: bool,
    pub tracing: bool,
    pub verbose_prompt: bool,
    pub prompt: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub seed: u64,
    pub sample_len: usize,
    pub model_id: Option<String>,
    pub model: String,
    pub revision: Option<String>,
    pub weight_file: Option<String>,
    pub tokenizer: Option<String>,
    pub quantized: bool,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub dtype: Option<String>,
}

// ==================================================
// Main entry point for Chat
// ==================================================
#[tokio::main]
pub async fn run_chat(args: ChatArgs) -> Result<()> {
    let prompt = match &args.prompt {
        Some(p) => p,
        None => anyhow::bail!("No prompt provided. Cannot run chat."),
    };

    let model_id = args.model_id.clone().unwrap_or_else(|| {
        // fallback if none given
        "microsoft/Phi-3.5-mini-instruct".to_string()
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
            ",
        )
        .add_message(TextMessageRole::User, prompt);

    let mut stream = model.stream_chat_request(messages).await?;

    while let Some(chunk) = stream.next().await {
        if let Response::Chunk(chunk) = chunk {
            print!("{}", chunk.choices[0].delta.content);
        }
    }
    Ok(())
}
