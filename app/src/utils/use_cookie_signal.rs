use leptos::*;
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use codee::{Decoder, Encoder};


pub struct UseCookieSignalResult<T, FStore>
where 
    T: Clone + 'static,
    FStore: Fn() + Clone + 'static,
{
    pub signal_reader: ReadSignal<T>,
    pub signal_writer: WriteSignal<T>,
    pub store_value: FStore,
}


pub fn use_cookie_signal<T, C>(
    value: T,
    cookie_name: &str,
    coolie_options: UseCookieOptions<T, <C as Encoder<T>>::Error, <C as Decoder<T>>::Error>,
) -> UseCookieSignalResult<
    T,
    impl Fn() + Clone + 'static,
> 
where
    T: Clone,
    C: Encoder<T, Encoded = String> + Decoder<T, Encoded = str>,
{
    let (value, set_value) = create_signal::<T>(value);
    let (
        value_cookie, set_value_cookie
    ) = use_cookie_with_options::<T, C>(cookie_name, coolie_options);    

    if value_cookie.get_untracked().is_none() {
        set_value_cookie.set(Some(value.get()));
    }
    else {
        set_value.set(value_cookie.get_untracked().unwrap());
    }

    UseCookieSignalResult {
        signal_reader: value,
        signal_writer: set_value,
        store_value: move || {
            set_value_cookie.set(Some(value.get()));
        },
    }
}
