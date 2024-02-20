use base64::prelude::*;
use rand::{distributions::Uniform, Rng};
use sha2::{Digest, Sha256};
// sha256('nouce_str' + ?) = '0' * difficulty + ...
pub struct POW {
    pub nonce_str: Option<String>,
    difficulty: Option<usize>,
}
impl POW {
    pub fn init(difficulty: usize) -> Self {
        let mut rng = rand::thread_rng();
        let dist = Uniform::new(0, 256);
        let random_bytes: Vec<u8> = (0..6).map(|_| rng.sample(&dist) as u8).collect();
        let nouce_str = BASE64_STANDARD.encode(&random_bytes);
        POW {
            nonce_str: Some(nouce_str),
            difficulty: Some(difficulty),
        }
    }
    pub fn verify(&self, nonce: String) -> bool {
        let mut hasher = Sha256::new();
        let mut input = self.nonce_str.clone().unwrap();
        input.push_str(&nonce);
        hasher.update(input);
        let result = hasher.finalize();
        let result_str = format!("{:x}", result);
        if &result_str[..self.difficulty.unwrap()] == "0".repeat(self.difficulty.unwrap()) {
            return true;
        }
        return false;
    }
}
