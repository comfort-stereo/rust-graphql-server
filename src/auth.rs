use std::{
    fmt::{Display, Formatter, Result as FormatResult},
    ops::Deref,
};

use hmac::{Hmac, NewMac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn encode(data: SessionTokenData, secret: &SessionTokenSecret) -> Self {
        SessionToken(data.sign_with_key(secret).unwrap())
    }

    pub fn decode(token: &str, secret: &SessionTokenSecret) -> Option<SessionTokenData> {
        token.verify_with_key(secret).ok()
    }

    pub fn verify(token: &str, secret: &SessionTokenSecret) -> Option<SessionToken> {
        Self::decode(token, secret).map(|data| Self::encode(data, secret))
    }

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

pub type SessionTokenSecret = Hmac<Sha256>;

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionTokenData {
    pub session_id: Uuid,
    pub session_token_id: Uuid,
    pub user_id: Uuid,
}
