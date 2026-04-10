//! Eigen-Tune Curator — dataset building pipeline.
//!
//! Transforms a raw collection of `TrainingPair`s into balanced, deduplicated
//! train/eval sets ready for fine-tuning. The trainer (`engine::trainer`)
//! consumes the output of `build_training_dataset()`.
//!
//! Pipeline stages:
//! 1. Load high-quality pairs from the store
//! 2. Dedup by SHA-256 hash of `messages_json` (exact match only)
//! 3. Compute diversity entropy (gating condition)
//! 4. Optionally balance categories via Thompson sampling
//! 5. Stratified holdout split (per `EigenTier × domain_category`)
//! 6. Export train + valid + eval sets to ChatML JSONL files

use crate::config::EigenTuneConfig;
use crate::stats::{entropy, thompson::ThompsonSampler};
use crate::store::EigenTuneStore;
use crate::types::{EigenTier, TrainingPair};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use temm1e_core::types::error::Temm1eError;

/// Output of the full curation pipeline.
#[derive(Debug, Clone)]
pub struct CuratorOutput {
    pub train_path: PathBuf,
    pub valid_path: PathBuf,
    pub eval_path: PathBuf,
    pub train_count: usize,
    pub valid_count: usize,
    pub eval_count: usize,
    pub diversity_j: f64,
    pub category_distribution: Vec<(String, f64)>,
}

/// Load all pairs for a tier with quality score above the threshold.
pub async fn load_tier_pairs(
    store: &EigenTuneStore,
    tier: &str,
    min_quality: f64,
) -> Result<Vec<TrainingPair>, Temm1eError> {
    store.get_pairs_for_tier(tier, min_quality).await
}

/// Remove exact duplicates by SHA-256 hash of `messages_json`.
/// Preserves first-occurrence order.
pub fn dedup_by_messages_hash(pairs: Vec<TrainingPair>) -> Vec<TrainingPair> {
    let mut seen: HashSet<String> = HashSet::with_capacity(pairs.len());
    let mut result: Vec<TrainingPair> = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let mut hasher = Sha256::new();
        hasher.update(pair.messages_json.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        if seen.insert(hash) {
            result.push(pair);
        }
    }
    result
}

/// Compute normalized Shannon entropy J for the category distribution.
/// Returns a value in [0.0, 1.0] where 1.0 is perfectly uniform.
pub fn compute_diversity_entropy(pairs: &[TrainingPair]) -> f64 {
    let mut category_counts: HashMap<String, u64> = HashMap::new();
    for pair in pairs {
        let cat = pair
            .domain_category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        *category_counts.entry(cat).or_insert(0) += 1;
    }
    let counts: Vec<u64> = category_counts.values().copied().collect();
    entropy::normalized_entropy(&counts)
}

/// Balance category representation via Thompson sampling.
///
/// Each domain category becomes an arm in a multi-armed bandit. Pairs are
/// sampled proportionally to the posterior expected quality per category,
/// using each pair's `quality_score` (>= 0.6 → reward) as the SPRT signal.
///
/// `target_count` is the maximum number of pairs to return; if the input
/// has fewer pairs, returns all of them.
///
/// `rng_seed` makes the function deterministic for tests; pass None for
/// production thread_rng-based sampling.
pub fn balance_by_thompson_sampling(
    pairs: &[TrainingPair],
    target_count: usize,
    rng_seed: Option<u64>,
) -> Vec<TrainingPair> {
    if pairs.is_empty() || target_count == 0 {
        return Vec::new();
    }
    if pairs.len() <= target_count {
        return pairs.to_vec();
    }

    // Group by domain_category, preserving original ordering
    let mut buckets: HashMap<String, Vec<TrainingPair>> = HashMap::new();
    let mut category_order: Vec<String> = Vec::new();
    for pair in pairs {
        let cat = pair
            .domain_category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        if !buckets.contains_key(&cat) {
            category_order.push(cat.clone());
        }
        buckets.entry(cat).or_default().push(pair.clone());
    }

    let k = category_order.len();
    let mut sampler = ThompsonSampler::new(k);
    let mut selected: Vec<TrainingPair> = Vec::with_capacity(target_count);

    // Two RNG paths: deterministic seeded (for tests) vs thread_rng (production)
    if let Some(seed) = rng_seed {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        while selected.len() < target_count {
            // Stop if every bucket is exhausted
            if category_order
                .iter()
                .all(|c| buckets.get(c).map(|b| b.is_empty()).unwrap_or(true))
            {
                break;
            }
            let arm_idx = sampler.sample_with_rng(&mut rng);
            let cat = &category_order[arm_idx];
            if let Some(bucket) = buckets.get_mut(cat) {
                if bucket.is_empty() {
                    // Discourage further sampling of this arm
                    sampler.update(arm_idx, false);
                    continue;
                }
                let idx = rng.gen_range(0..bucket.len());
                let pair = bucket.swap_remove(idx);
                let reward = pair.quality_score.unwrap_or(0.5) >= 0.6;
                sampler.update(arm_idx, reward);
                selected.push(pair);
            }
        }
    } else {
        let mut rng = rand::thread_rng();
        while selected.len() < target_count {
            if category_order
                .iter()
                .all(|c| buckets.get(c).map(|b| b.is_empty()).unwrap_or(true))
            {
                break;
            }
            let arm_idx = sampler.sample_with_rng(&mut rng);
            let cat = &category_order[arm_idx];
            if let Some(bucket) = buckets.get_mut(cat) {
                if bucket.is_empty() {
                    sampler.update(arm_idx, false);
                    continue;
                }
                let idx = rng.gen_range(0..bucket.len());
                let pair = bucket.swap_remove(idx);
                let reward = pair.quality_score.unwrap_or(0.5) >= 0.6;
                sampler.update(arm_idx, reward);
                selected.push(pair);
            }
        }
    }

    selected
}

/// Stratified holdout split by `(EigenTier, domain_category)` tuple.
///
/// Returns `(eval_pairs, train_pairs)`. Each eval pair is marked
/// `is_eval_holdout = true`. Stratification preserves the category and
/// tier distribution across both splits.
///
/// `holdout_pct` should be in (0.0, 1.0). Pass `rng_seed` for deterministic
/// behavior in tests.
pub fn split_holdout_set(
    pairs: Vec<TrainingPair>,
    holdout_pct: f64,
    rng_seed: Option<u64>,
) -> (Vec<TrainingPair>, Vec<TrainingPair>) {
    if pairs.is_empty() || holdout_pct <= 0.0 || holdout_pct >= 1.0 {
        return (Vec::new(), pairs);
    }

    let mut strata: HashMap<(EigenTier, String), Vec<TrainingPair>> = HashMap::new();
    for pair in pairs {
        let cat = pair
            .domain_category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        strata.entry((pair.complexity, cat)).or_default().push(pair);
    }

    let mut eval_pairs: Vec<TrainingPair> = Vec::new();
    let mut train_pairs: Vec<TrainingPair> = Vec::new();

    if let Some(seed) = rng_seed {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for (_, mut bucket) in strata {
            bucket.shuffle(&mut rng);
            let holdout_n = ((bucket.len() as f64) * holdout_pct).ceil() as usize;
            for (i, mut pair) in bucket.into_iter().enumerate() {
                if i < holdout_n {
                    pair.is_eval_holdout = true;
                    eval_pairs.push(pair);
                } else {
                    pair.is_eval_holdout = false;
                    train_pairs.push(pair);
                }
            }
        }
    } else {
        let mut rng = rand::thread_rng();
        for (_, mut bucket) in strata {
            bucket.shuffle(&mut rng);
            let holdout_n = ((bucket.len() as f64) * holdout_pct).ceil() as usize;
            for (i, mut pair) in bucket.into_iter().enumerate() {
                if i < holdout_n {
                    pair.is_eval_holdout = true;
                    eval_pairs.push(pair);
                } else {
                    pair.is_eval_holdout = false;
                    train_pairs.push(pair);
                }
            }
        }
    }

    (eval_pairs, train_pairs)
}

/// Export pairs to ChatML JSONL format.
///
/// Format: one JSON object per line, shape `{"messages": [<chatml messages>]}`.
/// The `messages` value is parsed from each pair's stored `messages_json`
/// (which is already in ChatML form from collection time) and re-serialized.
///
/// Returns the number of lines written.
pub async fn export_chatml_jsonl(
    pairs: &[TrainingPair],
    output_path: &Path,
) -> Result<usize, Temm1eError> {
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            Temm1eError::Tool(format!("curator: create_dir_all {}: {e}", parent.display()))
        })?;
    }

    let mut content = String::with_capacity(pairs.len() * 256);
    let mut written: usize = 0;

    for pair in pairs {
        // Parse the stored messages_json. Skip pairs whose messages don't parse.
        let messages: serde_json::Value = match serde_json::from_str(&pair.messages_json) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    pair_id = %pair.id,
                    error = %e,
                    "curator: skipping pair with malformed messages_json"
                );
                continue;
            }
        };
        let row = serde_json::json!({ "messages": messages });
        let line = serde_json::to_string(&row)
            .map_err(|e| Temm1eError::Tool(format!("curator: serialize jsonl line: {e}")))?;
        content.push_str(&line);
        content.push('\n');
        written += 1;
    }

    tokio::fs::write(output_path, content)
        .await
        .map_err(|e| Temm1eError::Tool(format!("curator: write {}: {e}", output_path.display())))?;

    Ok(written)
}

/// Validate a ChatML JSONL file produced by `export_chatml_jsonl`.
///
/// Returns `(valid_count, total_count)`. A line is valid iff it parses as
/// `{"messages": [...]}` with each message having a valid `role` in
/// `{"system", "user", "assistant", "tool"}` and a `content` field.
pub fn validate_chatml_jsonl(file_path: &Path) -> Result<(usize, usize), Temm1eError> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| Temm1eError::Tool(format!("curator: read {}: {e}", file_path.display())))?;
    let mut total = 0usize;
    let mut valid = 0usize;
    const VALID_ROLES: &[&str] = &["system", "user", "assistant", "tool"];

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let messages = match parsed.get("messages").and_then(|v| v.as_array()) {
            Some(m) if !m.is_empty() => m,
            _ => continue,
        };
        let mut all_messages_valid = true;
        for msg in messages {
            let role = msg.get("role").and_then(|v| v.as_str());
            let has_content = msg.get("content").is_some();
            if !role.map(|r| VALID_ROLES.contains(&r)).unwrap_or(false) || !has_content {
                all_messages_valid = false;
                break;
            }
        }
        if all_messages_valid {
            valid += 1;
        }
    }

    Ok((valid, total))
}

/// Top-level curation pipeline.
///
/// Sequence: load → dedup → diversity gate → optional balance → stratified
/// holdout split → write three files (train.jsonl, valid.jsonl, eval.jsonl).
///
/// `output_dir` should be a fresh directory; the trainer creates one per
/// run under `config.artifacts_dir`.
///
/// Returns a `CuratorOutput` with file paths and metrics. Errors if:
/// - The store query fails
/// - Fewer than `config.min_pairs` pairs after dedup
/// - Diversity entropy below `config.diversity_target`
/// - File writes fail
pub async fn build_training_dataset(
    store: &EigenTuneStore,
    config: &EigenTuneConfig,
    tier: EigenTier,
    output_dir: &Path,
) -> Result<CuratorOutput, Temm1eError> {
    // Stage 1: Load
    let raw_pairs = load_tier_pairs(store, tier.as_str(), config.quality_threshold).await?;

    // Stage 2: Dedup
    let deduped = dedup_by_messages_hash(raw_pairs);

    if deduped.len() < config.min_pairs as usize {
        return Err(Temm1eError::Tool(format!(
            "curator: insufficient pairs after dedup ({} < {})",
            deduped.len(),
            config.min_pairs
        )));
    }

    // Stage 3: Diversity gate
    let diversity_j = compute_diversity_entropy(&deduped);
    if diversity_j < config.diversity_target {
        return Err(Temm1eError::Tool(format!(
            "curator: diversity entropy {:.3} below target {:.3}",
            diversity_j, config.diversity_target
        )));
    }

    // Stage 4: Balance (optional, only if we have more than min_pairs * 2)
    // For MVP, we keep balancing simple: only balance if we have abundant data.
    let balanced = if deduped.len() > (config.min_pairs as usize) * 2 {
        let target = (config.min_pairs as usize) * 2;
        balance_by_thompson_sampling(&deduped, target, None)
    } else {
        deduped
    };

    // Stage 5: Stratified holdout split
    let (eval_pairs, train_and_valid) = split_holdout_set(balanced, config.eval_holdout_pct, None);

    // Take 10% of training set as in-loop validation set (for trainer eval-during-training)
    let valid_count = (train_and_valid.len() / 10).max(1);
    let valid_pairs: Vec<TrainingPair> =
        train_and_valid.iter().take(valid_count).cloned().collect();
    let train_pairs: Vec<TrainingPair> = train_and_valid.into_iter().skip(valid_count).collect();

    // Stage 6: Compute final category distribution for the report
    let mut all_cats: HashMap<String, u64> = HashMap::new();
    for p in &train_pairs {
        let cat = p
            .domain_category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        *all_cats.entry(cat).or_insert(0) += 1;
    }
    let train_total = train_pairs.len() as f64;
    let mut category_distribution: Vec<(String, f64)> = all_cats
        .into_iter()
        .map(|(cat, n)| {
            (
                cat,
                if train_total > 0.0 {
                    n as f64 / train_total
                } else {
                    0.0
                },
            )
        })
        .collect();
    category_distribution
        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Stage 7: Export
    let train_path = output_dir.join("train.jsonl");
    let valid_path = output_dir.join("valid.jsonl");
    let eval_path = output_dir.join("eval.jsonl");

    let train_count = export_chatml_jsonl(&train_pairs, &train_path).await?;
    let valid_written = export_chatml_jsonl(&valid_pairs, &valid_path).await?;
    let eval_count = export_chatml_jsonl(&eval_pairs, &eval_path).await?;

    tracing::info!(
        tier = %tier.as_str(),
        train = train_count,
        valid = valid_written,
        eval = eval_count,
        diversity_j,
        "curator: dataset built"
    );

    Ok(CuratorOutput {
        train_path,
        valid_path,
        eval_path,
        train_count,
        valid_count: valid_written,
        eval_count,
        diversity_j,
        category_distribution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EigenTier, TrainingPair};
    use chrono::Utc;

    fn make_pair(id: &str, tier: EigenTier, category: &str, quality: f64) -> TrainingPair {
        TrainingPair {
            id: id.to_string(),
            conversation_id: "conv-1".to_string(),
            turn: 1,
            created_at: Utc::now(),
            messages_json: format!(
                r#"[{{"role":"user","content":"hello {id}"}},{{"role":"assistant","content":"hi"}}]"#
            ),
            system_prompt: Some("You are helpful.".to_string()),
            tools_json: None,
            response_json: r#"{"role":"assistant","content":"hi"}"#.to_string(),
            source_model: "claude-sonnet-4-20250514".to_string(),
            source_provider: "anthropic".to_string(),
            complexity: tier,
            domain_category: Some(category.to_string()),
            quality_alpha: 2.0,
            quality_beta: 2.0,
            quality_score: Some(quality),
            user_continued: None,
            user_retried: None,
            tool_success: None,
            response_error: None,
            tokens_in: Some(10),
            tokens_out: Some(20),
            cost_usd: Some(0.001),
            dataset_version: None,
            is_eval_holdout: false,
        }
    }

    #[test]
    fn dedup_removes_exact_duplicates() {
        let mut p1 = make_pair("a", EigenTier::Simple, "coding", 0.8);
        let mut p2 = make_pair("b", EigenTier::Simple, "coding", 0.8);
        p1.messages_json = r#"[{"role":"user","content":"identical"}]"#.to_string();
        p2.messages_json = r#"[{"role":"user","content":"identical"}]"#.to_string();
        let p3 = make_pair("c", EigenTier::Simple, "coding", 0.8);
        let pairs = vec![p1, p2, p3];
        let deduped = dedup_by_messages_hash(pairs);
        assert_eq!(deduped.len(), 2, "expected 2 unique pairs after dedup");
        assert_eq!(deduped[0].id, "a");
        assert_eq!(deduped[1].id, "c");
    }

    #[test]
    fn dedup_preserves_order() {
        let pairs = (0..5)
            .map(|i| make_pair(&format!("p{i}"), EigenTier::Simple, "math", 0.7))
            .collect();
        let deduped = dedup_by_messages_hash(pairs);
        assert_eq!(deduped.len(), 5);
        for (i, p) in deduped.iter().enumerate() {
            assert_eq!(p.id, format!("p{i}"));
        }
    }

    #[test]
    fn compute_diversity_entropy_uniform_returns_one() {
        let pairs: Vec<TrainingPair> = (0..4)
            .flat_map(|i| {
                let cat = format!("cat{i}");
                (0..10).map(move |j| make_pair(&format!("p{i}-{j}"), EigenTier::Simple, &cat, 0.8))
            })
            .collect();
        let j = compute_diversity_entropy(&pairs);
        assert!((j - 1.0).abs() < 1e-9, "expected J=1.0, got {}", j);
    }

    #[test]
    fn compute_diversity_entropy_monoculture_returns_zero() {
        let pairs: Vec<TrainingPair> = (0..20)
            .map(|i| make_pair(&format!("p{i}"), EigenTier::Simple, "only", 0.8))
            .collect();
        let j = compute_diversity_entropy(&pairs);
        assert!(j < 1e-9, "expected J=0.0 for monoculture, got {}", j);
    }

    #[test]
    fn split_holdout_pct_15_yields_about_15pct_eval() {
        let pairs: Vec<TrainingPair> = (0..100)
            .map(|i| {
                let cat = if i % 2 == 0 { "a" } else { "b" };
                make_pair(&format!("p{i}"), EigenTier::Standard, cat, 0.8)
            })
            .collect();
        let (eval, train) = split_holdout_set(pairs, 0.15, Some(42));
        let total = eval.len() + train.len();
        assert_eq!(total, 100);
        // ceil(50 * 0.15) per stratum = 8 each, so 16 total
        assert_eq!(eval.len(), 16);
        assert_eq!(train.len(), 84);
    }

    #[test]
    fn split_holdout_marks_is_eval_holdout() {
        let pairs: Vec<TrainingPair> = (0..20)
            .map(|i| make_pair(&format!("p{i}"), EigenTier::Simple, "math", 0.8))
            .collect();
        let (eval, train) = split_holdout_set(pairs, 0.2, Some(7));
        for p in &eval {
            assert!(p.is_eval_holdout, "eval pair {} not marked", p.id);
        }
        for p in &train {
            assert!(!p.is_eval_holdout, "train pair {} marked", p.id);
        }
    }

    #[test]
    fn split_holdout_is_stratified_per_category() {
        // 30 pairs across 3 categories (10 each)
        let pairs: Vec<TrainingPair> = (0..30)
            .map(|i| {
                let cat = match i % 3 {
                    0 => "a",
                    1 => "b",
                    _ => "c",
                };
                make_pair(&format!("p{i}"), EigenTier::Simple, cat, 0.8)
            })
            .collect();
        let (eval, _train) = split_holdout_set(pairs, 0.2, Some(99));
        // ceil(10 * 0.2) = 2 per category × 3 = 6 total
        assert_eq!(eval.len(), 6);
        let mut by_cat: HashMap<String, usize> = HashMap::new();
        for p in &eval {
            *by_cat
                .entry(p.domain_category.clone().unwrap_or_default())
                .or_insert(0) += 1;
        }
        assert_eq!(by_cat.get("a"), Some(&2));
        assert_eq!(by_cat.get("b"), Some(&2));
        assert_eq!(by_cat.get("c"), Some(&2));
    }

    #[tokio::test]
    async fn export_chatml_jsonl_one_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.jsonl");
        let pairs: Vec<TrainingPair> = (0..3)
            .map(|i| make_pair(&format!("p{i}"), EigenTier::Simple, "math", 0.8))
            .collect();
        let written = export_chatml_jsonl(&pairs, &path).await.unwrap();
        assert_eq!(written, 3);
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.get("messages").is_some());
            assert!(parsed["messages"].is_array());
        }
    }

    #[tokio::test]
    async fn export_chatml_jsonl_validates_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.jsonl");
        let pairs: Vec<TrainingPair> = (0..5)
            .map(|i| make_pair(&format!("p{i}"), EigenTier::Standard, "code", 0.7))
            .collect();
        export_chatml_jsonl(&pairs, &path).await.unwrap();
        let (valid, total) = validate_chatml_jsonl(&path).unwrap();
        assert_eq!(valid, 5);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn build_training_dataset_full_pipeline_inmem() {
        let store = EigenTuneStore::new("sqlite::memory:").await.unwrap();
        // Seed 60 pairs across 3 categories and 1 tier
        for i in 0..60 {
            let cat = match i % 3 {
                0 => "coding",
                1 => "reasoning",
                _ => "factual",
            };
            let mut p = make_pair(&format!("p{i:03}"), EigenTier::Simple, cat, 0.8);
            p.id = format!("p{i:03}");
            store.save_pair(&p).await.unwrap();
        }
        // Build a config that allows the small dataset
        let cfg = EigenTuneConfig {
            min_pairs: 30,
            diversity_target: 0.5,
            eval_holdout_pct: 0.2,
            quality_threshold: 0.6,
            ..EigenTuneConfig::default()
        };

        let dir = tempfile::tempdir().unwrap();
        let out = build_training_dataset(&store, &cfg, EigenTier::Simple, dir.path())
            .await
            .unwrap();

        assert!(out.train_count > 0);
        assert!(out.eval_count > 0);
        assert!(out.train_path.exists());
        assert!(out.eval_path.exists());
        assert!(out.diversity_j >= cfg.diversity_target);
        // Round-trip validation
        let (v, t) = validate_chatml_jsonl(&out.train_path).unwrap();
        assert_eq!(v, t);
        assert!(t > 0);
    }
}
