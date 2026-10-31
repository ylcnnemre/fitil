//! Exchange live feeds, run as iced subscriptions.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use fitil_core::Market;
use fitil_exchange::Exchange;
use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};

use crate::Message;

/// The live feed of one market. Identified by the market alone, so iced keeps the same
/// feed running across updates.
pub struct Source {
    pub market: Market,
    pub exchange: Arc<dyn Exchange>,
}

impl Hash for Source {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.market.hash(state);
    }
}

/// Runs the feed, turning its events into messages. The first message hands over the
/// channel used to subscribe and unsubscribe.
pub fn run(source: &Source) -> impl Stream<Item = Message> + use<> {
    let market = source.market;
    let exchange = Arc::clone(&source.exchange);
    iced::stream::channel(100, async move |mut output: mpsc::Sender<Message>| {
        let (commands, command_receiver) = mpsc::unbounded();
        if output
            .send(Message::FeedReady(market, commands))
            .await
            .is_err()
        {
            return;
        }
        let (event_sender, mut events) = mpsc::channel(100);
        let forward = async move {
            while let Some(event) = events.next().await {
                if output.send(Message::Feed(market, event)).await.is_err() {
                    break;
                }
            }
        };
        futures::join!(
            exchange.feed(market.kind, command_receiver, event_sender),
            forward
        );
    })
}
