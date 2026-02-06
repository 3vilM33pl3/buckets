use crate::errors::BucketError;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokenizers::{PaddingParams, Tokenizer};

// Global lazy static for the model components
struct ModelComponents {
    model: BertModel,
    tokenizer: Tokenizer,
}

static MODEL: Lazy<Mutex<Option<ModelComponents>>> = Lazy::new(|| Mutex::new(None));

pub struct EmbeddingGenerator;

impl EmbeddingGenerator {
    /// Generates a vector embedding using all-MiniLM-L6-v2 via Candle
    pub fn generate(text: &str) -> Result<Vec<f32>, BucketError> {
        let mut guard = MODEL.lock().map_err(|_| {
            BucketError::EmbeddingError("Failed to acquire lock on embedding model".to_string())
        })?;

        if guard.is_none() {
            // Load model components
            let model_id = "sentence-transformers/all-MiniLM-L6-v2".to_string();
            let _revision = "refs/pr/21".to_string(); // Helper to pin version if needed, or use main

            let api = Api::new()
                .map_err(|e| BucketError::EmbeddingError(format!("Failed to create API: {}", e)))?;
            let repo = api.repo(Repo::new(model_id, RepoType::Model));

            let config_filename = repo
                .get("config.json")
                .map_err(|e| BucketError::EmbeddingError(format!("Failed to get config: {}", e)))?;
            let tokenizer_filename = repo.get("tokenizer.json").map_err(|e| {
                BucketError::EmbeddingError(format!("Failed to get tokenizer: {}", e))
            })?;
            let weights_filename = repo.get("model.safetensors").map_err(|e| {
                BucketError::EmbeddingError(format!("Failed to get weights: {}", e))
            })?;

            let config_str = std::fs::read_to_string(config_filename).map_err(|e| {
                BucketError::EmbeddingError(format!("Failed to read config file: {}", e))
            })?;
            let config: Config = serde_json::from_str(&config_str).map_err(|e| {
                BucketError::EmbeddingError(format!("Failed to parse config: {}", e))
            })?;
            let mut tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(|e| {
                BucketError::EmbeddingError(format!("Failed to load tokenizer: {}", e))
            })?;

            let device = Device::Cpu;
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_filename], DType::F32, &device)
            }
            .map_err(|e| BucketError::EmbeddingError(format!("Failed to load weights: {}", e)))?;

            let model = BertModel::load(vb, &config)
                .map_err(|e| BucketError::EmbeddingError(format!("Failed to load model: {}", e)))?;

            if let Some(pp) = tokenizer.get_padding_mut() {
                pp.strategy = tokenizers::PaddingStrategy::BatchLongest
            } else {
                let pp = PaddingParams {
                    strategy: tokenizers::PaddingStrategy::BatchLongest,
                    ..Default::default()
                };
                tokenizer.with_padding(Some(pp));
            }

            *guard = Some(ModelComponents { model, tokenizer });
        }

        let components = guard.as_ref().ok_or_else(|| {
            BucketError::EmbeddingError("Model components not initialized".to_string())
        })?;
        let device = &Device::Cpu;

        let tokenizer = &components.tokenizer;
        let model = &components.model;

        // Tokenize
        let tokens = tokenizer
            .encode(text, true)
            .map_err(|e| BucketError::EmbeddingError(format!("Tokenization failed: {}", e)))?;

        let token_ids = Tensor::new(tokens.get_ids(), device)
            .map_err(|e| BucketError::EmbeddingError(format!("Tensor error: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| BucketError::EmbeddingError(format!("Tensor error: {}", e)))?;

        // Inference
        let token_type_ids = token_ids
            .zeros_like()
            .map_err(|e| BucketError::EmbeddingError(format!("Tensor error: {}", e)))?;

        let embeddings = model
            .forward(&token_ids, &token_type_ids, None)
            .map_err(|e| BucketError::EmbeddingError(format!("Model forward failed: {}", e)))?;

        // Mean pooling (simple version)
        let (_n_sentence, n_tokens, _hidden_size) = embeddings
            .dims3()
            .map_err(|e| BucketError::EmbeddingError(format!("Dims error: {}", e)))?;

        let embeddings = (embeddings
            .sum(1)
            .map_err(|e| BucketError::EmbeddingError(format!("Sum error: {}", e)))?
            / (n_tokens as f64))
            .map_err(|e| BucketError::EmbeddingError(format!("Div error: {}", e)))?;

        let embeddings_vec = embeddings
            .squeeze(0)
            .map_err(|e| BucketError::EmbeddingError(format!("Squeeze error: {}", e)))?
            .to_vec1::<f32>()
            .map_err(|e| BucketError::EmbeddingError(format!("ToVec error: {}", e)))?;

        Ok(embeddings_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify embedding generator produces 384-dimension vectors (all-MiniLM-L6-v2)
    /// Note: Requires network on first run to download model (~90MB)
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_embedding_dimension() {
        let embedding = EmbeddingGenerator::generate("Test text for embedding")
            .expect("Failed to generate embedding");
        assert_eq!(
            embedding.len(),
            384,
            "Expected 384 dimensions from all-MiniLM-L6-v2"
        );
    }

    /// Verify similar texts produce similar embeddings
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_embedding_similarity() {
        let emb1 =
            EmbeddingGenerator::generate("The API should respond quickly").expect("embedding 1");
        let emb2 =
            EmbeddingGenerator::generate("API response time must be fast").expect("embedding 2");
        let emb3 = EmbeddingGenerator::generate("The weather is nice today").expect("embedding 3");

        // Cosine similarity helper
        fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            dot / (norm_a * norm_b)
        }

        let sim_related = cosine_similarity(&emb1, &emb2);
        let sim_unrelated = cosine_similarity(&emb1, &emb3);

        assert!(
            sim_related > sim_unrelated,
            "Related texts should be more similar: related={:.3} unrelated={:.3}",
            sim_related,
            sim_unrelated
        );
        assert!(
            sim_related > 0.7,
            "Related texts should have high similarity: {:.3}",
            sim_related
        );
    }
}
