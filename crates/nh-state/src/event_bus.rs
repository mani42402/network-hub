//! The daemon-wide domain event bus (§5.4).
//!
//! Everything significant that happens in the daemon becomes an event —
//! `DeviceJoined`, `VpnStateChanged`, `CapabilityChanged`, `ConflictDetected` —
//! and the bus fans it out to the API streams, the automation engine, and the
//! audit log. Two properties are load-bearing:
//!
//! - **Every event carries a monotonically increasing sequence number.** A
//!   client that reconnects gets a fresh snapshot plus the current sequence, then
//!   deltas; a gap in sequence tells it it missed something and should resync.
//! - **Slow subscribers are dropped, never allowed to back-pressure the
//!   producer.** A stalled UI must not be able to slow the engine. This is the
//!   whole reason the bus exists rather than a plain channel: publishing never
//!   blocks, and a subscriber that falls behind is told it lagged and skips
//!   ahead (recovering via a snapshot resync).
//!
//! The bus is generic over the event type so the daemon supplies its own `Event`
//! enum; the sequencing and fan-out live here.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::broadcast;

/// An event tagged with its position in the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequenced<E> {
    /// Monotonically increasing across the whole bus. The first event published
    /// is sequence 1.
    pub seq: u64,
    /// The event payload.
    pub event: E,
}

/// A multi-producer, multi-subscriber event bus with sequencing and
/// drop-for-slow-consumers semantics.
#[derive(Debug)]
pub struct EventBus<E> {
    tx: broadcast::Sender<Sequenced<E>>,
    seq: Arc<AtomicU64>,
}

impl<E: Clone> EventBus<E> {
    /// Creates a bus whose per-subscriber ring holds `capacity` events. A
    /// subscriber that falls more than `capacity` events behind is lagged and
    /// loses the intervening events (by design).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            tx,
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Publishes an event and returns its sequence number.
    ///
    /// Never blocks and never fails from the producer's perspective: if there are
    /// no subscribers, or a subscriber's ring is full, the event is still
    /// sequenced and sent (slow subscribers simply lag). The producer — the
    /// engine — is never held up by a consumer.
    pub fn publish(&self, event: E) -> u64 {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        // A send error only means there are currently no subscribers, which is
        // fine — the sequence still advanced, so a late subscriber resyncs.
        let _ = self.tx.send(Sequenced { seq, event });
        seq
    }

    /// The sequence number of the most recently published event (0 if none).
    /// A newly-connected client records this alongside its snapshot so it can
    /// detect gaps.
    #[must_use]
    pub fn current_seq(&self) -> u64 {
        self.seq.load(Ordering::SeqCst)
    }

    /// Subscribes to future events. The subscription only sees events published
    /// after it is created; the daemon pairs a fresh subscription with a state
    /// snapshot so nothing is missed (§5.4).
    #[must_use]
    pub fn subscribe(&self) -> Subscription<E> {
        Subscription {
            rx: self.tx.subscribe(),
        }
    }

    /// How many subscribers are currently attached.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl<E: Clone> Clone for EventBus<E> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            seq: self.seq.clone(),
        }
    }
}

/// A subscription to the event bus.
#[derive(Debug)]
pub struct Subscription<E> {
    rx: broadcast::Receiver<Sequenced<E>>,
}

/// The outcome of receiving from a subscription.
#[derive(Debug, PartialEq, Eq)]
pub enum Received<E> {
    /// An event, in order.
    Event(Sequenced<E>),
    /// The subscriber fell behind and skipped `missed` events. It should resync
    /// from a fresh snapshot; the next `recv` returns the oldest still-buffered
    /// event.
    Lagged {
        /// How many events were dropped because this subscriber was too slow.
        missed: u64,
    },
    /// The bus was dropped; no more events will arrive.
    Closed,
}

impl<E: Clone> Subscription<E> {
    /// Awaits the next event.
    ///
    /// Returns [`Received::Lagged`] rather than an error when this subscriber was
    /// too slow — the caller treats that as "resync from a snapshot", which is
    /// exactly the drop-for-slow-consumers contract.
    pub async fn recv(&mut self) -> Received<E> {
        match self.rx.recv().await {
            Ok(seq) => Received::Event(seq),
            Err(broadcast::error::RecvError::Lagged(missed)) => Received::Lagged { missed },
            Err(broadcast::error::RecvError::Closed) => Received::Closed,
        }
    }

    /// Tries to receive without waiting. Returns `None` if no event is ready.
    pub fn try_recv(&mut self) -> Option<Received<E>> {
        match self.rx.try_recv() {
            Ok(seq) => Some(Received::Event(seq)),
            Err(broadcast::error::TryRecvError::Lagged(missed)) => {
                Some(Received::Lagged { missed })
            }
            Err(broadcast::error::TryRecvError::Closed) => Some(Received::Closed),
            Err(broadcast::error::TryRecvError::Empty) => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn events_are_sequenced_in_order() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        assert_eq!(bus.publish("a"), 1);
        assert_eq!(bus.publish("b"), 2);
        assert_eq!(bus.current_seq(), 2);

        assert_eq!(
            sub.recv().await,
            Received::Event(Sequenced { seq: 1, event: "a" })
        );
        assert_eq!(
            sub.recv().await,
            Received::Event(Sequenced { seq: 2, event: "b" })
        );
    }

    #[tokio::test]
    async fn publishing_with_no_subscribers_still_advances_sequence() {
        let bus: EventBus<&str> = EventBus::new(16);
        assert_eq!(bus.publish("lonely"), 1);
        assert_eq!(bus.current_seq(), 1);
    }

    #[tokio::test]
    async fn slow_subscriber_lags_without_blocking_producer() {
        // A ring of 4. The producer publishes 10 without the subscriber reading.
        // The producer must NOT block, and the subscriber must be told it lagged
        // rather than the producer stalling — the core §5.4 guarantee.
        let bus = EventBus::new(4);
        let mut slow = bus.subscribe();

        for i in 0..10 {
            // publish returns immediately regardless of the slow subscriber.
            bus.publish(i);
        }
        assert_eq!(bus.current_seq(), 10, "producer advanced through all 10");

        // The slow subscriber is told it lagged, then resumes from the oldest
        // still-buffered event.
        match slow.recv().await {
            Received::Lagged { missed } => assert!(missed >= 6, "missed the overflow"),
            other => panic!("expected Lagged, got {other:?}"),
        }
        // After lagging it can still read the buffered tail.
        match slow.recv().await {
            Received::Event(_) => {}
            other => panic!("expected an event after lag, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multiple_subscribers_each_get_every_event() {
        let bus = EventBus::new(16);
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        bus.publish("x");
        assert_eq!(
            a.recv().await,
            Received::Event(Sequenced { seq: 1, event: "x" })
        );
        assert_eq!(
            b.recv().await,
            Received::Event(Sequenced { seq: 1, event: "x" })
        );
    }
}
