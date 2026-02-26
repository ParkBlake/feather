use may::coroutine::{self, Coroutine};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// VTable for the custom RawWaker.
/// This table defines how to clone, wake, and drop the custom waker which
/// interacts directly with the May coroutine scheduler.
static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |ptr| RawWaker::new(ptr, &VTABLE),
    |ptr| unsafe { (&*(ptr as *const Coroutine)).unpark() },
    |ptr| unsafe { (&*(ptr as *const Coroutine)).unpark() },
    |_| {},
);

/// Blocks the current May coroutine until the given future is resolved.
///
/// This is the core executor for Feather's async support. It allows stackful coroutines
/// to wait on standard Rust `Future`s without blocking the underlying OS thread.
///
/// This must be called from within a May coroutine context.
pub fn block_on<F: Future>(mut fut: F) -> F::Output {
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    let current_co = coroutine::current();

    // Create a waker that points to the current stackful coroutine
    let raw_waker = RawWaker::new(&current_co as *const _ as *const (), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);

    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(res) => return res,
            // If the future is not ready, we park the current coroutine.
            // It will be unparked by the VTABLE unpark function when the waker is called.
            Poll::Pending => coroutine::park(),
        }
    }
}