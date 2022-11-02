use std::fmt;

use futures::stream::Stream;
use serde::de::Deserialize;
use serde::ser::Serialize;

use super::traits::Reactor;
use super::worker::ReactorWorker;
use crate::actor::WorkerRegistrar;
use crate::codec::{Bincode, Codec};
use crate::traits::Registrable;

/// A registrar for reactor workers.
pub struct ReactorRegistrar<R, CODEC = Bincode>
where
    R: Reactor + 'static,
    CODEC: Codec + 'static,
{
    inner: WorkerRegistrar<ReactorWorker<R>, CODEC>,
}

impl<R, CODEC> Default for ReactorRegistrar<R, CODEC>
where
    R: Reactor + 'static,
    CODEC: Codec + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R, CODEC> ReactorRegistrar<R, CODEC>
where
    R: Reactor + 'static,
    CODEC: Codec + 'static,
{
    /// Creates a new reactor registrar.
    pub fn new() -> Self {
        Self {
            inner: ReactorWorker::<R>::registrar().encoding::<CODEC>(),
        }
    }

    /// Sets the encoding.
    pub fn encoding<C>(&self) -> ReactorRegistrar<R, C>
    where
        C: Codec + 'static,
    {
        ReactorRegistrar {
            inner: self.inner.encoding::<C>(),
        }
    }

    /// Registers the worker.
    pub fn register(&self)
    where
        <R::InputStream as Stream>::Item: Serialize + for<'de> Deserialize<'de>,
        <R::OutputStream as Stream>::Item: Serialize + for<'de> Deserialize<'de>,
    {
        self.inner.register()
    }
}

impl<R, CODEC> fmt::Debug for ReactorRegistrar<R, CODEC>
where
    R: Reactor + 'static,
    CODEC: Codec + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactorRegistrar<_>").finish()
    }
}
