
// -------------------- Iterator ------------------


// pub trait Iterator{
//     type Item;
//     fn next(&mut self)-> Options<Self::Item>;

// }

//............................. Custom Iterator ...................................


struct Counter {
    count: u32,
}

impl Counter{
    fn new() -> Counter{
        Counter { count: 0}
    }
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self)-> Option<Self::Item>{
        if self.count <5{
            self.count += 1;
            Some(self.count)
        }else {
            None
        }
    }
}

struct Fibonacci{
    curr: u32,
    next: u32,
}

impl Iterator for Fibonacci{
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item>{
        let forward= self.curr + self.next;
        self.curr = self.next;
        self.next = forward;
        Some(self.curr)

    }
}

fn fibonacci() -> Fibonacci{
    Fibonacci { curr: 0, next: 1 }
}
// ............................... Custom Iterator .......................................

fn main (){

    let mut counter =Counter::new();
    assert_eq!(counter.next(), Some(1));
    assert_eq!(counter.next(), Some(2));
    assert_eq!(counter.next(), Some(3));
    assert_eq!(counter.next(), Some(4));
    assert_eq!(counter.next(), Some(5));
    assert_eq!(counter.next(), None);
    
    let mut fib = fibonacci();
    assert_eq!(fib.next(), Some(1));
    assert_eq!(fib.next(), Some(1));
    assert_eq!(fib.next(), Some(2));
    assert_eq!(fib.next(), Some(3));
    assert_eq!(fib.next(), Some(5));


    let v= vec![1,2,3];
    for i in v {
        println!("{}", i); // is similar to x in v.into_iter(){
    }

    let arr = [0;10];
    for i in 0..arr.len(){
        println!("{}", arr[i]);
    }

    let mut v = Vec::new();
    for n in 0..100{
        v.push(n);
    }
    assert_eq!(v.len(), 100);

    let mut v1= vec![1,2,3].into_iter();
    assert_eq!(v1.next(),Some(1));
    assert_eq!(v1.next(),Some(2));
    println!("v1:{:?}",v1);

    let v2 = vec![1,2,3];
    for i in v2.iter(){
        println!("{}",i);
    }
    print!("{:?}",v2);

    let mut names : Vec<&str>= vec!["Bob","Frank","Ferris"];
    for name in names.iter_mut(){
        *name =match name{
            &mut "Ferris" =>"Ferris name has been changed to Fern.",
            _=> "Hello",
        }
    }
    println!("Names {:?}", names);

    let mut values = vec![1,2,3];
    let mut values_iter= values.iter_mut();
    if let Some(v) =values_iter.next(){
        *v = 0;
    }
    assert_eq!(values,vec![0,2,3]);
  


}

