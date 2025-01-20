use anyhow::{Error, Result};
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::prelude::*;

use candle_transformers::models::mixformer::{
    Config as MixConfig, MixFormerSequentialForCausalLM as MixFormer,
};
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;

use candle_core::utils::{cuda_is_available, metal_is_available};

pub fn device(cpu: bool) -> Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else if cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            println!(
                "Running on CPU, to run on GPU(metal), build this example with `--features metal`"
            );
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            println!("Running on CPU, to run on GPU, build this example with `--features cuda`");
        }
        Ok(Device::Cpu)
    }
}

/// This is a wrapper around a tokenizer to ensure that tokens can be returned to the user in a
/// streaming way rather than having to wait for the full decoding.
pub struct TokenOutputStream {
    tokenizer: tokenizers::Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl TokenOutputStream {
    pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
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

    // https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68
    pub fn next_token(&mut self, token: u32) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
            let text = text.split_at(prev_text.len());
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text.1.to_string()))
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
            let text = text.split_at(prev_text.len());
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn get_token(&self, token_s: &str) -> Option<u32> {
        self.tokenizer.get_vocab(true).get(token_s).copied()
    }

    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }
}

enum Model {
    MixFormer(MixFormer),
    Quantized(QMixFormer),
}

// TextGeneration struct (same as in your snippet)
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
        tokenizer: tokenizers::Tokenizer,
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
        println!("starting the inference loop");
        let tokens = self
            .tokenizer
            .tokenizer()
            .encode(prompt, true)
            .map_err(Error::msg)?;
        if tokens.is_empty() {
            anyhow::bail!("Empty prompts are not supported in the phi model.")
        }
        if self.verbose_prompt {
            for (token, id) in tokens.get_tokens().iter().zip(tokens.get_ids().iter()) {
                let token = token.replace('▁', " ").replace("<0x0A>", "\n");
                println!("{id:7} -> '{token}'");
            }
        }
        let mut tokens = tokens.get_ids().to_vec();
        let mut generated_tokens = 0usize;
        let eos_token = match self.tokenizer.get_token("<|endoftext|>") {
            Some(token) => token,
            None => anyhow::bail!("cannot find the endoftext token"),
        };

        std::io::stdout().flush()?;
        let start_gen = std::time::Instant::now();
        let mut _pos = 0;
        for index in 0..sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = match &mut self.model {
                Model::MixFormer(m) => m.forward(&input)?,
                Model::Quantized(m) => m.forward(&input)?,
            };
            let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
            let logits = if self.repeat_penalty == 1. {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(self.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.repeat_penalty,
                    &tokens[start_at..],
                )?
            };

            let next_token = self.logits_processor.sample(&logits)?;
            tokens.push(next_token);
            generated_tokens += 1;
            if next_token == eos_token {
                if let Some(t) = self.tokenizer.decode_rest()? {
                    print!("{t}");
                    std::io::stdout().flush()?;
                }
                break;
            }
            if let Some(t) = self.tokenizer.next_token(next_token)? {
                print!("{t}");
                std::io::stdout().flush()?;
            }
            _pos += context_size;
        }
        let dt = start_gen.elapsed();
        println!(
            "\n{generated_tokens} tokens generated ({:.2} token/s)",
            generated_tokens as f64 / dt.as_secs_f64(),
        );
        Ok(())
    }
}

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

/// The main entry point for the Chat functionality
pub fn run_chat(args: ChatArgs) -> Result<()> {
    // Enable (or not) tracing if requested
    let _guard = if args.tracing {
        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };

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
    let api = Api::new()?;

    // Example: figure out model_id
    let model_id = match args.model_id {
        Some(m) => m,
        None => "lmz/candle-quantized-phi".to_string(),
    };

    // Similarly for revision
    let revision = match args.revision {
        Some(r) => r,
        None => "main".to_string(),
    };

    // Build the repo reference with HF Hub
    let repo = api.repo(Repo::with_revision(
        model_id.clone(),
        RepoType::Model,
        revision.clone(),
    ));
    println!("Using model_id={model_id}, revision={revision}");

    // tokenizer filename
    let tokenizer_filename = match args.tokenizer {
        Some(path) => std::path::PathBuf::from(path),
        None => repo.get("tokenizer.json")?,
    };

    // handle weight_file or download from HF
    let filenames = match args.weight_file {
        Some(weight_file) => vec![std::path::PathBuf::from(weight_file)],
        None => {
            vec![repo.get("model-v2-q4k.gguf")?]
        }
    };

    println!("Retrieved all necessary files in {:?}", start.elapsed());
    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(Error::msg)?;

    // Load the model
    let start = std::time::Instant::now();

    // Example config for MixFormer (v2) if you’re not using a dynamic config.
    // If you want to parse a config file, do that here with e.g. `repo.get("config.json")?`
    let config = MixConfig::v2();

    let device = device(args.cpu)?;

    // Use quantized var builder
    let vb =
        candle_transformers::quantized_var_builder::VarBuilder::from_gguf(&filenames[0], &device)?;
    // Example for MixFormer v2
    let mixformer = QMixFormer::new_v2(&config, vb)?;
    let model = Model::Quantized(mixformer);

    println!("Loaded the model in {:?}", start.elapsed());

    // If you have more logic for MMLU or anything else, you can replicate it here.
    // We'll do a straightforward “if we have a prompt, do the chat”
    let prompt = match &args.prompt {
        Some(p) => p,
        None => {
            anyhow::bail!("No prompt given. (If you have MMLU logic, put it here.)");
        }
    };

    // Create the pipeline and run it
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
