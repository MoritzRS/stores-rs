use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{Callback, Emitter, Readable};

/// A readable observable value that is derived from other observables.
pub struct Derived<Value>
where
    Value: Clone + Send + Sync,
{
    value: RwLock<Value>,
    compute: Box<dyn Fn() -> Value + Send + Sync>,
    callbacks: RwLock<HashMap<usize, Callback<Value>>>,
    counter: RwLock<usize>,
}

impl<Value> Derived<Value>
where
    Value: Clone + Send + Sync + 'static,
{
    /// Creates a new derived value.
    ///
    /// The result is wrapped inside an Arc to be easily transferable.
    ///
    /// # Example
    ///
    /// ```
    /// use stores::{Observable, Derived, Readable};
    /// let a = Observable::new(1);
    /// let b = Observable::new(2);
    /// let sum = Derived::new(&[a.clone(), b.clone()], {
    ///     let a = a.clone();
    ///     let b = b.clone();
    ///     move || a.get() + b.get()
    /// });
    /// ```
    pub fn new(
        targets: &[Arc<impl Emitter + Send + Sync + 'static>],
        compute: impl Fn() -> Value + Send + Sync + 'static,
    ) -> Arc<Self> {
        let value = compute();

        let instance = Arc::new(Self {
            value: RwLock::new(value),
            compute: Box::new(compute),
            callbacks: RwLock::new(HashMap::new()),
            counter: RwLock::new(0),
        });

        for target in targets {
            let _unsubscribe = target.listen({
                let instance = instance.clone();
                move || {
                    let new_value = (instance.compute)();
                    *instance.value.write().unwrap() = new_value.clone();

                    instance.notify();
                }
            });
        }

        instance
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

impl<Value> Emitter for Derived<Value>
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

impl<Value> Readable<Value> for Derived<Value>
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

impl<Value> Debug for Derived<Value>
where
    Value: Debug + Clone + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Derived")
            .field("value", &self.value.read().unwrap())
            .field("callbacks", &self.callbacks.read().unwrap().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, thread};

    use crate::{Observable, Readable, Writable};

    use super::*;

    #[test]
    fn it_derives() {
        let observable = Observable::new(0);
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        assert_eq!(derived.get(), 0);

        observable.set(5);
        assert_eq!(derived.get(), 10);
    }

    #[test]
    fn it_derives_many() {
        let observable_a = Observable::new(0);
        let observable_b = Observable::new(0);
        let derived = Derived::new(&[observable_a.clone(), observable_b.clone()], {
            let observable_a = observable_a.clone();
            let observable_b = observable_b.clone();
            move || observable_a.get() + observable_b.get()
        });

        assert_eq!(derived.get(), 0);

        observable_a.set(5);
        assert_eq!(derived.get(), 5);

        observable_b.set(10);
        assert_eq!(derived.get(), 15);
    }

    #[test]
    fn it_triggers_emitter_on_change() {
        let observable = Observable::new(0);
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));
        let _ = derived.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 0);

        observable.set(1);
        assert_eq!(counter.lock().unwrap().clone(), 1);

        observable.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_unsubscribes_from_emitter() {
        let observable = Observable::new(0);
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));
        let unsubscribe = derived.listen({
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
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));
        let _ = derived.subscribe({
            let counter = counter.clone();
            move |value| {
                *counter.lock().unwrap() = *value;
            }
        });

        assert_eq!(derived.get(), 0);
        assert_eq!(counter.lock().unwrap().clone(), 0);

        observable.set(1);
        assert_eq!(derived.get(), 2);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_triggers_subscription_directly() {
        let observable = Observable::new(0);
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));
        let _ = derived.subscribe({
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
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));
        let unsubscribe = derived.subscribe({
            let counter = counter.clone();
            move |_| {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        observable.set(1);
        assert_eq!(counter.lock().unwrap().clone(), 2);

        unsubscribe();
        observable.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_works_in_threads() {
        let observable = Observable::new(0);
        let derived = Derived::new(&[observable.clone()], {
            let observable = observable.clone();
            move || observable.get() * 2
        });

        let counter = Arc::new(Mutex::new(0));

        let _ = derived.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        (0..10)
            .map(|_| {
                let observable = observable.clone();
                thread::spawn(move || {
                    observable.update(|value| value + 1);
                })
            })
            .for_each(|thread| thread.join().unwrap());

        assert_eq!(derived.get(), 20);
        assert_eq!(counter.lock().unwrap().clone(), 10);
    }
}
