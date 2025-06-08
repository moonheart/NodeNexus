-- 1. 创建 notification_channels 表
-- 该表用于存储用户配置的各种通知渠道，例如 Telegram, Webhook 等。
CREATE TABLE public.notification_channels (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    channel_type VARCHAR(50) NOT NULL, -- 例如: 'telegram', 'webhook'
    -- 使用 BYTEA 类型存储加密后的配置信息 (JSON格式)
    config BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE public.notification_channels IS '存储用户配置的推送渠道';
COMMENT ON COLUMN public.notification_channels.config IS '加密后的渠道配置信息，以JSON格式存储';

-- 为 user_id 创建索引以加速查询
CREATE INDEX idx_notification_channels_user_id ON public.notification_channels(user_id);

-- 2. 创建 alert_rule_channels 表
-- 这是一个连接表，用于建立警报规则 (alert_rules) 和通知渠道 (notification_channels) 之间的多对多关系。
CREATE TABLE public.alert_rule_channels (
    alert_rule_id INTEGER NOT NULL REFERENCES public.alert_rules(id) ON DELETE CASCADE,
    channel_id INTEGER NOT NULL REFERENCES public.notification_channels(id) ON DELETE CASCADE,
    PRIMARY KEY (alert_rule_id, channel_id)
);

COMMENT ON TABLE public.alert_rule_channels IS '警报规则与通知渠道的多对多关系表';

-- 为连接表的外键创建索引
CREATE INDEX idx_alert_rule_channels_alert_rule_id ON public.alert_rule_channels(alert_rule_id);
CREATE INDEX idx_alert_rule_channels_channel_id ON public.alert_rule_channels(channel_id);

-- 3. 修改现有的 alert_rules 表
-- 移除旧的、功能单一的 notification_channel 字段，因为它已被新的多对多关系取代。
-- 注意: 此操作会删除现有警报规则中的通知渠道设置，这是预期行为。
ALTER TABLE public.alert_rules DROP COLUMN notification_channel;