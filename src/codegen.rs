//! Support for the codegen module.
#![doc(hidden)]

use std::mem::{size_of, zeroed};
use std::fmt::*;

/// Implementors correspond to formatting traits which may apply to values.
pub trait FormatTrait {
    /// Return whether this format trait is applicable to a type.
    #[inline]
    fn allowed<T>() -> bool;
    /// Format a value of the given trait using this format trait.
    /// Must panic if `allowed::<T>()` is false.
    #[inline]
    fn perform<T>(t: &T, f: &mut Formatter) -> Result;
}

// Abuse specialization to provide the `FormatTrait` impl for the actual
// format traits without requiring HKT or other deep chicanery.
trait Specialized<T> {
    #[inline]
    fn allowed() -> bool;
    #[inline]
    fn perform(t: &T, f: &mut Formatter) -> Result;
}

macro_rules! impl_format_trait {
    ($($name:ident,)*) => {
        $(
            impl<T> Specialized<T> for $name {
                #[inline]
                default fn allowed() -> bool { false }
                #[inline]
                default fn perform(_: &T, _: &mut Formatter) -> Result {
                    panic!()
                }
            }

            impl<T: $name> Specialized<T> for $name {
                #[inline]
                fn allowed() -> bool { true }
                #[inline]
                fn perform(t: &T, f: &mut Formatter) -> Result {
                    t.fmt(f)
                }
            }

            impl FormatTrait for $name {
                #[inline]
                fn allowed<T>() -> bool { <Self as Specialized<T>>::allowed() }
                #[inline]
                fn perform<T>(t: &T, f: &mut Formatter) -> Result {
                    <Self as Specialized<T>>::perform(t, f)
                }
            }
        )*
    }
}

impl_format_trait! {
    Display, Debug, LowerExp, UpperExp, Octal, Pointer, Binary, LowerHex,
    UpperHex,
}

// Local type alias for the formatting function pointer type.
type FormatFn<T> = fn(&T, &mut Formatter) -> Result;

/// Attempt to convert a function from `&This` to `&Value` into a function that formats
/// an `&Value` with the given format type `Format`.
/// Returns `Some` only when `Value` implements `Format`
#[inline]
pub fn get_formatter<Format, This, Value, Mapper>(_: Mapper) -> Option<FormatFn<This>>
    where Format: FormatTrait + ?Sized, Mapper: Fn(&This) -> &Value {

    assert!(size_of::<Mapper>() == 0,
            "Mapper from parent to child must be zero-sized, instead size was {}",
            size_of::<Mapper>());

    if Format::allowed::<Value>() {
        fn inner<Format, This, Value, Mapper>(this: &This, fmt: &mut Formatter) -> Result
            where Format: FormatTrait + ?Sized, Mapper: Fn(&This) -> &Value {

            let mapper = unsafe { zeroed::<Mapper>() };
            Format::perform::<Value>(mapper(this), fmt)
        }
        Some(inner::<Format, This, Value, Mapper>)
    }
    else {
        None
    }
}

// Specialization abuse to select only functions which return `&usize`.
trait SpecUsize {
    #[inline]
    fn convert<T>(f: fn(&T) -> &Self) -> Option<fn(&T) -> &usize>;
}

impl<U> SpecUsize for U {
    #[inline]
    default fn convert<T>(_: fn(&T) -> &Self) -> Option<fn(&T) -> &usize> { None }
}

impl SpecUsize for usize {
    #[inline]
    fn convert<T>(f: fn(&T) -> &usize) -> Option<fn(&T) -> &usize> { Some(f) }
}

/// Attempt to convert a function from `&This` to `&VAlue` to a function from `&This`
/// to `&usize`. Returns `Some` only when `B` is `usize`.
#[inline]
pub fn get_as_usize<This, Value, Mapper>(_: Mapper) -> Option<fn(&This) -> &usize>
    where Mapper: Fn(&This) -> &Value {

    assert!(size_of::<Mapper>() == 0,
            "Mapper from parent to child must be zero-sized, instead size was {}",
            size_of::<Mapper>());

    fn inner<This, Value, Mapper>(this: &This) -> &Value
        where Mapper: Fn(&This) -> &Value {

        let mapper = unsafe { zeroed::<Mapper>() };
        mapper(this)
    }
    <Value as SpecUsize>::convert(inner::<This, Value, Mapper>)
}

/// A trait for types against which formatting specifiers may be pre-checked.
///
/// Implementations may be generated automatically using `runtime-fmt-derive`
/// and `#[derive(FormatArgs)]`.
pub trait FormatArgs {
    /// Find the index within this type corresponding to the provided name.
    ///
    /// If this function returns `Some`, `get_child` with the returned index
    /// must not panic.
    fn validate_name(name: &str) -> Option<usize>;

    /// Validate that a given index is within range for this type.
    ///
    /// If this function returns `true`, `get_child` with the given index must
    /// not panic.
    fn validate_index(index: usize) -> bool;

    /// Return the formatter function for the given format trait, accepting
    /// `&Self` and using the given format trait on the value at that index.
    ///
    /// Returns `None` if the given format trait cannot format the child at
    /// that index. Panics if the index is invalid.
    fn get_child<F: FormatTrait + ?Sized>(index: usize) -> Option<FormatFn<Self>>;

    /// Return the value at the given index interpreted as a `usize`.
    ///
    /// Returns `None` if the child at the given index cannot be interpreted
    /// as a `usize`. Panics if the index is invalid.
    fn as_usize(index: usize) -> Option<fn(&Self) -> &usize>;
}
