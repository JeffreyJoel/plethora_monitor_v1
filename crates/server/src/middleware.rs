use clerk_rs::clerk::Clerk;
use clerk_rs::validators::axum::ClerkLayer;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;

pub fn auth_layer(clerk: Clerk) -> ClerkLayer<MemoryCacheJwksProvider> {
    ClerkLayer::new(MemoryCacheJwksProvider::new(clerk), None, true)
}
