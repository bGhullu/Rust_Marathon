//! This crate provides a custom iterator for splitting strings by a delimiter.
//!
//! It includes the `StrSplit` struct, which implements the `Iterator` trait.
#![warn(missing_debug_implementations, rust_2018_idioms,missing_docs)]




/// An iterator that splits a string by a delimiter.
#[derive(Debug)]
pub struct StrSplit<'haystack,D>{
    remainder: Option<&'haystack str>,
    // delimiter: &'haystack str,
    delimiter: D
}
// implement struct
impl<'haystack, D> StrSplit<'haystack,D>{
        /// Creates a new `StrSplit` iterator over `haystack`, splitting by `delimiter`.
    pub fn new(haystack: &'haystack str, delimiter: D) -> Self{
        Self { 
            remainder: Some(haystack),
            delimiter,
        }
    }
}

//Delimiter Trait for finding first and last delimeter
pub trait Delimiter {
    fn find_next(&self, s: &str)-> Option<(usize, usize)>;
}

impl<'haystack,D> Iterator for StrSplit<'haystack,D>
where D: Delimiter,
{
    type Item= &'haystack str;

    /// Returns the next substring until the delimiter, or `None` if done.
    fn next(&mut self)-> Option<Self::Item>{

        let  remainder= self.remainder.as_mut()?; 
            if let Some((delim_start, delim_end))= self.delimiter.find_next(remainder){
                let until_delimeter = &remainder[..delim_start];
                *remainder= &remainder[delim_end..];
                Some(until_delimeter)
            
            }else {
                self.remainder.take()
            } 
    }    
}   


//Delimiter Trait for finding first and last delimeter
impl Delimiter for &str {
    fn find_next(&self, s: &str) -> Option <(usize, usize)>{
        s.find(self).map(|start| (start, start + self.len()))
    }
}




#[cfg(test)]
mod tests {
    use super::*;

#[test]
fn it_works0() {
    let haystack = "a b c d ";
    let letters: Vec<_> = StrSplit::new(haystack,  " ").collect();
    assert_eq!(letters, vec!["a", "b", "c", "d", ""]);
}

#[test]
fn it_works() {
    let haystack = "a b c d e";
    let letters: Vec<_> = StrSplit::new(haystack,  " ").collect();
    assert_eq!(letters, vec!["a", "b", "c", "d", "e"]);
}
}