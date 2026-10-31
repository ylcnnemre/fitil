//! A reconnecting WebSocket loop shared by all exchange adapters.
//!
//! Connecting, reconnecting with growing delays, noticing a silent connection and
//! subscribing again are the same everywhere. An adapter only describes its own
//! protocol through [`StreamProtocol`].

use std::collections::BTreeSet;
use std::time::Duration;

use futures::channel::mpsc;
use futures::{Sink, SinkExt, StreamExt};
use tokio::time::Instant;
pub use tokio_tungstenite::tungstenite::Message;

use crate::{FeedCommand, FeedEvent, Topic};

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

/// The parts of a WebSocket API that differ between exchanges.
pub trait StreamProtocol: Send + 'static {
    /// Where to connect.
    fn url(&self) -> String;

    /// The exchange's name for the stream of `topic`.
    fn stream_name(&self, topic: &Topic) -> String;

    /// Messages that subscribe to `streams`, or unsubscribe from them when `subscribe`
    /// is false.
    fn request(&mut self, subscribe: bool, streams: &[String]) -> Vec<Message>;

    /// Turns a message from the exchange into events, plus a reply to send back if the
    /// exchange expects one (such as an answer to its ping).
    fn read(&mut self, message: &Message) -> Incoming;

    /// A message the exchange wants at a fixed interval to keep the connection open.
    fn keepalive(&self) -> Option<(Duration, Message)> {
        None
    }

    /// With no message for this long, the connection counts as dead.
    fn idle_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

/// What one incoming message produced.
#[derive(Debug, Default)]
pub struct Incoming {
    pub events: Vec<FeedEvent>,
    pub reply: Option<Message>,
}

impl Incoming {
    pub fn event(event: FeedEvent) -> Self {
        Self {
            events: vec![event],
            reply: None,
        }
    }
}

/// Streams live data into `events` until either channel closes.
///
/// Connects once there is at least one topic. When the connection drops, it reconnects
/// with growing delays and subscribes to every topic again.
pub async fn run_stream<P: StreamProtocol>(
    mut protocol: P,
    mut commands: mpsc::UnboundedReceiver<FeedCommand>,
    mut events: mpsc::Sender<FeedEvent>,
) {
    crate::install_crypto_provider();
    let mut streams = BTreeSet::new();
    let mut attempt = 0;

    loop {
        while streams.is_empty() {
            let Some(command) = commands.next().await else {
                return;
            };
            apply(&protocol, &mut streams, &command);
        }

        if let Ok((socket, _)) = tokio_tungstenite::connect_async(protocol.url()).await {
            let (mut write, mut read) = socket.split();
            let all: Vec<String> = streams.iter().cloned().collect();
            if send_all(&mut write, protocol.request(true, &all))
                .await
                .is_ok()
            {
                attempt = 0;
                if events.send(FeedEvent::Connected).await.is_err() {
                    return;
                }
                let idle = protocol.idle_timeout();
                let (mut ticker, ping) = match protocol.keepalive() {
                    Some((every, message)) => (
                        Some(tokio::time::interval_at(Instant::now() + every, every)),
                        Some(message),
                    ),
                    None => (None, None),
                };
                loop {
                    let tick = async {
                        match &mut ticker {
                            Some(ticker) => {
                                ticker.tick().await;
                            }
                            None => std::future::pending().await,
                        }
                    };
                    tokio::select! {
                        message = tokio::time::timeout(idle, read.next()) => {
                            let Ok(Some(Ok(message))) = message else {
                                break;
                            };
                            if matches!(message, Message::Close(_)) {
                                break;
                            }
                            let incoming = protocol.read(&message);
                            if let Some(reply) = incoming.reply
                                && write.send(reply).await.is_err()
                            {
                                break;
                            }
                            for event in incoming.events {
                                if events.send(event).await.is_err() {
                                    return;
                                }
                            }
                        }
                        command = commands.next() => {
                            let Some(command) = command else {
                                return;
                            };
                            if let Some((subscribe, stream)) = apply(&protocol, &mut streams, &command)
                                && send_all(&mut write, protocol.request(subscribe, &[stream])).await.is_err()
                            {
                                break;
                            }
                        }
                        () = tick => {
                            if let Some(ping) = &ping
                                && write.send(ping.clone()).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            }
        }

        attempt += 1;
        let delay = reconnect_delay(attempt);
        if events
            .send(FeedEvent::Reconnecting { attempt, delay })
            .await
            .is_err()
        {
            return;
        }
        tokio::time::sleep(delay).await;
    }
}

/// Updates the subscribed stream names. Returns whether to subscribe (or unsubscribe)
/// and to which stream, if anything changed.
fn apply<P: StreamProtocol>(
    protocol: &P,
    streams: &mut BTreeSet<String>,
    command: &FeedCommand,
) -> Option<(bool, String)> {
    match command {
        FeedCommand::Subscribe(topic) => {
            let name = protocol.stream_name(topic);
            streams.insert(name.clone()).then_some((true, name))
        }
        FeedCommand::Unsubscribe(topic) => {
            let name = protocol.stream_name(topic);
            streams.remove(&name).then_some((false, name))
        }
    }
}

async fn send_all<S>(write: &mut S, messages: Vec<Message>) -> Result<(), S::Error>
where
    S: Sink<Message> + Unpin,
{
    for message in messages {
        write.send(message).await?;
    }
    Ok(())
}

/// 1, 2, 4, 8, 16, then 30 seconds.
fn reconnect_delay(attempt: u32) -> Duration {
    let seconds = 1u64 << attempt.saturating_sub(1).min(5);
    Duration::from_secs(seconds).min(MAX_RECONNECT_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// A tiny protocol: "sub a,b" to subscribe, "ping" → "pong", anything else is a tick.
    struct TestProtocol {
        url: String,
    }

    impl StreamProtocol for TestProtocol {
        fn url(&self) -> String {
            self.url.clone()
        }

        fn stream_name(&self, topic: &Topic) -> String {
            match topic {
                Topic::AllTickers => "all".into(),
                Topic::Candles { symbol, .. } => symbol.clone(),
            }
        }

        fn request(&mut self, subscribe: bool, streams: &[String]) -> Vec<Message> {
            let verb = if subscribe { "sub" } else { "unsub" };
            vec![Message::text(format!("{verb} {}", streams.join(",")))]
        }

        fn read(&mut self, message: &Message) -> Incoming {
            match message {
                Message::Text(text) if text.as_str() == "ping" => Incoming {
                    events: Vec::new(),
                    reply: Some(Message::text("pong")),
                },
                Message::Text(_) => Incoming::event(FeedEvent::Tickers(Vec::new())),
                _ => Incoming::default(),
            }
        }
    }

    #[tokio::test]
    async fn answers_pings_reconnects_and_subscribes_again() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());

        // The server takes two connections and drops the first one after a tick.
        let server = tokio::spawn(async move {
            let mut received = Vec::new();
            for round in 0..2 {
                let (tcp, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
                let Some(Ok(Message::Text(subscription))) = socket.next().await else {
                    panic!("expected a subscription");
                };
                received.push(subscription.to_string());
                if round == 0 {
                    socket.send(Message::text("ping")).await.unwrap();
                    let Some(Ok(Message::Text(reply))) = socket.next().await else {
                        panic!("expected a pong");
                    };
                    received.push(reply.to_string());
                    socket.send(Message::text("tick")).await.unwrap();
                    socket.close(None).await.unwrap();
                }
            }
            received
        });

        let (commands, command_receiver) = mpsc::unbounded();
        let (event_sender, mut events) = mpsc::channel(16);
        let feed = tokio::spawn(run_stream(
            TestProtocol { url },
            command_receiver,
            event_sender,
        ));
        commands
            .unbounded_send(FeedCommand::Subscribe(Topic::AllTickers))
            .unwrap();

        assert!(matches!(events.next().await, Some(FeedEvent::Connected)));
        assert!(matches!(events.next().await, Some(FeedEvent::Tickers(_))));
        assert!(matches!(
            events.next().await,
            Some(FeedEvent::Reconnecting { attempt: 1, .. })
        ));
        assert!(matches!(events.next().await, Some(FeedEvent::Connected)));
        assert_eq!(server.await.unwrap(), ["sub all", "pong", "sub all"]);

        drop(events);
        drop(commands);
        tokio::time::timeout(Duration::from_secs(5), feed)
            .await
            .expect("the feed stops once its channels close")
            .unwrap();
    }

    #[test]
    fn only_changes_produce_requests() {
        let protocol = TestProtocol { url: String::new() };
        let mut streams = BTreeSet::new();
        let subscribe = FeedCommand::Subscribe(Topic::AllTickers);
        assert_eq!(
            apply(&protocol, &mut streams, &subscribe),
            Some((true, "all".to_owned()))
        );
        assert_eq!(apply(&protocol, &mut streams, &subscribe), None);
        let unsubscribe = FeedCommand::Unsubscribe(Topic::AllTickers);
        assert_eq!(
            apply(&protocol, &mut streams, &unsubscribe),
            Some((false, "all".to_owned()))
        );
        assert_eq!(apply(&protocol, &mut streams, &unsubscribe), None);
    }

    #[test]
    fn reconnect_delays_grow_then_cap() {
        let delays: Vec<u64> = (1..=8).map(|n| reconnect_delay(n).as_secs()).collect();
        assert_eq!(delays, [1, 2, 4, 8, 16, 30, 30, 30]);
    }
}
