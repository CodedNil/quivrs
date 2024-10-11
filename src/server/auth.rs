use crate::common::TOKEN_LENGTH;
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use arrayvec::ArrayString;
use chrono::{DateTime, Utc};
use rand::{distributions, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt,
};
use uuid::Uuid;

const MIN_PASSWORD_LENGTH: usize = 6;
const AUTH_FILE: &str = "auth.ron";

#[derive(Serialize, Deserialize)]
struct Account {
    admin: bool,
    uuid: Uuid,
    username: String,
    password_hash: String,
    tokens: Vec<Token>,
}

#[derive(Serialize, Deserialize)]
struct Token {
    token: ArrayString<TOKEN_LENGTH>,
    last_used: DateTime<Utc>,
}

type Accounts = HashMap<Uuid, Account>;

async fn read_accounts() -> Result<Accounts> {
    if fs::metadata(AUTH_FILE).await.is_err() {
        return Ok(HashMap::new());
    }

    let mut file = OpenOptions::new().read(true).open(AUTH_FILE).await?;
    let mut data = String::new();
    file.read_to_string(&mut data).await?;
    let accounts: Accounts = ron::from_str(&data)?;
    Ok(accounts)
}

async fn write_accounts(accounts: &Accounts) -> Result<()> {
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let data = ron::ser::to_string_pretty(accounts, pretty)?;
    fs::write(AUTH_FILE, data).await?;
    Ok(())
}

/// Login to account, returning a token
/// If no password is set, it will set the password
/// If no accounts exist, it will create an admin account
pub async fn login(
    username: String,
    password: String,
) -> Result<(String, ArrayString<TOKEN_LENGTH>)> {
    let mut accounts = read_accounts().await.unwrap_or_default();

    // Create initial admin account if no accounts exist
    if accounts.is_empty() {
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(anyhow!(
                "Password must be at least {} characters long",
                MIN_PASSWORD_LENGTH
            ));
        }

        // Hash the password
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|_| anyhow!("Failed to hash password"))?
            .to_string();

        // Create a new admin account
        let (token_entry, token) = generate_token();
        let new_account = Account {
            admin: true,
            uuid: Uuid::new_v4(),
            username: username.clone(),
            password_hash,
            tokens: vec![token_entry],
        };

        // Serialize and save the admin account to the database
        accounts.insert(new_account.uuid, new_account);
        write_accounts(&accounts).await?;

        return Ok(("Admin Account Created".to_string(), token));
    }

    // Retrieve account data using username as the key
    let account = accounts.values_mut().find(|acc| acc.username == username);
    if let Some(account) = account {
        if account.password_hash.is_empty() {
            // This is a new account setup case
            if password.len() < MIN_PASSWORD_LENGTH {
                return Err(anyhow!("Password must be at least 6 characters long"));
            }

            // Hash the new password
            let password_hash = Argon2::default()
                .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
                .map_err(|_| anyhow!("Failed to hash password"))?
                .to_string();

            // Update the account with the new password and add a token
            let (token_entry, token) = generate_token();
            account.tokens.push(token_entry);
            account.password_hash = password_hash;

            write_accounts(&accounts).await?;

            return Ok(("Admit Set".to_string(), token));
        }

        // Verify password for an existing account
        let parsed_hash = PasswordHash::new(&account.password_hash)
            .map_err(|_| anyhow!("Incorrect username or password"))?;

        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            let (token_entry, token) = generate_token();
            account.tokens.push(token_entry);
            write_accounts(&accounts).await?;
            return Ok((String::new(), token));
        }
    }
    Err(anyhow!("Incorrect username or password"))
}

/// Helper function to generate a random token
fn generate_token() -> (Token, ArrayString<TOKEN_LENGTH>) {
    let mut new_token = ArrayString::<TOKEN_LENGTH>::new();
    let mut rng = thread_rng();
    for _ in 0..TOKEN_LENGTH {
        new_token.push(rng.sample(distributions::Alphanumeric) as char);
    }

    (
        Token {
            token: new_token,
            last_used: Utc::now(),
        },
        new_token,
    )
}

/// Verify tokens, updating the `last_used`
pub async fn verify_token(input_token: ArrayString<TOKEN_LENGTH>) -> Result<bool> {
    let mut accounts = read_accounts().await?;

    for account in accounts.values_mut() {
        if let Some(token_entry) = account
            .tokens
            .iter_mut()
            .find(|token| token.token == input_token)
        {
            token_entry.last_used = Utc::now();
            write_accounts(&accounts).await?;
            return Ok(true);
        }
    }

    Ok(false)
}
