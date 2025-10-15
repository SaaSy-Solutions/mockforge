use rdkafka::admin::AdminClient; fn main() { let _ = AdminClient::<rdkafka::client::DefaultClientContext>::alter_consumer_group_offsets; }
