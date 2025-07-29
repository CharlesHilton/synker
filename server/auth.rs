use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use anyhow::{Result, anyhow};
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::types::User;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub username: String,
    pub exp: i64,     // Expiration time
    pub iat: i64,     // Issued at
    pub device_id: Option<String>,
}

pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn hash_password(&self, password: &str) -> Result<String> {
        let hashed = hash(password, DEFAULT_COST)?;
        Ok(hashed)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let is_valid = verify(password, hash)?;
        Ok(is_valid)
    }

    pub fn generate_token(&self, user: &User, device_id: Option<String>) -> Result<String> {
        let now = Utc::now();
        let expiration = now + Duration::hours(24); // Token expires in 24 hours

        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            device_id,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        
        // Check if token is expired
        let now = Utc::now().timestamp();
        if token_data.claims.exp < now {
            return Err(anyhow!("Token has expired"));
        }

        Ok(token_data.claims)
    }

    pub fn extract_user_id(&self, token: &str) -> Result<Uuid> {
        let claims = self.verify_token(token)?;
        let user_id = Uuid::parse_str(&claims.sub)?;
        Ok(user_id)
    }
}

// Middleware for token validation
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    State(auth_service): State<AuthService>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    match auth_service.verify_token(token) {
        Ok(claims) => {
            // Add user info to request extensions
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_password_hashing() {
        let auth_service = AuthService::new("test_secret");
        let password = "test_password";
        
        let hash = auth_service.hash_password(password).unwrap();
        assert!(auth_service.verify_password(password, &hash).unwrap());
        assert!(!auth_service.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_generation_and_verification() {
        let auth_service = AuthService::new("test_secret");
        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: "hash".to_string(),
            created_at: Utc::now(),
            last_login: None,
            is_active: true,
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        let token = auth_service.generate_token(&user, Some("device123".to_string())).unwrap();
        let claims = auth_service.verify_token(&token).unwrap();
        
        assert_eq!(claims.username, user.username);
        assert_eq!(claims.device_id, Some("device123".to_string()));
    }
}
