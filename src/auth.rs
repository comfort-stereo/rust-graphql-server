use std::fmt::{Display, Formatter, Result as FormatResult};
use std::ops::Deref;

use hmac::{Hmac, NewMac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

/// Represents an encoded JWT session token.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionToken(String);

impl SessionToken {
    /// Encode session token data as a session token using a specified secret.
    pub fn encode(data: SessionTokenData, secret: &SessionTokenSecret) -> Self {
        SessionToken(data.sign_with_key(secret).unwrap())
    }

    /// Attempt to decode a session token using a specified secret. This will return the session
    /// token's data if the token is validated and decoded successfully and none otherwise.
    pub fn decode(token: &str, secret: &SessionTokenSecret) -> Option<SessionTokenData> {
        token.verify_with_key(secret).ok()
    }

    /// Verify a possible session token. This will return the verified session token if the token is
    /// validated successfully and none otherwise.
    pub fn verify(token: &str, secret: &SessionTokenSecret) -> Option<SessionToken> {
        Self::decode(token, secret).map(|data| Self::encode(data, secret))
    }

    /// Convert a string into a session token secret.
    pub fn secret(string: &str) -> SessionTokenSecret {
        SessionTokenSecret::new_varkey(string.as_bytes()).unwrap()
    }
}

impl Display for SessionToken {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(formatter, "{}", self.0)
    }
}

impl Deref for SessionToken {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// Secret used to encode/decode session tokens.
pub type SessionTokenSecret = Hmac<Sha256>;

/// Data stored in a session token.
#[derive(Clone, Serialize, Deserialize)]
pub struct SessionTokenData {
    /// The ID of the session this token is associated with. Used as a key in the Redis database.
    pub session_id: Uuid,
    /// Unique ID of this particular session token. This is used to differentiate session tokens
    /// associated with the same session. Only one active session token is allowed per session.
    /// Refreshing a session token will return a new session token with a different token ID.
    pub session_token_id: Uuid,
    /// The ID of the user this session token is associated with.
    pub user_id: Uuid,
}
