#![warn(
    // missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data

/// Todo:
mod model;
mod subscriber;
mod exchange;

pub trait Identifier<T> {
    fn id(&self) -> T;
}