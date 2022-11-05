use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::sink::Sink;
use futures::stream::{FusedStream, Stream};
use pinned::mpsc;
use pinned::mpsc::{UnboundedReceiver, UnboundedSender};
use thiserror::Error;

use super::messages::{ReactorInput, ReactorOutput};
use super::traits::Reactor;
use super::worker::ReactorWorker;
use crate::actor::WorkerBridge;
use crate::{Codec, WorkerSpawner};

/// A connection manager for components interaction with oneshot workers.
///
/// As this type implements [Stream] + [Sink], it can be splitted with [`StreamExt::split`].
pub struct ReactorBridge<R>
where
    R: Reactor + 'static,
{
    inner: WorkerBridge<ReactorWorker<R>>,
    rx: UnboundedReceiver<<R::OutputStream as Stream>::Item>,
}

impl<R> fmt::Debug for ReactorBridge<R>
where
    R: Reactor,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReactorBridge<_>")
    }
}

impl<R> ReactorBridge<R>
where
    R: Reactor + 'static,
{
    #[inline(always)]
    pub(crate) fn new(
        inner: WorkerBridge<ReactorWorker<R>>,
        rx: UnboundedReceiver<<R::OutputStream as Stream>::Item>,
    ) -> Self {
        Self { inner, rx }
    }

    pub(crate) fn output_callback(
        tx: &UnboundedSender<<R::OutputStream as Stream>::Item>,
        output: ReactorOutput<<R::OutputStream as Stream>::Item>,
    ) {
        match output {
            ReactorOutput::Output(m) => {
                let _ = tx.send_now(m);
            }
            ReactorOutput::Finish => {
                tx.close_now();
            }
        }
    }

    #[inline(always)]
    pub(crate) fn register_callback<CODEC>(
        spawner: &mut WorkerSpawner<ReactorWorker<R>, CODEC>,
    ) -> UnboundedReceiver<<R::OutputStream as Stream>::Item>
    where
        CODEC: Codec,
    {
        let (tx, rx) = mpsc::unbounded();
        spawner.callback(move |output| Self::output_callback(&tx, output));

        rx
    }

    /// Forks the bridge.
    ///
    /// This method creates a new bridge connected to a new reactor on the same worker instance.
    pub fn fork(&self) -> Self {
        let (tx, rx) = mpsc::unbounded();
        let inner = self
            .inner
            .fork(Some(move |output| Self::output_callback(&tx, output)));

        Self { inner, rx }
    }

    /// Sends an input to the current reactor.
    pub fn send_input(&self, msg: <R::InputStream as Stream>::Item) {
        self.inner.send(ReactorInput::Input(msg));
    }
}

impl<R> Stream for ReactorBridge<R>
where
    R: Reactor + 'static,
{
    type Item = <R::OutputStream as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rx.size_hint()
    }
}

impl<R> FusedStream for ReactorBridge<R>
where
    R: Reactor + 'static,
{
    fn is_terminated(&self) -> bool {
        self.rx.is_terminated()
    }
}

/// An error type for bridge sink.
#[derive(Error, Clone, PartialEq, Eq, Debug)]
pub enum ReactorBridgeSinkError {
    /// A bridge is an RAII Guard, it can only be closed by dropping the value.
    #[error("attempting to close the bridge via the sink")]
    AttemptClosure,
}

impl<R> Sink<<R::InputStream as Stream>::Item> for ReactorBridge<R>
where
    R: Reactor + 'static,
{
    type Error = ReactorBridgeSinkError;

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(ReactorBridgeSinkError::AttemptClosure))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: <R::InputStream as Stream>::Item,
    ) -> Result<(), Self::Error> {
        self.send_input(item);

        Ok(())
    }
}
