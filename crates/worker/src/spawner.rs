use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

use js_sys::Array;
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::bridge::{Bridge, CallbackMap};
use crate::handler_id::HandlerId;
use crate::messages::{FromWorker, Packed};
use crate::native_worker::{DedicatedWorker, NativeWorkerExt};
use crate::traits::Worker;
use crate::Shared;

fn create_worker(path: &str) -> DedicatedWorker {
    let wasm_url = path.replace(".js", "_bg.wasm");
    let array = Array::new();
    array.push(&format!(r#"importScripts("{}");wasm_bindgen("{}");"#, path, wasm_url).into());
    let blob = Blob::new_with_str_sequence_and_options(
        &array,
        BlobPropertyBag::new().type_("application/javascript"),
    )
    .unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();

    DedicatedWorker::new(&url).expect("failed to spawn worker")
}

/// A spawner to create workers.
#[derive(Clone)]
pub struct Spawner<W>
where
    W: Worker,
{
    _marker: PhantomData<W>,
    callback: Option<Rc<dyn Fn(W::Output)>>,
}

impl<W: Worker> fmt::Debug for Spawner<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkerScope<_>")
    }
}

impl<W> Default for Spawner<W>
where
    W: Worker,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<W> Spawner<W>
where
    W: Worker,
{
    /// Creates a [Spawner].
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            callback: None,
        }
    }

    /// Sets a callback.
    pub fn callback<F>(&mut self, cb: F) -> &mut Self
    where
        F: 'static + Fn(W::Output),
    {
        self.callback = Some(Rc::new(cb));

        self
    }

    /// Spawns a Worker.
    pub fn spawn(&self, path: &str) -> Bridge<W> {
        let pending_queue = Rc::new(RefCell::new(Some(Vec::new())));

        let handler_id = HandlerId::new();

        let mut callbacks = HashMap::new();

        if let Some(m) = self.callback.as_ref().map(Rc::downgrade) {
            callbacks.insert(handler_id, m);
        }

        let callbacks: Shared<CallbackMap<W>> = Rc::new(RefCell::new(callbacks));

        let handler = {
            let pending_queue = pending_queue.clone();
            let callbacks = callbacks.clone();

            move |data: Vec<u8>, worker: &web_sys::Worker| {
                let msg = FromWorker::<W>::unpack(&data);
                match msg {
                    FromWorker::WorkerLoaded => {
                        if let Some(pending_queue) = pending_queue.borrow_mut().take() {
                            for to_worker in pending_queue.into_iter() {
                                worker.post_packed_message(to_worker);
                            }
                        }
                    }
                    FromWorker::ProcessOutput(id, output) => {
                        let mut callbacks = callbacks.borrow_mut();

                        if let Some(m) = callbacks.get(&id) {
                            if let Some(m) = Weak::upgrade(m) {
                                m(output);
                            } else {
                                callbacks.remove(&id);
                            }
                        }
                    }
                }
            }
        };

        let handler_cell = Rc::new(RefCell::new(Some(handler)));

        let worker = {
            let handler_cell = handler_cell.clone();
            let worker = create_worker(path);
            let worker_clone = worker.clone();
            worker.set_on_packed_message(move |data: Vec<u8>| {
                if let Some(handler) = handler_cell.borrow().as_ref() {
                    handler(data, &worker_clone)
                }
            });
            worker
        };

        Bridge::<W>::new(
            handler_id,
            worker,
            pending_queue,
            callbacks,
            self.callback.clone(),
        )
    }
}
