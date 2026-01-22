--NOTE: INT is INT8 in cockroachdb

-- Subscription plans (cache of Stripe products)
CREATE TABLE IF NOT EXISTS plans (
    id VARCHAR(50) PRIMARY KEY,           -- 'free', 'pro', 'business'
    stripe_price_id VARCHAR(100),         -- Stripe price ID (null for free)
    monitor_limit INT NOT NULL,           -- Max monitors allowed (-1 = unlimited)
    notification_limit INT NOT NULL,      -- Monthly notification limit
    overage_price_cents INT DEFAULT 0     -- Cents per overage notification
);

-- User subscriptions
CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_id VARCHAR(50) NOT NULL REFERENCES plans(id),
    stripe_customer_id VARCHAR(100),
    stripe_subscription_id VARCHAR(100),
    status VARCHAR(50) DEFAULT 'active',  -- active, canceled, past_due
    current_period_start TIMESTAMP,
    current_period_end TIMESTAMP,
    created_at TIMESTAMP DEFAULT current_timestamp(),
    UNIQUE(user_id)
);

-- Monthly usage tracking
CREATE TABLE IF NOT EXISTS usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start DATE NOT NULL,           -- First day of billing period
    monitors_count INT DEFAULT 0,
    notifications_sent INT DEFAULT 0,
    overage_notifications INT DEFAULT 0,
    UNIQUE(user_id, period_start)
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_usage_user_period ON usage(user_id, period_start);

-- Seed default plans
INSERT INTO plans (id, stripe_price_id, monitor_limit, notification_limit, overage_price_cents) VALUES
    ('free', NULL, 2, 100, 0),
    ('pro', NULL, 10, 5000, 20),
    ('business', NULL, -1, 50000, 10)
ON CONFLICT (id) DO NOTHING;
