use anyhow::{Result, anyhow};
use burn::backend::wgpu::{Wgpu, WgpuDevice};
use minilm_burn::{MiniLmModel, MiniLmVariant, mean_pooling, normalize_l2, tokenize_batch};
use std::sync::{LazyLock, Mutex};
use tokenizers::Tokenizer;

struct EmbedState {
    model: MiniLmModel<Wgpu>,
    tokenizer: Tokenizer,
    device: WgpuDevice,
}

static STATE: LazyLock<Mutex<EmbedState>> = LazyLock::new(|| {
    let device = WgpuDevice::default();
    let (model, tokenizer) =
        MiniLmModel::<Wgpu>::pretrained(&device, MiniLmVariant::default(), None)
            .expect("Failed to load MiniLM model");
    Mutex::new(EmbedState {
        model,
        tokenizer,
        device,
    })
});

pub async fn get_embedding(text: String) -> Result<Vec<f32>> {
    let state = STATE.lock().map_err(|_| anyhow!("Lock poisoned"))?;

    let (ids, mask) = tokenize_batch::<Wgpu>(&state.tokenizer, &[&text], &state.device);
    let output = state.model.forward(ids, mask.clone(), None);

    drop(state);
    let embeddings = normalize_l2(mean_pooling(output.hidden_states, mask));

    embeddings
        .to_data()
        .as_slice::<f32>()
        .map(<[f32]>::to_vec)
        .map_err(|e| anyhow!("Failed to read embedding data: {e:?}"))
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}
