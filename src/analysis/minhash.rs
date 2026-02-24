use lazy_static::lazy_static;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

const NUM_HASHES: usize = 128;

lazy_static! {
    static ref HASH_SEEDS: [u64; NUM_HASHES] = {
        let mut seeds = [0u64; NUM_HASHES];
        let mut rng = StdRng::seed_from_u64(42);  // Deterministic
        for seed in &mut seeds {
            *seed = rng.gen();
        }
        seeds
    };
}

#[derive(Debug, Clone)]
pub struct MinHashSignature {
    pub hashes: [u64; NUM_HASHES],
}

impl MinHashSignature {
    pub fn new() -> Self {
        Self {
            hashes: [u64::MAX; NUM_HASHES],
        }
    }
}

impl Default for MinHashSignature {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MinHash;

impl MinHash {
    pub fn compute_signature(text: &str) -> MinHashSignature {
        let mut sig = MinHashSignature::new();
        let shingles = Self::generate_shingles(text);
        
        if shingles.is_empty() {
            return sig;
        }
        
        for shingle in shingles {
            for (i, &seed) in HASH_SEEDS.iter().enumerate() {
                let hash = Self::hash_with_seed(&shingle, seed);
                if hash < sig.hashes[i] {
                    sig.hashes[i] = hash;
                }
            }
        }
        
        sig
    }
    
    fn generate_shingles(text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        
        if words.len() < 3 {
            // For short text, use individual words or pairs
            if words.is_empty() {
                return vec![];
            }
            if words.len() == 1 {
                return vec![words[0].to_lowercase()];
            }
            // 2 words: return both individual and pair
            return vec![
                words[0].to_lowercase(),
                words[1].to_lowercase(),
                format!("{} {}", words[0].to_lowercase(), words[1].to_lowercase()),
            ];
        }
        
        let mut shingles = Vec::with_capacity(words.len() - 2);
        for window in words.windows(3) {
            let shingle = format!(
                "{} {} {}",
                window[0].to_lowercase(),
                window[1].to_lowercase(),
                window[2].to_lowercase()
            );
            shingles.push(shingle);
        }
        
        shingles
    }
    
    fn hash_with_seed(text: &str, seed: u64) -> u64 {
        let mut hasher = FxHasher::default();
        seed.hash(&mut hasher);
        text.hash(&mut hasher);
        hasher.finish()
    }
    
    pub fn estimate_similarity(sig1: &MinHashSignature, sig2: &MinHashSignature) -> f64 {
        let matches = sig1.hashes.iter()
            .zip(sig2.hashes.iter())
            .filter(|(a, b)| a == b)
            .count();
        
        matches as f64 / NUM_HASHES as f64
    }
    
    pub fn find_clusters(
        signatures: &[MinHashSignature],
        scores: &[f64],
        threshold: f64,
    ) -> Vec<usize> {
        let n = signatures.len();
        if n == 0 {
            return vec![];
        }
        
        // Union-find
        let mut parent: Vec<usize> = (0..n).collect();
        let mut rank: Vec<usize> = vec![0; n];
        
        fn find(parent: &mut [usize], i: usize) -> usize {
            if parent[i] != i {
                parent[i] = find(parent, parent[i]);
            }
            parent[i]
        }
        
        fn union(parent: &mut [usize], rank: &mut [usize], i: usize, j: usize) {
            let root_i = find(parent, i);
            let root_j = find(parent, j);
            
            if root_i != root_j {
                if rank[root_i] < rank[root_j] {
                    parent[root_i] = root_j;
                } else if rank[root_i] > rank[root_j] {
                    parent[root_j] = root_i;
                } else {
                    parent[root_j] = root_i;
                    rank[root_i] += 1;
                }
            }
        }
        
        // Find similar pairs and union them
        for i in 0..n {
            for j in (i + 1)..n {
                let sim = Self::estimate_similarity(&signatures[i], &signatures[j]);
                if sim >= threshold {
                    union(&mut parent, &mut rank, i, j);
                }
            }
        }
        
        // Flatten and assign cluster representatives (highest score in cluster)
        for i in 0..n {
            find(&mut parent, i);
        }
        
        // For each cluster, find the representative (highest score)
        let mut cluster_reps: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        
        for i in 0..n {
            let root = parent[i];
            let current_rep = cluster_reps.get(&root).copied();
            match current_rep {
                Some(rep) => {
                    if scores[i] > scores[rep] {
                        cluster_reps.insert(root, i);
                    }
                }
                None => {
                    cluster_reps.insert(root, i);
                }
            }
        }
        
        // Return cluster assignments (using representative index)
        parent.iter().map(|&root| *cluster_reps.get(&root).unwrap_or(&root)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_text() {
        let text = "The quick brown fox jumps over the lazy dog";
        let sig1 = MinHash::compute_signature(text);
        let sig2 = MinHash::compute_signature(text);
        let sim = MinHash::estimate_similarity(&sig1, &sig2);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_different_text() {
        let text1 = "The quick brown fox jumps over the lazy dog";
        let text2 = "Lorem ipsum dolor sit amet consectetur adipiscing elit";
        let sig1 = MinHash::compute_signature(text1);
        let sig2 = MinHash::compute_signature(text2);
        let sim = MinHash::estimate_similarity(&sig1, &sig2);
        assert!(sim < 0.3);
    }

    #[test]
    fn test_similar_text() {
        let text1 = "The quick brown fox jumps over the lazy dog";
        let text2 = "The quick brown fox leaps over the lazy dog";
        let sig1 = MinHash::compute_signature(text1);
        let sig2 = MinHash::compute_signature(text2);
        let sim = MinHash::estimate_similarity(&sig1, &sig2);
        // With 3-word shingles, one word change reduces overlap significantly
        // 7 shingles in each, only 4-5 overlapping = ~0.3-0.5 similarity
        assert!(sim > 0.2);  // Should be somewhat similar
        assert!(sim < 1.0);  // But not identical
    }

    #[test]
    fn test_deterministic() {
        let text = "Testing deterministic behavior";
        let sig1 = MinHash::compute_signature(text);
        let sig2 = MinHash::compute_signature(text);
        assert_eq!(sig1.hashes, sig2.hashes);
    }

    #[test]
    fn test_empty_text() {
        let sig = MinHash::compute_signature("");
        assert_eq!(sig.hashes, [u64::MAX; NUM_HASHES]);
    }

    #[test]
    fn test_short_text() {
        let sig1 = MinHash::compute_signature("hello");
        let sig2 = MinHash::compute_signature("hello world");
        // Should not panic, should produce valid signatures
        assert!(sig1.hashes[0] < u64::MAX);
        assert!(sig2.hashes[0] < u64::MAX);
    }

    #[test]
    fn test_clustering() {
        let texts = [
            "The quick brown fox jumps over the lazy dog",
            "The quick brown fox leaps over the lazy dog",
            "Lorem ipsum dolor sit amet consectetur adipiscing elit",
        ];
        let signatures: Vec<_> = texts.iter().map(|t| MinHash::compute_signature(t)).collect();
        let scores = vec![0.5, 0.8, 0.3];
        // Use lower threshold since 3-word shingles have less overlap
        let clusters = MinHash::find_clusters(&signatures, &scores, 0.2);
        
        // First two should be in same cluster (similar), third different
        assert_eq!(clusters[0], clusters[1]);  // Same cluster representative
        // Third is its own cluster
        assert_ne!(clusters[2], clusters[0]);
    }

    #[test]
    fn test_cluster_representative() {
        let texts = [
            "The quick brown fox jumps over the lazy dog",
            "The quick brown fox leaps over the lazy dog",
        ];
        let signatures: Vec<_> = texts.iter().map(|t| MinHash::compute_signature(t)).collect();
        let scores = vec![0.3, 0.9];  // Second has higher score
        // Use lower threshold since 3-word shingles have less overlap
        let clusters = MinHash::find_clusters(&signatures, &scores, 0.2);
        
        // Both should point to index 1 (higher score)
        assert_eq!(clusters[0], 1);
        assert_eq!(clusters[1], 1);
    }

    #[test]
    fn test_empty_clusters() {
        let clusters = MinHash::find_clusters(&[], &[], 0.5);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_shingle_generation() {
        let shingles = MinHash::generate_shingles("one two three four");
        assert_eq!(shingles.len(), 2);  // "one two three", "two three four"
    }
}
