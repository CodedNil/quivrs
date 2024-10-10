use anyhow::Result;
use candle_core::{DType, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config, PositionEmbeddingType};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

pub struct ModelState {
    pub model: BertModel,
    pub tokenizer: tokenizers::Tokenizer,
}
pub static MODEL_STATE: LazyLock<Arc<Mutex<ModelState>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(
        init_embeddings().expect("Failed to initialize embeddings"),
    ))
});

fn init_embeddings() -> Result<ModelState> {
    let model_name = "jinaai/jina-embeddings-v2-base-en".to_string();
    let model = Api::new()?
        .repo(Repo::new(model_name.to_string(), RepoType::Model))
        .get("model.safetensors")?;
    let tokenizer = Api::new()?
        .repo(Repo::new(model_name, RepoType::Model))
        .get("tokenizer.json")?;
    let device = candle_core::Device::Cpu;
    let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer).map_err(anyhow::Error::msg)?;
    if let Some(pp) = tokenizer.get_padding_mut() {
        pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
    } else {
        let pp = tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        };
        tokenizer.with_padding(Some(pp));
    }
    let config = Config::new(
        tokenizer.get_vocab_size(true),
        768,
        12,
        12,
        3072,
        candle_nn::Activation::Gelu,
        8192,
        2,
        0.02,
        1e-12,
        0,
        PositionEmbeddingType::Alibi,
    );
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model], DType::F32, &device)? };
    let model = BertModel::new(vb, &config)?;

    Ok(ModelState { model, tokenizer })
}

pub async fn candle_test() -> Result<()> {
    let sentences = [
        "The cat sits outside",
        "A man is playing guitar",
        "I love pasta",
        "The new movie is awesome",
        "The cat plays in the garden",
        "A woman watches TV",
        "The new movie is so great",
        "Do you like pizza?",
    ];

    // Access the static MODEL_STATE
    let state = MODEL_STATE.lock().await;

    let tokenizer = &state.tokenizer;
    let model = &state.model;

    let n_sentences = sentences.len();
    let tokens = tokenizer
        .encode_batch(sentences.to_vec(), true)
        .map_err(anyhow::Error::msg)?;
    let token_ids = tokens
        .iter()
        .map(|tokens| {
            let tokens = tokens.get_ids().to_vec();
            Tensor::new(tokens.as_slice(), &model.device)
        })
        .collect::<candle_core::Result<Vec<_>>>()?;

    let token_ids = Tensor::stack(&token_ids, 0)?;
    println!("running inference on batch {:?}", token_ids.shape());
    let embeddings = model.forward(&token_ids)?;
    drop(state);

    println!("generated embeddings {:?}", embeddings.shape());
    // Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
    let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
    let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
    // Normalize embeddings
    let embeddings = embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?;
    println!("pooled embeddings {:?}", embeddings.shape());

    let mut similarities = vec![];
    for i in 0..n_sentences {
        let e_i = embeddings.get(i)?;
        for j in (i + 1)..n_sentences {
            let e_j = embeddings.get(j)?;
            let sum_ij = (&e_i * &e_j)?.sum_all()?.to_scalar::<f32>()?;
            let sum_i2 = (&e_i * &e_i)?.sum_all()?.to_scalar::<f32>()?;
            let sum_j2 = (&e_j * &e_j)?.sum_all()?.to_scalar::<f32>()?;
            let cosine_similarity = sum_ij / (sum_i2 * sum_j2).sqrt();
            similarities.push((cosine_similarity, i, j));
        }
    }
    similarities.sort_by(|u, v| v.0.total_cmp(&u.0));
    for &(score, i, j) in &similarities[..5] {
        println!("score: {score:.2} '{}' '{}'", sentences[i], sentences[j]);
    }

    Ok(())
}
