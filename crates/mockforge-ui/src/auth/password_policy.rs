//! Password policy enforcement
//!
//! This module provides password validation and complexity requirements
//! for user account creation and password changes.

use std::collections::HashSet;

/// Password policy configuration
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    /// Minimum password length
    pub min_length: usize,
    /// Maximum password length
    pub max_length: usize,
    /// Require uppercase letters
    pub require_uppercase: bool,
    /// Require lowercase letters
    pub require_lowercase: bool,
    /// Require numbers
    pub require_numbers: bool,
    /// Require special characters
    pub require_special: bool,
    /// Forbidden passwords (common passwords, username, etc.)
    pub forbidden_passwords: HashSet<String>,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: false, // Optional for better UX
            forbidden_passwords: Self::common_passwords(),
        }
    }
}

impl PasswordPolicy {
    /// Create a strict password policy
    pub fn strict() -> Self {
        Self {
            min_length: 12,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
            forbidden_passwords: Self::common_passwords(),
        }
    }

    /// Create a relaxed password policy (for development)
    pub fn relaxed() -> Self {
        Self {
            min_length: 6,
            max_length: 128,
            require_uppercase: false,
            require_lowercase: true,
            require_numbers: false,
            require_special: false,
            forbidden_passwords: HashSet::new(),
        }
    }

    /// Common passwords to forbid
    fn common_passwords() -> HashSet<String> {
        [
            "password", "123456", "12345678", "1234", "qwerty",
            "abc123", "monkey", "1234567", "letmein", "trustno1",
            "dragon", "baseball", "iloveyou", "master", "sunshine",
            "ashley", "bailey", "passw0rd", "shadow", "123123",
            "654321", "superman", "qazwsx", "michael", "football",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Validate a password against the policy
    pub fn validate(&self, password: &str, username: Option<&str>) -> Result<(), PasswordValidationError> {
        // Check length
        if password.len() < self.min_length {
            return Err(PasswordValidationError::TooShort(self.min_length));
        }
        if password.len() > self.max_length {
            return Err(PasswordValidationError::TooLong(self.max_length));
        }

        // Check character requirements
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_number = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| {
            "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)
        });

        if self.require_uppercase && !has_uppercase {
            return Err(PasswordValidationError::MissingUppercase);
        }
        if self.require_lowercase && !has_lowercase {
            return Err(PasswordValidationError::MissingLowercase);
        }
        if self.require_numbers && !has_number {
            return Err(PasswordValidationError::MissingNumber);
        }
        if self.require_special && !has_special {
            return Err(PasswordValidationError::MissingSpecial);
        }

        // Check forbidden passwords
        let password_lower = password.to_lowercase();
        if self.forbidden_passwords.contains(&password_lower) {
            return Err(PasswordValidationError::CommonPassword);
        }

        // Check if password contains username
        if let Some(username) = username {
            if password_lower.contains(&username.to_lowercase()) {
                return Err(PasswordValidationError::ContainsUsername);
            }
        }

        Ok(())
    }
}

/// Password validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordValidationError {
    TooShort(usize),
    TooLong(usize),
    MissingUppercase,
    MissingLowercase,
    MissingNumber,
    MissingSpecial,
    CommonPassword,
    ContainsUsername,
}

impl std::fmt::Display for PasswordValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordValidationError::TooShort(min) => {
                write!(f, "Password must be at least {} characters long", min)
            }
            PasswordValidationError::TooLong(max) => {
                write!(f, "Password must be at most {} characters long", max)
            }
            PasswordValidationError::MissingUppercase => {
                write!(f, "Password must contain at least one uppercase letter")
            }
            PasswordValidationError::MissingLowercase => {
                write!(f, "Password must contain at least one lowercase letter")
            }
            PasswordValidationError::MissingNumber => {
                write!(f, "Password must contain at least one number")
            }
            PasswordValidationError::MissingSpecial => {
                write!(f, "Password must contain at least one special character")
            }
            PasswordValidationError::CommonPassword => {
                write!(f, "Password is too common. Please choose a more unique password")
            }
            PasswordValidationError::ContainsUsername => {
                write!(f, "Password cannot contain your username")
            }
        }
    }
}

impl std::error::Error for PasswordValidationError {}
