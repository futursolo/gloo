//! A worker that follows the Actor Model.

use std::cell::RefCell;
use std::rc::Rc;

mod bridge;
mod handler_id;
mod lifecycle;
mod messages;
mod native_worker;
mod registrar;
mod scope;
mod spawner;
mod traits;

pub use bridge::WorkerBridge;
pub use handler_id::HandlerId;
pub use registrar::WorkerRegistrar;
pub use scope::{WorkerDestroyHandle, WorkerScope};
pub use spawner::WorkerSpawner;
pub use traits::Registrable;
pub use traits::{Spawnable, Worker};

/// Alias for `Rc<RefCell<T>>`
pub(crate) type Shared<T> = Rc<RefCell<T>>;

/// Alias for `Rc<dyn Fn(IN)>`
pub(crate) type Callback<IN> = Rc<dyn Fn(IN)>;
