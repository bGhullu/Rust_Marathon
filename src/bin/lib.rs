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
/// A trait for types that can act as delimiters in string splitting.
pub trait Delimiter {
    /// Finds the next occurrence of the delimiter in the given string.
    ///
    /// Returns `Some((start, end))` if found, or `None` if no match is found.
    fn find_next(&self, s: &str) -> Option<(usize, usize)>;
}


impl Delimiter for &str {
    fn find_next(&self, s: &str) -> Option <(usize, usize)>{
        s.find(self).map(|start| (start, start + self.len()))
    }
}

impl Delimiter for char{
    fn find_next(&self,  s: &str) -> Option<(usize,usize)>{
        s.char_indices()
            .find(|(_,c)| c == self)
            .map(|(start, _)| (start, start+self.len_utf8()))
    }
}
impl<'haystack,D> Iterator for StrSplit<'haystack,D>
where D: Delimiter,
{
    type Item= &'haystack str;

    /// Returns the next substring until the delimiter, or `None` if done.
    fn next(&mut self)-> Option<Self::Item>{
   

        let  remainder= self.remainder.as_mut()?; 
    

            if let Some((delim_start, delim_end))= self.delimiter.find_next(remainder){
                println!("remainder: {:?}", remainder);
                println!("delim_start: {}, delim_end: {}", delim_start, delim_end);
                println!("slice: {:?}", &remainder[..delim_start]);
                println!("new remainder: {:?}", &remainder[delim_end..]);
                let until_delimeter = &remainder[..delim_start];
                *remainder= &remainder[delim_end..];
                Some(until_delimeter)
            
            }else {
                self.remainder.take()
            } 
    }    
}   

/// Returns the substring of `s` up to (but not including) the first occurrence of the character `c`.
/// If `c` is not found, returns the entire string `s`.
pub fn until_char(s: &str, c: char) -> &'_ str{

    StrSplit::new(s,c)
    .next()
    .expect("StrSplit always gives at least one result")
}


#[cfg(test)]
mod tests {
    use super::*;

#[test]
fn until_char_test(){
    assert_eq!(until_char("hello world", 'o'), "hell");
}

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