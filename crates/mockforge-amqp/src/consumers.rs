/// Consumer for AMQP queues
#[derive(Debug)]
pub struct Consumer {
    pub tag: String,
    pub queue: String,
    pub no_ack: bool,
    pub prefetch_count: u16,
}

impl Consumer {
    pub fn new(tag: String, queue: String, no_ack: bool, prefetch_count: u16) -> Self {
        Self {
            tag,
            queue,
            no_ack,
            prefetch_count,
        }
    }
}
