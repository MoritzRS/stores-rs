use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::Emitter;

/// A simple observable that holds no value.
pub struct Event {
    callbacks: RwLock<HashMap<usize, Box<dyn Fn() + Send + Sync>>>,
    counter: RwLock<usize>,
}

impl Event {
    /// Creates a new Event.
    ///
    /// The result is wrapped inside an Arc to be easily transferable.
    ///
    /// # Example
    ///
    /// ```
    /// use stores::Event;
    /// let event = Event::new();
    /// ```
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            callbacks: RwLock::new(HashMap::new()),
            counter: RwLock::new(0),
        })
    }

    /// Runs all registered callbacks.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::Event;
    /// # let event = Event::new();
    /// event.dispatch();
    /// ```
    pub fn dispatch(&self) {
        for callback in self.callbacks.read().unwrap().values() {
            callback();
        }
    }
}

impl Emitter for Event {
    fn listen(&self, callback: impl Fn() + Send + Sync + 'static) -> impl Fn() {
        let callback = Box::new(callback);
        let id = *self.counter.read().unwrap();
        *self.counter.write().unwrap() += 1;

        self.callbacks.write().unwrap().insert(id, callback);

        move || {
            self.callbacks.write().unwrap().remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn it_dispatches() {
        let event = Event::new();
        let counter = Arc::new(Mutex::new(0));

        let _ = event.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(*counter.lock().unwrap(), 0);

        event.dispatch();
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn it_unsubscribes() {
        let event = Event::new();
        let counter = Arc::new(Mutex::new(0));

        let unsubscribe = event.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(*counter.lock().unwrap(), 0);

        event.dispatch();
        assert_eq!(*counter.lock().unwrap(), 1);

        unsubscribe();
        event.dispatch();
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn it_works_in_threads() {
        let event = Event::new();
        let counter = Arc::new(Mutex::new(0));

        let _ = event.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(*counter.lock().unwrap(), 0);

        (0..10)
            .map(|_| {
                thread::spawn({
                    let event = event.clone();
                    move || {
                        event.dispatch();
                    }
                })
            })
            .for_each(|thread| {
                thread.join().unwrap();
            });

        assert_eq!(*counter.lock().unwrap(), 10);
    }
}
