use alloc::collections::LinkedList;

use thiserror_no_std::Error;

#[derive(Error, Debug)]
#[error("array is full")]
pub struct ArrayFullError;

/// This list type allows for use with no memory allocation.
///
/// After heap allocation is present, we can switch to using a dynamic
/// linked list type.
pub enum StaticList<T, const N: usize> {
    Array([Option<T>; N]),
    LinkedList(LinkedList<T>),
}
impl<T: Copy, const N: usize> StaticList<T, N> {
    pub const fn new() -> StaticList<T, N> {
        StaticList::Array([None; N])
    }

    pub fn push_back(&mut self, value: T) -> Result<(), ArrayFullError> {
        match self {
            StaticList::Array(arr) => {
                for item in arr {
                    if item.is_none() {
                        *item = Some(value);
                        return Ok(());
                    }
                }
                Err(ArrayFullError)
            }
            StaticList::LinkedList(list) => {
                list.push_back(value);
                Ok(())
            }
        }
    }

    pub fn push_front(&mut self, value: T) -> Result<(), ArrayFullError> {
        match self {
            StaticList::Array(arr) => {
                if let Some(None) = arr.last() {
                    arr.rotate_right(1);
                    arr[0].replace(value);
                    Ok(())
                } else {
                    Err(ArrayFullError)
                }
            }
            StaticList::LinkedList(list) => {
                list.push_front(value);
                Ok(())
            }
        }
    }

    pub fn convert_to_heap_allocated(&mut self) {
        let new_list = match self {
            StaticList::Array(arr) => {
                let mut list = LinkedList::new();
                for item in arr {
                    if let Some(item) = item.clone().take() {
                        list.push_back(item);
                    }
                }
                list
            }
            StaticList::LinkedList(_) => return,
        };
        *self = StaticList::LinkedList(new_list);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let mut iter_arr = None;
        let mut iter_list = None;
        match self {
            StaticList::Array(arr) => iter_arr = Some(arr.iter().flatten()),
            StaticList::LinkedList(list) => iter_list = Some(list.iter()),
        }
        iter_arr
            .into_iter()
            .flatten()
            .chain(iter_list.into_iter().flatten())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let mut iter_arr = None;
        let mut iter_list = None;
        match self {
            StaticList::Array(arr) => iter_arr = Some(arr.iter_mut().flatten()),
            StaticList::LinkedList(list) => iter_list = Some(list.iter_mut()),
        }
        iter_arr
            .into_iter()
            .flatten()
            .chain(iter_list.into_iter().flatten())
    }
}
