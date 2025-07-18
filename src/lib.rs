//! This crate provides a custom iterator for splitting strings by a delimiter.
//!
//! It includes the `StrSplit` struct, which implements the `Iterator` trait.
#![warn(missing_debug_implementations, rust_2018_idioms,missing_docs)]


/// An iterator that splits a string by a delimiter.
#[derive(Debug)]
pub struct StrSplit<'a>{
    remainder: &'a str,
    delimiter: &'a str,
}
// implement struct
impl<'a> StrSplit<'a>{
        /// Creates a new `StrSplit` iterator over `haystack`, splitting by `delimiter`.
    pub fn new(haystack: &'a str, delimiter: &'a str) -> Self{
        Self { 
            remainder: haystack,
            delimiter,
        }
    }
}

impl<'a> Iterator for StrSplit<'a>{
    type Item= &'a str;

    /// Returns the next substring until the delimiter, or `None` if done.
    fn next(&mut self)-> Option<Self::Item>{
        println!("Calling .next() on StrSplit");
        if let Some(next_delim)= self.remainder.find(self.delimiter){
            let until_delimeter = &self.remainder[..next_delim];
            self.remainder= &self.remainder[(next_delim + self.delimiter.len())..];
            Some(until_delimeter)
            
        }else if self.remainder.is_empty(){
            None
        }else {
            let rest = self.remainder;
            self.remainder= "";
            Some(rest)
        }
    }
}

#[test]
fn it_works() {
    let haystack = "a b c d e";
    let letters: Vec<_> = StrSplit::new(haystack,  " ").collect();
    assert_eq!(letters, vec!["a", "b", "c", "d", "e"]);
}