use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use sha2::{Digest, Sha256};

use crate::{GuidError, Result};

pub fn generate_guid(id: &str) -> String {
    let mut hasher = Sha256::new();

    match validate_id(id) {
        Ok(validated_id) => {
            hasher.update(validated_id);
            let hash_result = hasher.finalize();

            let hash_to_base64 = general_purpose::URL_SAFE.encode(&hash_result);

            let base64_regex = Regex::new(r"/").unwrap();
            let guid = base64_regex.replace_all(&hash_to_base64, "_");

            guid.to_string()
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn validate_id(id: &str) -> Result<String> {
    if id.len() != 17 {
        return Err(GuidError::InvalidLength);
    } else if id.starts_with("7656119") == false {
        return Err(GuidError::InvalidPrefix);
    } else if id.chars().all(|c| c.is_digit(10)) == false {
        return Err(GuidError::InvalidCharacters);
    }

    Ok(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_guid() {
        let steam64id: &str = "76561198039479170";
        let expected_guid = "VmZuANA0NjRb6XNrwgOZsaRtzk3Xy2DFd91Usr8Q61E=";
        let generated_guid = generate_guid(steam64id);
        assert_eq!(generated_guid, expected_guid);
    }

    #[test]
    fn test_validate_id_valid() {
        let valid_id = "76561198000000000";
        assert_eq!(validate_id(valid_id), Ok(valid_id.to_string()));
    }

    #[test]
    fn test_validate_id_invalid_length() {
        let invalid_id = "7656119800000000";
        assert_eq!(validate_id(invalid_id), Err(GuidError::InvalidLength));
    }

    #[test]
    fn test_validate_id_invalid_prefix() {
        let invalid_id = "86561198000000000";
        assert_eq!(validate_id(invalid_id), Err(GuidError::InvalidPrefix));
    }

    #[test]
    fn test_validate_id_invalid_characters() {
        let invalid_id = "76561198000000abc";
        assert_eq!(validate_id(invalid_id), Err(GuidError::InvalidCharacters));
    }
}
