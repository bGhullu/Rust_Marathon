use std::num::ParseIntError;
use std::fmt::{self, write};

struct List(Vec<i32>);

impl fmt::Display for List{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let vec: &Vec<i32> = &self.0;
        write!(f, "[");

        for (count,v) in vec.iter().enumerate(){
            if count!=0{write!(f,",")?;}
            write!(f,"{}:{}",count,v)?;
        }
        write!(f,"]")
    }
}

type Res<i32> = Result<i32, ParseIntError>;

fn add_two(n_str: &str) -> Res<i32> {
    n_str.parse::<i32>().map(|n|n+2)
}
fn addition(n_str: &str) ->Result<i32,ParseIntError>{
    n_str.parse::<i32>().and_then(|n| Ok(n+2))
}

fn multiply(n1_str: &str, n2_str: &str) -> Result<i32,ParseIntError>{
    n1_str.parse::<i32>().and_then(|n1| n2_str.parse::<i32>().map(|n2|n1*n2))
}


fn main () {

    let v: List = List(vec![1,2,3]);
    assert_eq!(format!("{}",v),"[0:1,1:2,2:3]");
    println!("{}",v);

    assert_eq!(add_two("4").unwrap(),6);
    assert_eq!(addition("5").unwrap(),7);
    assert_eq!(multiply("2" ,"3").unwrap(), 6);
    println!("Success!");
}