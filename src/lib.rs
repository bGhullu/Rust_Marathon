//! This crate provides a custom iterator for splitting strings by a delimiter.
//!
//! It includes the `StrSplit` struct, which implements the `Iterator` trait.
#![warn(missing_debug_implementations, rust_2018_idioms,missing_docs)]




/// An iterator that splits a string by a delimiter.
#[derive(Debug)]
pub struct StrSplit<'a>{
    remainder: Option<&'a str>,
    delimiter: &'a str,
}
// implement struct
impl<'a> StrSplit<'a>{
        /// Creates a new `StrSplit` iterator over `haystack`, splitting by `delimiter`.
    pub fn new(haystack: &'a str, delimiter: &'a str) -> Self{
        Self { 
            remainder: Some(haystack),
            delimiter,
        }
    }
}

impl<'a> Iterator for StrSplit<'a>{
    type Item= &'a str;

    /// Returns the next substring until the delimiter, or `None` if done.
    fn next(&mut self)-> Option<Self::Item>{

        let  remainder= self.remainder.as_mut()?; 
            if let Some(next_delim)= remainder.find(self.delimiter){
                let until_delimeter = &remainder[..next_delim];
                *remainder= &remainder[(next_delim + self.delimiter.len())..];
                Some(until_delimeter)
            
            }else {
                self.remainder.take()
            } 
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