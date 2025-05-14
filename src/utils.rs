/// Simplifies the creation of derived values
///
/// # Example
///
/// ```
/// use stores::{Observable, Readable, derive};
/// let a = Observable::new(1);
/// let b = Observable::new(2);
/// let sum = derive!([a, b] => move || a.get() + b.get());
/// ```
#[macro_export]
macro_rules! derive {
    ([$($target:ident),*] => $func:expr) => {

        $crate::Derived::new(
            &[$($target.clone()),*],
            {
                $( let $target = $target.clone(); )*
                $func
            }
        )

    };
}

/// Simplifies cloning for callbacks.
///
/// # Example
///
/// ```
/// use stores::{Observable, Readable, clone};
/// let a = Observable::new(1);
/// let b = Observable::new(2);
/// let c = Observable::new(3);
///
/// let _ = c.subscribe(clone!([a,b] => move |c| {
///     println!("{}, {}, {}", a.get(), b.get(), c);
/// }));
/// ```
#[macro_export]
macro_rules! clone {
    ( [$($target:ident),*] => $func:expr) => {{
        $( let $target = $target.clone(); )*
        $func
    }};
}

#[cfg(test)]
mod tests {

    use crate::{Observable, Readable, Writable};

    #[test]
    fn it_derives() {
        let observable = Observable::new(1);
        let doubled = derive!([observable] => move || observable.get() * 2);

        assert_eq!(doubled.get(), 2);
    }

    #[test]
    fn it_clones() {
        let a = Observable::new(1);
        let b = Observable::new(2);

        let sum = Observable::new(0);

        let _ = a.subscribe(clone!([b, sum] => move |a| {
            sum.set(b.get() + a);
        }));

        let _ = b.subscribe(clone!([a, sum] => move |b| {
            sum.set(a.get() + b);
        }));

        a.set(2);
        b.set(5);

        assert_eq!(sum.get(), 7);
    }
}
