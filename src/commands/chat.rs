use anyhow::{Error, Result};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::prelude::*;

use candle_core::utils::{cuda_is_available, metal_is_available};
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::mixformer::{
    Config as MixConfig, MixFormerSequentialForCausalLM as MixFormer,
};
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;

// ==================================================
// 1. CLI args
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
// 2. Device selection
// ==================================================
fn device(cpu: bool) -> Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else if cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            println!("Running on CPU, to run on GPU(metal), build with `--features metal`");
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            println!("Running on CPU, to run on GPU, build with `--features cuda`");
        }
        Ok(Device::Cpu)
    }
}

// ==================================================
// 3. Token output streaming
// ==================================================
/// A helper to decode output tokens incrementally, so you can
/// stream text generation without waiting for all tokens.
pub struct TokenOutputStream {
    tokenizer: Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl TokenOutputStream {
    pub fn new(tokenizer: Tokenizer) -> Self {
        Self {
            tokenizer,
            tokens: Vec::new(),
            prev_index: 0,
            current_index: 0,
        }
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        match self.tokenizer.decode(tokens, true) {
            Ok(str) => Ok(str),
            Err(err) => anyhow::bail!("cannot decode: {err}"),
        }
    }

    pub fn next_token(&mut self, token: u32) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        // Example heuristic used: if the next char is alphanumeric and extends text, return it
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
            let text_diff = text.split_at(prev_text.len()).1.to_string();
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text_diff))
        } else {
            Ok(None)
        }
    }

    pub fn decode_rest(&self) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() {
            let text_diff = text.split_at(prev_text.len()).1.to_string();
            Ok(Some(text_diff))
        } else {
            Ok(None)
        }
    }

    pub fn get_token(&self, token_s: &str) -> Option<u32> {
        self.tokenizer.get_vocab(true).get(token_s).copied()
    }

    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }
}

// ==================================================
// 4. Model abstraction
// ==================================================
/// If you want to add a new Hugging Face model, add it here as a new variant
/// and implement the logic in the `forward` calls.
enum Model {
    MixFormer(MixFormer),
    Quantized(QMixFormer),
    // MyNewAwesomeModel(MyAwesomeStruct),  <-- example for future extension
}

impl Model {
    /// The forward pass must return the logits (shape [batch_size, seq_len, vocab_size]).
    fn forward(&mut self, input: &Tensor) -> Result<Tensor> {
        match self {
            Model::MixFormer(ref mut m) => Ok(m.forward(input)?),
            Model::Quantized(ref mut m) => Ok(m.forward(input)?),
            // Add other models here as needed
        }
    }
}

// ==================================================
// 5. The text generation pipeline
// ==================================================
struct TextGeneration {
    model: Model,
    device: Device,
    tokenizer: TokenOutputStream,
    logits_processor: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
    verbose_prompt: bool,
}

impl TextGeneration {
    #[allow(clippy::too_many_arguments)]
    fn new(
        model: Model,
        tokenizer: Tokenizer,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        repeat_penalty: f32,
        repeat_last_n: usize,
        verbose_prompt: bool,
        device: &Device,
    ) -> Self {
        let logits_processor = LogitsProcessor::new(seed, temp, top_p);
        Self {
            model,
            tokenizer: TokenOutputStream::new(tokenizer),
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            verbose_prompt,
            device: device.clone(),
        }
    }

    fn run(&mut self, prompt: &str, sample_len: usize) -> Result<()> {
        use std::io::Write;
        println!("Starting the inference loop...");

        let encoded = self
            .tokenizer
            .tokenizer()
            .encode(prompt, true)
            .map_err(Error::msg)?;
        if encoded.is_empty() {
            anyhow::bail!("Empty prompts are not supported in the phi model.")
        }

        // Optionally print debug info about prompt tokens
        if self.verbose_prompt {
            for (token, id) in encoded.get_tokens().iter().zip(encoded.get_ids().iter()) {
                let token_display = token.replace('▁', " ").replace("<0x0A>", "\n");
                println!("{id:7} -> '{token_display}'");
            }
        }

        let mut tokens = encoded.get_ids().to_vec();
        let eos_token = self
            .tokenizer
            .get_token("<|endoftext|>")
            .ok_or_else(|| anyhow::anyhow!("Cannot find the <|endoftext|> token"))?;
        let mut generated_tokens = 0usize;

        // Start generation
        std::io::stdout().flush()?;
        let start_gen = std::time::Instant::now();

        for index in 0..sample_len {
            // For the first token we take the entire sequence as context; afterwards only last token
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];

            // 1) Forward pass
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = self
                .model
                .forward(&input)?
                .squeeze(0)?
                .to_dtype(DType::F32)?;

            // 2) Optional repeat penalty
            let logits = if self.repeat_penalty == 1.0 {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(self.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.repeat_penalty,
                    &tokens[start_at..],
                )?
            };

            // 3) Sample next token
            let next_token = self.logits_processor.sample(&logits)?;
            tokens.push(next_token);
            generated_tokens += 1;

            // 4) Check for end-of-text
            if next_token == eos_token {
                if let Some(output) = self.tokenizer.decode_rest()? {
                    print!("{output}");
                    std::io::stdout().flush()?;
                }
                break;
            }

            // 5) Print any newly generated text incrementally
            if let Some(output) = self.tokenizer.next_token(next_token)? {
                print!("{output}");
                std::io::stdout().flush()?;
            }
        }

        let dt = start_gen.elapsed();
        println!(
            "\n{generated_tokens} tokens generated in {:.2}s ({:.2} token/s)",
            dt.as_secs_f64(),
            generated_tokens as f64 / dt.as_secs_f64()
        );

        Ok(())
    }
}

// ==================================================
// 6. Main entry point for Chat
// ==================================================
pub fn run_chat(args: ChatArgs) -> Result<()> {
    // 6.1 Enable (or not) tracing
    let _guard = if args.tracing {
        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };

    // 6.2 Print some debug info
    println!(
        "avx: {}, neon: {}, simd128: {}, f16c: {}",
        candle_core::utils::with_avx(),
        candle_core::utils::with_neon(),
        candle_core::utils::with_simd128(),
        candle_core::utils::with_f16c()
    );
    println!(
        "temp: {:.2} repeat-penalty: {:.2} repeat-last-n: {}",
        args.temperature.unwrap_or(0.0),
        args.repeat_penalty,
        args.repeat_last_n
    );

    let start = std::time::Instant::now();

    // 6.3 Setup huggingface repo references
    let api = Api::new()?;
    let model_id = args.model_id.clone().unwrap_or_else(|| {
        // fallback if none given
        "lmz/candle-quantized-phi".to_string()
    });
    let revision = args.revision.clone().unwrap_or_else(|| "main".to_string());
    let repo = api.repo(Repo::with_revision(
        model_id.clone(),
        RepoType::Model,
        revision.clone(),
    ));
    println!("Using model_id={model_id}, revision={revision}");

    // 6.4 Tokenizer file
    let tokenizer_filename = match &args.tokenizer {
        Some(path) => std::path::PathBuf::from(path),
        None => repo.get("tokenizer.json")?,
    };

    // 6.5 Weight file
    let weight_files = match &args.weight_file {
        Some(weight_file) => vec![std::path::PathBuf::from(weight_file)],
        None => {
            // If we haven't provided a local weight file, fetch from HF
            vec![repo.get("model-v2-q4k.gguf")?]
        }
    };

    println!("Retrieved necessary files in {:?}", start.elapsed());

    // 6.6 Load tokenizer
    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(Error::msg)?;

    // 6.7 Load model
    let start = std::time::Instant::now();
    let device = device(args.cpu)?;

    // Example: if you always want the quantized version:
    // (Here’s where you could add a “match” on `args.model` or `args.quantized`
    //  to load a different builder or a different HF model.)
    let config = MixConfig::v2();
    let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
        &weight_files[0],
        &device,
    )?;
    let mixformer_q = QMixFormer::new_v2(&config, vb)?;
    let model = Model::Quantized(mixformer_q);

    println!("Loaded the model in {:?}", start.elapsed());

    // 6.8 Retrieve or bail if no prompt
    let prompt = match &args.prompt {
        Some(p) => p,
        None => anyhow::bail!("No prompt provided. Cannot run chat."),
    };

    // 6.9 Create pipeline and run
    let mut pipeline = TextGeneration::new(
        model,
        tokenizer,
        args.seed,
        args.temperature,
        args.top_p,
        args.repeat_penalty,
        args.repeat_last_n,
        args.verbose_prompt,
        &device,
    );

    pipeline.run(prompt, args.sample_len)?;

    Ok(())
}
