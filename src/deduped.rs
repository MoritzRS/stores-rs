use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{Callback, Emitter, Observable, Readable, Writable};

/// A deduplicated observable value.
///
/// Wraps around an observable and only triggers callbacks when the new value is different from the
/// current value.
/// If the wrapped value implements Writable, all changes will be propagated to the target.
pub struct Deduped<Value, Target>
where
    Value: PartialEq + Eq + Clone + Send + Sync,
    Target: Readable<Value> + Emitter + Send + Sync,
{
    target: Arc<Target>,
    value: RwLock<Value>,
    callbacks: RwLock<HashMap<usize, Callback<Value>>>,
    counter: RwLock<usize>,
}

impl<Value, Target> Deduped<Value, Target>
where
    Value: PartialEq + Eq + Clone + Send + Sync + 'static,
    Target: Readable<Value> + Emitter + Send + Sync + 'static,
{
    /// Creates a new deduplicated value by wrapping another observable.
    ///
    /// # Example
    ///
    /// ```
    /// use stores::{Observable, Deduped};
    /// let observable = Observable::new(1);
    /// let deduped = Deduped::from(observable.clone());
    /// ```
    pub fn from(target: Arc<Target>) -> Arc<Self> {
        let instance = Arc::new(Self {
            target: target.clone(),
            value: RwLock::new(target.get()),
            callbacks: RwLock::new(HashMap::new()),
            counter: RwLock::new(0),
        });

        let _ = target.subscribe({
            let instance = instance.clone();
            move |value| {
                if *instance.value.read().unwrap() != *value {
                    *instance.value.write().unwrap() = value.clone();
                    instance.notify();
                }
            }
        });

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

impl<Value> Deduped<Value, Observable<Value>>
where
    Value: PartialEq + Eq + Clone + Send + Sync + 'static,
{
    /// Creates a standalone Deduped.
    ///
    /// Creates an internal Observable that it wraps.
    ///
    /// # Example
    ///
    /// ```
    /// use stores::Deduped;
    /// let deduped = Deduped::new(1);
    /// ```
    pub fn new(value: Value) -> Arc<Self> {
        let target = Observable::new(value);
        Self::from(target)
    }
}

impl<Value, Target> Emitter for Deduped<Value, Target>
where
    Value: PartialEq + Eq + Clone + Send + Sync,
    Target: Readable<Value> + Emitter + Send + Sync,
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

impl<Value, Target> Readable<Value> for Deduped<Value, Target>
where
    Value: PartialEq + Eq + Clone + Send + Sync + 'static,
    Target: Readable<Value> + Emitter + Send + Sync + 'static,
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

impl<Value, Target> Writable<Value> for Deduped<Value, Target>
where
    Value: PartialEq + Eq + Clone + Send + Sync,
    Target: Readable<Value> + Emitter + Writable<Value> + Send + Sync,
{
    fn set(&self, value: Value) {
        self.target.set(value);
    }

    fn update(&self, updater: impl Fn(&Value) -> Value + Send + Sync + 'static) {
        self.target.update(updater);
    }
}

impl<Value, Target> Debug for Deduped<Value, Target>
where
    Value: Debug + PartialEq + Eq + Clone + Send + Sync,
    Target: Readable<Value> + Emitter + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deduped")
            .field("value", &self.value.read().unwrap())
            .field("callbacks", &self.callbacks.read().unwrap().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Mutex;

    use super::*;

    #[test]
    fn it_provdes_getter() {
        let deduped = Deduped::new(1);
        assert_eq!(deduped.get(), 1);
    }

    #[test]
    fn it_provides_setters_for_writle() {
        let deduped = Deduped::new(1);

        deduped.set(2);
        assert_eq!(deduped.get(), 2);

        deduped.update(|value| value + 1);
        assert_eq!(deduped.get(), 3);
    }

    #[test]
    fn it_uses_setters_from_target() {
        let target = Observable::new(1);
        let deduped = Deduped::from(target.clone());

        deduped.set(2);
        assert_eq!(target.get(), 2);
        assert_eq!(deduped.get(), 2);

        deduped.update(|value| value + 1);
        assert_eq!(target.get(), 3);
        assert_eq!(deduped.get(), 3);
    }

    #[test]
    fn it_triggers_emitter_only_on_change() {
        let deduped = Deduped::new(1);
        let counter = Arc::new(Mutex::new(0));

        let _ = deduped.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 0);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 1);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 1);
    }

    #[test]
    fn it_unsubscribes_from_emitter() {
        let deduped = Deduped::new(1);
        let counter = Arc::new(Mutex::new(0));

        let unsubscribe = deduped.listen({
            let counter = counter.clone();
            move || {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 0);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 1);

        unsubscribe();
        deduped.set(3);
        assert_eq!(counter.lock().unwrap().clone(), 1);
    }

    #[test]
    fn it_provides_value_to_subscription() {
        let deduped = Deduped::new(1);
        let counter = Arc::new(Mutex::new(0));

        let _ = deduped.subscribe({
            let counter = counter.clone();
            move |value| {
                *counter.lock().unwrap() = *value;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);

        deduped.set(3);
        assert_eq!(counter.lock().unwrap().clone(), 3);
    }

    #[test]
    fn it_triggers_subscription_directly_and_only_on_change() {
        let deduped = Deduped::new(1);
        let counter = Arc::new(Mutex::new(0));

        let _ = deduped.subscribe({
            let counter = counter.clone();
            move |_| {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_unsubscribes_from_subscription() {
        let deduped = Deduped::new(1);
        let counter = Arc::new(Mutex::new(0));

        let unsubscribe = deduped.subscribe({
            let counter = counter.clone();
            move |_| {
                *counter.lock().unwrap() += 1;
            }
        });

        assert_eq!(counter.lock().unwrap().clone(), 1);

        deduped.set(2);
        assert_eq!(counter.lock().unwrap().clone(), 2);

        unsubscribe();
        deduped.set(3);
        assert_eq!(counter.lock().unwrap().clone(), 2);
    }

    #[test]
    fn it_works_in_threads() {
        let deduped = Deduped::new(0);
        let counter = Arc::new(Mutex::new(0));

        let _ = deduped.subscribe({
            let counter = counter.clone();
            move |value| {
                *counter.lock().unwrap() = *value;
            }
        });

        (0..10)
            .map(|_| {
                let deduped = deduped.clone();
                std::thread::spawn(move || {
                    deduped.update(|value| value + 1);
                })
            })
            .for_each(|thread| thread.join().unwrap());

        assert_eq!(deduped.get(), 10);
        assert_eq!(counter.lock().unwrap().clone(), 10);
    }
}
