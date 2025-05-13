mod deduped;
mod derived;
mod observable;

pub use deduped::Deduped;
pub use derived::Derived;
pub use observable::Observable;

/// Enum to differentiate between Emitter and Readable subscriptions.
enum Callback<Value>
where
    Value: Clone + Send + Sync,
{
    Subscriber(Box<dyn Fn(&Value) + Send + Sync>),
    Listener(Box<dyn Fn() + Send + Sync>),
}

/// Contract used to subscribe to changes.
pub trait Emitter {
    /// Subscribe to internal changes.
    ///
    /// Registers a callback that is run whenever there are internal changes.
    /// The callback will not be run until the first change.
    /// It returns a function that can be used to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::{Observable, Emitter};
    /// # let observable = Observable::new(0);
    /// let unsubscribe = observable.listen(|| println!("Change detected"));
    /// ```
    fn listen(&self, callback: impl Fn() + Send + Sync + 'static) -> impl Fn();
}

/// Contract for reading and subscribing to values.
pub trait Readable<Value>
where
    Value: Clone + Send + Sync,
{
    /// Read the current value.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::{Observable, Readable};
    /// # let observable = Observable::new(1);
    /// let current = observable.get();
    /// ```
    fn get(&self) -> Value;

    /// Subscribe to any value changes.
    ///
    /// Registers a callback that is run whenever the internal value changes.
    /// The callback will also be run once immediately.
    /// It returns a function that can be used to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::{Observable, Readable};
    /// # let observable = Observable::new(1);
    /// let unsubscribe = observable.subscribe(|value| println!("{}", value));
    /// ```
    fn subscribe(&self, callback: impl Fn(&Value) + Send + Sync + 'static) -> impl Fn();
}

/// Contract for writing and updating values.
pub trait Writable<Value>
where
    Value: Clone + Send + Sync,
{
    /// Sets a new internal value.
    ///
    /// Calling this will trigger all registered callbacks.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::{Observable, Writable};
    /// # let observable = Observable::new(0);
    /// observable.set(123);
    /// ```
    fn set(&self, value: Value);

    /// Updates the internal value based on its current value.
    ///
    /// Calling this will trigger all registered callbacks.
    ///
    /// # Example
    ///
    /// ```
    /// # use stores::{Observable, Writable};
    /// # let observable = Observable::new(0);
    /// observable.update(|value| value * 2);
    /// ```
    fn update(&self, updater: impl Fn(&Value) -> Value + Send + Sync + 'static);
}
