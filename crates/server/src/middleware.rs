//! # Middleware Module
//!
//! Provides authentication middleware for protecting API routes.
//!
//! ## Authentication Flow
//!
//! Uses Clerk's JWT validation middleware to:
//!
//! 1. Extract JWT token from `Authorization` header
//! 2. Validate token signature using Clerk's JWKS (JSON Web Key Set)
//! 3. Cache JWKS in memory for performance
//! 4. Extract user claims and make them available to handlers
//!
//! ## Usage
//!
//! The middleware is applied to all `/api/*` routes via the router.
//! Handlers can access the validated JWT via `Extension<ClerkJwt>`.
//!
//! ## Security
//!
//! - JWKS are cached in memory to reduce network requests
//! - Tokens are validated on every request
//! - Invalid or expired tokens result in 401 Unauthorized responses

use clerk_rs::clerk::Clerk;
use clerk_rs::validators::axum::ClerkLayer;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;

pub fn auth_layer(clerk: Clerk) -> ClerkLayer<MemoryCacheJwksProvider> {
    ClerkLayer::new(MemoryCacheJwksProvider::new(clerk), None, true)
}
