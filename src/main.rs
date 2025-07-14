
// -------------------- Closures ------------------

/*
    Fn: The closure uses the captured value by reference(&T)
    FnMut: The closure uses the captured value by mutable reference(&mut T)
    FnOnce: The closure uses the captured value by value (T)

*/


fn fn_once<F>(func: F)
where 
    F:  Fn(usize) -> bool,
{
    println!("{}",func(3));
    println!("{}",func(4));
}    

fn main(){

    let x: Vec<i32> = vec![1,2,3];
    fn_once(|z|{
        z == x.len()
    });
    let mut count: i32= 0;

    let mut inc = move || {
        count +=1;
        println!("Count:{}", count);
    };
    inc();

    let _reborrow: &i32= & count;
    inc();
    
    let _count_reborrowed = &mut count;
    assert_eq!(count,0);

    let moveable: Box<i32>= Box::new(3);

    let consume = || {
        println!("moveable:{:?}", moveable);
        take(&moveable);
    };
    consume();
    consume();

    let mut s:  String= String::new();
    let update_string =|str| s.push_str(str);

    exec(update_string);
    let update_string1 = |str| s.push_str(str);
    exec1(update_string1);    

    println!("{}", s);

}

fn exec<'a, F>(mut f: F)
where 
    F: FnMut(&'a str){
        f("hello ");
    }


fn exec1<'a, F: FnMut(&'a str)>(mut f: F){
    f("world!");
}

fn take<T>(_v: &T){}

