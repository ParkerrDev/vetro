//! The boundary: the only place in the workspace where `unsafe` is written.
//!
//! The C++ plugin lends memory for the length of a call, as a pointer and a
//! number of items. Turning that pair into a slice is the one thing safe Rust
//! cannot do, so it is done here, twice (to read, and to write), and nowhere
//! else. Single objects cross as `Option<&T>` and `Option<&mut T>`, which C
//! sees as a nullable pointer and Rust checks without any `unsafe`.
//!
//! The contract of every pointer and length the plugin hands over, which is
//! what makes the blocks below sound: the pointer is either null with a length
//! of zero, or it points to that many initialized, properly aligned items that
//! nothing else touches until the call returns, and for a pointer to write
//! through, that no other pointer of the same call aliases.

#![expect(
    unsafe_code,
    reason = "this module is the boundary: raw pointers from C++ become slices here and nowhere else"
)]

use crate::status::Failure;

/// The `len` items at `ptr`, or an empty slice for a length of zero.
///
/// # Errors
///
/// [`Failure::Null`] for a null or misaligned pointer with items behind it,
/// [`Failure::Argument`] for a length no allocation could have.
pub(crate) fn read_slice<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], Failure> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() || !ptr.is_aligned() {
        return Err(Failure::Null);
    }
    if len
        .checked_mul(size_of::<T>())
        .is_none_or(|bytes| isize::try_from(bytes).is_err())
    {
        return Err(Failure::Argument);
    }
    // SAFETY: `ptr` is non-null and aligned (checked above), and the length cannot
    // overflow an allocation (checked above). By the contract of the C API the
    // caller owns `len` initialized items of `T` at `ptr` that stay valid and
    // unmodified until the call that received them returns, and the slice does
    // not outlive that call: every caller in this crate drops it before returning.
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// The `len` items at `ptr`, to write, or an empty slice for a length of zero.
///
/// # Errors
///
/// As [`read_slice`].
pub(crate) fn write_slice<'a, T>(ptr: *mut T, len: usize) -> Result<&'a mut [T], Failure> {
    if len == 0 {
        return Ok(&mut []);
    }
    if ptr.is_null() || !ptr.is_aligned() {
        return Err(Failure::Null);
    }
    if len
        .checked_mul(size_of::<T>())
        .is_none_or(|bytes| isize::try_from(bytes).is_err())
    {
        return Err(Failure::Argument);
    }
    // SAFETY: `ptr` is non-null and aligned and the length fits an allocation
    // (checked above). By the contract of the C API the caller owns `len` items of
    // `T` at `ptr`, writable, with no other pointer of the same call into them, for
    // as long as the call lasts; the slice is dropped before the call returns.
    Ok(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_of_zero_is_an_empty_slice_whatever_the_pointer() {
        assert_eq!(read_slice::<u8>(std::ptr::null(), 0).unwrap().len(), 0);
        assert_eq!(
            write_slice::<u32>(std::ptr::null_mut(), 0).unwrap().len(),
            0
        );
    }

    #[test]
    fn a_null_pointer_with_items_behind_it_is_refused() {
        assert_eq!(read_slice::<u8>(std::ptr::null(), 4), Err(Failure::Null));
        assert_eq!(
            write_slice::<f64>(std::ptr::null_mut(), 1),
            Err(Failure::Null)
        );
    }

    #[test]
    fn a_misaligned_pointer_is_refused() {
        let words = [0_u32; 4];
        // one byte into an array of words: an address no u32 may sit at
        let address = words.as_ptr().addr().wrapping_add(1);
        let misaligned = std::ptr::with_exposed_provenance::<u32>(address);
        assert_eq!(read_slice(misaligned, 1), Err(Failure::Null));
    }

    #[test]
    fn a_length_no_allocation_could_have_is_refused() {
        let byte = 0_u8;
        assert_eq!(
            read_slice(&raw const byte, usize::MAX),
            Err(Failure::Argument)
        );
        let mut word = 0_u64;
        assert_eq!(
            write_slice(&raw mut word, usize::MAX / 4),
            Err(Failure::Argument)
        );
    }

    #[test]
    fn what_is_lent_is_read_and_written_through() {
        let mut items = [1_u16, 2, 3];
        let read = read_slice(items.as_ptr(), 3).unwrap();
        assert_eq!(read, &[1, 2, 3]);
        write_slice(items.as_mut_ptr(), 2)
            .unwrap()
            .copy_from_slice(&[7, 8]);
        assert_eq!(items, [7, 8, 3]);
    }
}
