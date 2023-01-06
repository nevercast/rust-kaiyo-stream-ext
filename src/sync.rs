pub type MessageProducer = tokio::sync::broadcast::Sender<crate::messages::SelectionMessage>;
pub type MessageConsumer = tokio::sync::broadcast::Receiver<crate::messages::SelectionMessage>;

#[derive(Clone)]
pub struct MessageConsumerFactory
{
    tx: MessageProducer,
}

impl MessageConsumerFactory
{
    pub fn new(tx: &MessageProducer) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn create(&self) -> MessageConsumer {
        self.tx.subscribe()
    }
}
