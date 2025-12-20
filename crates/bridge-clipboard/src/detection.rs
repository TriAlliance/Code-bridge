//! Sensitive content detection for clipboard

use crate::ContentType;
use regex::Regex;
use std::sync::LazyLock;

/// Patterns that indicate sensitive content
static SENSITIVE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // API Keys
        Regex::new(r"(?i)(api[_-]?key|apikey|api_secret)[=:]\s*['\"]?[a-zA-Z0-9_\-]{20,}").unwrap(),
        Regex::new(r"sk-[a-zA-Z0-9]{32,}").unwrap(), // OpenAI
        Regex::new(r"sk_live_[a-zA-Z0-9]{24,}").unwrap(), // Stripe
        Regex::new(r"rk_live_[a-zA-Z0-9]{24,}").unwrap(), // Stripe
        Regex::new(r"ghp_[a-zA-Z0-9]{36,}").unwrap(), // GitHub Personal Access Token
        Regex::new(r"gho_[a-zA-Z0-9]{36,}").unwrap(), // GitHub OAuth Token
        Regex::new(r"github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}").unwrap(), // GitHub Fine-grained PAT

        // AWS
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), // AWS Access Key
        Regex::new(r"(?i)aws[_-]?secret[_-]?access[_-]?key[=:]\s*['\"]?[a-zA-Z0-9/+=]{40}").unwrap(),

        // Private Keys
        Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
        Regex::new(r"-----BEGIN PGP PRIVATE KEY BLOCK-----").unwrap(),

        // Passwords
        Regex::new(r"(?i)(password|passwd|pwd)[=:]\s*['\"]?[^\s'\"]{8,}").unwrap(),
        Regex::new(r"(?i)(secret|token)[=:]\s*['\"]?[a-zA-Z0-9_\-]{16,}").unwrap(),

        // Database connection strings
        Regex::new(r"(?i)(mongodb|postgres|mysql|redis)://[^@\s]+:[^@\s]+@").unwrap(),
        Regex::new(r"(?i)postgres://[^\s]+").unwrap(),

        // JWT tokens
        Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap(),

        // Credit card numbers (basic pattern)
        Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b").unwrap(),

        // SSN
        Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),

        // Bearer tokens
        Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_\-.]+").unwrap(),

        // SSH keys
        Regex::new(r"ssh-(rsa|ed25519|ecdsa)\s+[A-Za-z0-9+/=]+").unwrap(),
    ]
});

/// Check if content might be sensitive
pub fn is_sensitive(data: &[u8], content_type: &ContentType) -> bool {
    // Only check text-based content
    match content_type {
        ContentType::Text | ContentType::Code { .. } | ContentType::RichText => {}
        _ => return false,
    }

    // Try to convert to string
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Check against sensitive patterns
    for pattern in SENSITIVE_PATTERNS.iter() {
        if pattern.is_match(text) {
            return true;
        }
    }

    false
}

/// Classify the type of sensitive content
pub fn classify_sensitive(data: &[u8]) -> Option<SensitiveType> {
    let text = std::str::from_utf8(data).ok()?;

    // Check each pattern category
    if text.contains("-----BEGIN") && text.contains("PRIVATE KEY") {
        return Some(SensitiveType::PrivateKey);
    }

    if text.contains("AKIA") || text.contains("aws_secret") {
        return Some(SensitiveType::AwsCredentials);
    }

    if text.contains("ghp_") || text.contains("gho_") || text.contains("github_pat_") {
        return Some(SensitiveType::GitHubToken);
    }

    if text.contains("sk-") && text.len() > 40 {
        return Some(SensitiveType::ApiKey);
    }

    if text.contains("eyJ") && text.matches('.').count() == 2 {
        return Some(SensitiveType::JwtToken);
    }

    if SENSITIVE_PATTERNS[11].is_match(text) {
        return Some(SensitiveType::DatabaseConnection);
    }

    if SENSITIVE_PATTERNS[13].is_match(text) {
        return Some(SensitiveType::CreditCard);
    }

    if SENSITIVE_PATTERNS[14].is_match(text) {
        return Some(SensitiveType::Ssn);
    }

    // Check generic password pattern last
    if SENSITIVE_PATTERNS[8].is_match(text) || SENSITIVE_PATTERNS[9].is_match(text) {
        return Some(SensitiveType::Password);
    }

    None
}

#[derive(Debug, Clone, PartialEq)]
pub enum SensitiveType {
    Password,
    ApiKey,
    PrivateKey,
    AwsCredentials,
    GitHubToken,
    JwtToken,
    DatabaseConnection,
    CreditCard,
    Ssn,
    Other,
}

impl std::fmt::Display for SensitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensitiveType::Password => write!(f, "Password"),
            SensitiveType::ApiKey => write!(f, "API Key"),
            SensitiveType::PrivateKey => write!(f, "Private Key"),
            SensitiveType::AwsCredentials => write!(f, "AWS Credentials"),
            SensitiveType::GitHubToken => write!(f, "GitHub Token"),
            SensitiveType::JwtToken => write!(f, "JWT Token"),
            SensitiveType::DatabaseConnection => write!(f, "Database Connection"),
            SensitiveType::CreditCard => write!(f, "Credit Card"),
            SensitiveType::Ssn => write!(f, "SSN"),
            SensitiveType::Other => write!(f, "Sensitive Data"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_api_key() {
        let data = b"api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz";
        assert!(is_sensitive(data, &ContentType::Text));
    }

    #[test]
    fn test_detect_private_key() {
        let data = b"-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----";
        assert!(is_sensitive(data, &ContentType::Text));
    }

    #[test]
    fn test_detect_jwt() {
        let data = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert!(is_sensitive(data, &ContentType::Text));
    }

    #[test]
    fn test_normal_text_not_sensitive() {
        let data = b"Hello, this is a normal message with no secrets.";
        assert!(!is_sensitive(data, &ContentType::Text));
    }

    #[test]
    fn test_classify_github_token() {
        let data = b"ghp_abcdefghij1234567890abcdefghij1234";
        assert_eq!(classify_sensitive(data), Some(SensitiveType::GitHubToken));
    }
}
