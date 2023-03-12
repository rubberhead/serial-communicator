/// Checks whether `small` is a sub-slice of `large` in O(n) time. 
pub fn subslice_of<T>(small: &[T], large: &[T]) -> bool 
where T: PartialEq {
    let window_size = small.len(); 
    assert!(window_size <= large.len(), "[util::subslice_of] `small` longer than `large`");
    for i in 0..=large.len() - window_size {
        let window = &large[i..(i + window_size)]; 
        if window == small { return true; }
    }
    false
}