use anyhow::{Error as E, Result};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::prelude::*;

use candle_transformers::models::mixformer::{
    Config as MixConfig, MixFormerSequentialForCausalLM as MixFormer,
};
use candle_transformers::models::phi::{Config as PhiConfig, Model as Phi};
use candle_transformers::models::phi3::{Config as Phi3Config, Model as Phi3};
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;

// Example Model enum from your snippet
enum Model {
    MixFormer(MixFormer),
    Phi(Phi),
    Phi3(Phi3),
    Quantized(QMixFormer),
}

// TextGeneration struct (same as in your snippet)
struct TextGeneration {
    model: Model,
    device: Device,
    tokenizer: candle_examples::token_output_stream::TokenOutputStream,
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
            tokenizer: candle_examples::token_output_stream::TokenOutputStream::new(tokenizer),
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
            .map_err(E::msg)?;
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
        print!("{prompt}");
        std::io::stdout().flush()?;
        let start_gen = std::time::Instant::now();
        let mut pos = 0;
        for index in 0..sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = match &mut self.model {
                Model::MixFormer(m) => m.forward(&input)?,
                Model::Phi(m) => m.forward(&input)?,
                Model::Phi3(m) => m.forward(&input, pos)?.i((.., 0, ..))?,
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
            pos += context_size;
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
        None => {
            // fallback logic - you can customize as needed
            if args.quantized {
                "lmz/candle-quantized-phi".to_string()
            } else {
                match args.model.as_str() {
                    "1" => "microsoft/phi-1".to_string(),
                    "1.5" => "microsoft/phi-1_5".to_string(),
                    "2" => "microsoft/phi-2".to_string(),
                    "3" => "microsoft/Phi-3-mini-4k-instruct".to_string(),
                    // fallback
                    _ => "microsoft/phi-2".to_string(),
                }
            }
        }
    };

    // Similarly for revision
    let revision = match args.revision {
        Some(r) => r,
        None => {
            if args.quantized {
                "main".to_string()
            } else {
                // example fallback
                "refs/pr/8".to_string()
            }
        }
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
            if args.quantized {
                // example fallback for quantized
                vec![repo.get("model-v2-q4k.gguf")?]
            } else {
                // example fallback for safetensors
                candle_examples::hub_load_safetensors(&repo, "model.safetensors.index.json")?
            }
        }
    };

    println!("Retrieved all necessary files in {:?}", start.elapsed());
    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

    // Load the model
    let start = std::time::Instant::now();

    // Example config for MixFormer (v2) if you’re not using a dynamic config.
    // If you want to parse a config file, do that here with e.g. `repo.get("config.json")?`
    let config = MixConfig::v2();

    let device = candle_examples::device(args.cpu)?;

    // Decide if we use quantized or normal
    let model = if args.quantized {
        // Use quantized var builder
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
            &filenames[0],
            &device,
        )?;
        // Example for MixFormer v2
        let mixformer = QMixFormer::new_v2(&config, vb)?;
        Model::Quantized(mixformer)
    } else {
        // Possibly parse dtype from string, or just default to F32
        let dtype = match &args.dtype {
            Some(dtype_str) => dtype_str.parse::<DType>()?,
            None => DType::F32,
        };
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };
        // Example building a “Phi3” or “MixFormer” depending on your `args.model`
        // For demonstration, we’re just building MixFormer v2
        let mixformer = MixFormer::new_v2(&config, vb)?;
        Model::MixFormer(mixformer)
    };
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
