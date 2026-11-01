//! Work outside of the window's thread: the capture must not wait for the window, and the window
//! must not wait for a filter over half a million packets.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use eframe::egui::Context;

/// Data of a page, loaded in the background. A new key loads again at once, and `every` loads
/// again after that time; meanwhile the last value stays in view, so nothing flickers.
pub struct Loader<K, T> {
    key: Option<K>,
    value: Option<T>,
    /// The key the value belongs to
    value_key: Option<K>,
    started: Option<Instant>,
    busy: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<(K, T)>>>,
}

impl<K, T> Default for Loader<K, T> {
    fn default() -> Self {
        Self {
            key: None,
            value: None,
            value_key: None,
            started: None,
            busy: Arc::new(AtomicBool::new(false)),
            slot: Arc::new(Mutex::new(None)),
        }
    }
}

impl<K: Clone + PartialEq + Send + 'static, T: Send + 'static> Loader<K, T> {
    /// The newest value, whatever key it belongs to
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// The value, if it belongs to `key`
    pub fn value_of(&self, key: &K) -> Option<&T> {
        self.value
            .as_ref()
            .filter(|_| self.value_key.as_ref() == Some(key))
    }

    /// Hands the value over; the next `load` still knows its key
    pub fn take(&mut self) -> Option<T> {
        self.value.take()
    }

    /// How long the job has been running, zero without one
    pub fn running_for(&self) -> Duration {
        match self.started {
            Some(started) if self.busy.load(Ordering::Acquire) => started.elapsed(),
            _ => Duration::ZERO,
        }
    }

    /// Takes a finished result; `true` if there is a new value
    pub fn poll(&mut self) -> bool {
        let done = self
            .slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        match done {
            // a result for a key that is out of date is dropped, the next `load` asks again
            Some((key, value)) if Some(&key) == self.key.as_ref() => {
                self.value = Some(value);
                self.value_key = Some(key);
                true
            }
            _ => false,
        }
    }

    /// Starts `job` if the key is new or `every` has passed, unless a job is still running
    pub fn load(
        &mut self,
        ctx: &Context,
        key: &K,
        every: Option<Duration>,
        job: impl FnOnce() -> T + Send + 'static,
    ) {
        if self.busy.load(Ordering::Acquire) {
            self.key_changed(key);
            return;
        }
        let due = self.key.as_ref() != Some(key)
            || self.value_key.as_ref() != Some(key)
            || every.is_some_and(|e| self.started.is_none_or(|s| s.elapsed() >= e));
        if !due {
            return;
        }
        self.key = Some(key.clone());
        self.started = Some(Instant::now());
        self.busy.store(true, Ordering::Release);
        let busy = self.busy.clone();
        let slot = self.slot.clone();
        let ctx = ctx.clone();
        let key = key.clone();
        let spawned = std::thread::Builder::new()
            .name("thread_window_load".to_owned())
            .spawn(move || {
                let value = job();
                *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some((key, value));
                busy.store(false, Ordering::Release);
                ctx.request_repaint();
            });
        if spawned.is_err() {
            self.busy.store(false, Ordering::Release);
        }
    }

    /// While a job runs, a new key only waits: the result of the old key is dropped by `poll`
    fn key_changed(&mut self, key: &K) {
        if self.key.as_ref() != Some(key) {
            self.key = Some(key.clone());
        }
    }

    /// Forgets the value, so the next `load` asks again at once
    pub fn reset(&mut self) {
        self.key = None;
        self.value = None;
        self.value_key = None;
        self.started = None;
    }
}

/// A single job in the background, such as starting the capture or saving a file
pub struct Task<T> {
    busy: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<T>>>,
}

impl<T> Default for Task<T> {
    fn default() -> Self {
        Self {
            busy: Arc::new(AtomicBool::new(false)),
            slot: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T: Send + 'static> Task<T> {
    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    pub fn start(&self, ctx: &Context, job: impl FnOnce() -> T + Send + 'static) {
        if self.busy.swap(true, Ordering::AcqRel) {
            return;
        }
        let busy = self.busy.clone();
        let slot = self.slot.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("thread_window_task".to_owned())
            .spawn(move || {
                let value = job();
                *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
                busy.store(false, Ordering::Release);
                ctx.request_repaint();
            });
        if spawned.is_err() {
            self.busy.store(false, Ordering::Release);
        }
    }

    /// The result of the job, once
    pub fn take(&self) -> Option<T> {
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(until: impl Fn() -> bool) {
        let start = Instant::now();
        while !until() && start.elapsed() < Duration::from_secs(5) {
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_result_of_an_old_key_is_dropped() {
        let ctx = Context::default();
        let mut loader: Loader<u32, u32> = Loader::default();
        let (go, gate) = std::sync::mpsc::channel::<()>();
        loader.load(&ctx, &1, None, move || {
            let _ = gate.recv();
            10
        });
        // the user went on while the first job runs
        loader.load(&ctx, &2, None, || 20);
        let _ = go.send(());
        wait(|| !loader.busy.load(Ordering::Acquire));
        assert!(!loader.poll());
        assert_eq!(loader.value(), None);

        loader.load(&ctx, &2, None, || 20);
        wait(|| !loader.busy.load(Ordering::Acquire));
        assert!(loader.poll());
        assert_eq!(loader.value_of(&2), Some(&20));
        assert_eq!(loader.value_of(&1), None);
    }

    #[test]
    fn the_value_stays_until_the_next_one_is_there() {
        let ctx = Context::default();
        let mut loader: Loader<u32, u32> = Loader::default();
        loader.load(&ctx, &1, None, || 10);
        wait(|| !loader.busy.load(Ordering::Acquire));
        assert!(loader.poll());
        loader.load(&ctx, &2, None, || 20);
        assert_eq!(loader.value(), Some(&10));
        wait(|| !loader.busy.load(Ordering::Acquire));
        assert!(loader.poll());
        assert_eq!(loader.value(), Some(&20));
    }

    #[test]
    fn a_task_gives_its_result_once() {
        let ctx = Context::default();
        let task: Task<u32> = Task::default();
        task.start(&ctx, || 7);
        wait(|| !task.busy());
        assert_eq!(task.take(), Some(7));
        assert_eq!(task.take(), None);
    }
}
