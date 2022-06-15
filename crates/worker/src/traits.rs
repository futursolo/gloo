use serde::{Deserialize, Serialize};

use crate::handler_id::HandlerId;
use crate::scope::WorkerScope;
use crate::spawner::WorkerSpawner;

/// Declares the behaviour of a worker.
pub trait Worker: Sized + 'static {
    /// Update message type.
    type Message;
    /// Incoming message type.
    type Input: Serialize + for<'de> Deserialize<'de>;
    /// Outgoing message type.
    type Output: Serialize + for<'de> Deserialize<'de>;

    /// Creates an instance of an worker.
    fn create(scope: &WorkerScope<Self>) -> Self;

    /// Receives an update.
    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message);

    /// This method called on when a new bridge created.
    fn connected(&mut self, _scope: &WorkerScope<Self>, _id: HandlerId) {}

    /// Receives an input.
    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId);

    /// This method called on when a new bridge destroyed.
    fn disconnected(&mut self, _scope: &WorkerScope<Self>, _id: HandlerId) {}

    /// This method called when the worker is destroyed.
    ///
    /// Returns a boolean indicating whether a worker is going to close itself afterwards.
    /// When the value is `true`, it means that it can be closed immediately.
    /// When the value is `false`, the worker itself is responsible to close it with
    /// [`WorkerScope::close`].
    ///
    /// # Note
    ///
    /// The worker will not receive any updates / messages after the destroy method regardless of
    /// whether it has declared to close itself at a later time.
    fn destroy(&mut self, _scope: &WorkerScope<Self>) -> bool {
        true
    }
}

/// A Worker that can be spawned by a spawner.
pub trait Spawnable: Worker {
    /// Creates a spawner.
    fn spawner() -> WorkerSpawner<Self>;
}

impl<T> Spawnable for T
where
    T: Worker,
{
    fn spawner() -> WorkerSpawner<Self> {
        WorkerSpawner::new()
    }
}
