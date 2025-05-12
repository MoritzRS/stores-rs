use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{Callback, Emitter, Readable, Writable};

/// A readable and writable observable value.
pub struct Observable<Value>
where
    Value: Clone + Send + Sync,
{
    value: RwLock<Value>,
    callbacks: RwLock<HashMap<usize, Callback<Value>>>,
    counter: RwLock<usize>,
}

impl<Value> Observable<Value>
where
    Value: Clone + Send + Sync,
{
    /// Creates a new observable value.
    ///
    /// The result is wrapped inside an Arc to be easily transferable.
    ///
    /// # Example
    ///
    /// ```
    /// use stores::Observable;
    /// let observable = Observable::new(1);
    /// ```
    pub fn new(value: Value) -> Arc<Self> {
        Arc::new(Self {
            value: RwLock::new(value),
            callbacks: RwLock::new(HashMap::new()),
            counter: RwLock::new(0),
        })
    }

    /// Internal function to run all registered callbacks.
    fn notify(&self) {
        let value = self.value.read().unwrap().clone();
        for callback in self.callbacks.read().unwrap().values() {
            match callback {
                Callback::Subscriber(func) => func(&value),
                Callback::Listener(func) => func(),
            }
        }
    }
}

impl<Value> Emitter for Observable<Value>
where
    Value: Clone + Send + Sync,
{
    fn listen(&self, callback: impl Fn() + Send + Sync + 'static) -> impl Fn() {
        let callback = Box::new(callback);
        let id = *self.counter.read().unwrap();
        *self.counter.write().unwrap() += 1;

        self.callbacks
            .write()
            .unwrap()
            .insert(id, Callback::Listener(callback));
        move || {
            self.callbacks.write().unwrap().remove(&id);
        }
    }
}

impl<Value> Readable<Value> for Observable<Value>
where
    Value: Clone + Send + Sync,
{
    fn get(&self) -> Value {
        self.value.read().unwrap().clone()
    }

    fn subscribe(&self, callback: impl Fn(&Value) + Send + Sync + 'static) -> impl Fn() {
        let value = self.value.read().unwrap().clone();
        callback(&value);

        let callback = Box::new(callback);
        let id = *self.counter.read().unwrap();
        *self.counter.write().unwrap() += 1;

        self.callbacks
            .write()
            .unwrap()
            .insert(id, Callback::Subscriber(callback));

        move || {
            self.callbacks.write().unwrap().remove(&id);
        }
    }
}

impl<Value> Writable<Value> for Observable<Value>
where
    Value: Clone + Send + Sync,
{
    fn set(&self, value: Value) {
        *self.value.write().unwrap() = value.clone();
        self.notify();
    }

    fn update(&self, updater: impl Fn(&Value) -> Value + Send + Sync + 'static) {
        let value = updater(&self.value.read().unwrap());
        self.set(value);
    }
}

impl<Value> Debug for Observable<Value>
where
    Value: Debug + Clone + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observable")
            .field("value", &self.value.read().unwrap())
            .field("callbacks", &self.callbacks.read().unwrap().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, thread};

    use super::*;

    #[test]
    fn it_provides_getter() {
        let observable = Observable::new(0);
        assert_eq!(observable.get(), 0);
    }

    #[test]
    fn it_provides_setters() {
        let observable = Observable::new(0);

        observable.set(1);
        assert_eq!(observable.get(), 1);

        observable.update(|value| value + 1);
        assert_eq!(observable.get(), 2);
    }

    #[test]
    fn it_triggers_emitter_on_change() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let _ = observable.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 0);

        observable.set(1);
        assert_eq!(observable.get(), 1);
    }

    #[test]
    fn it_unsubscribes_from_emitter() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let unsubscribe = observable.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 0);

        observable.set(1);
        assert_eq!(counter.lock().unwrap().clone(), 1);

        unsubscribe();
        observable.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 1);
    }

    #[test]
    fn it_provides_value_to_subscription() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let _ = observable.subscribe({
            let counter = counter.clone();
            move |value| {
                *counter.lock().unwrap() = *value;
            }
        });

        assert_eq!(observable.get(), 0);
        assert_eq!(counter.lock().unwrap().clone(), 0);

        observable.set(1);
        assert_eq!(observable.get(), 1);
        assert_eq!(counter.lock().unwrap().clone(), 1);
    }

    #[test]
    fn it_triggers_subscription_directly() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let _ = observable.subscribe({
            let counter = counter.clone();
            move |_| {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        observable.set(1);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_unsubscribes_from_subscription() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let unsubscribe = observable.subscribe({
            let counter = counter.clone();
            move |_| {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        unsubscribe();
        observable.set(1);
        assert_eq!(counter.lock().unwrap().clone(), 1);
    }

    #[test]
    fn it_works_in_threads() {
        let observable = Observable::new(0);
        let counter = Arc::new(Mutex::new(0));

        let _ = observable.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        (0..10)
            .map(|_| {
                let observable = observable.clone();
                thread::spawn({
                    let observable = observable.clone();
                    move || {
                        observable.update(|value| value + 1);
                    }
                })
            })
            .for_each(|thread| thread.join().unwrap());

        assert_eq!(observable.get(), 10);
        assert_eq!(counter.lock().unwrap().clone(), 10);
    }
}
