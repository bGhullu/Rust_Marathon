
// -------------------- Closures ------------------

/*
    Fn: The closure uses the captured value by reference(&T)
    FnMut: The closure uses the captured value by mutable reference(&mut T)
    FnOnce: The closure uses the captured value by value (T)

*/

struct MyStruct<F>
where
    F: Fn(i32) -> i32,
{
    callback: F,
}

impl<F> MyStruct<F>
where
    F: Fn(i32) -> i32,
{
    fn call(&self, input: i32) -> i32 {
        (self.callback)(input)
    }
}




fn fn_once<F>(func: F)
where 
    F:  Fn(usize) -> bool,
{
    println!("{}",func(3));
    println!("{}",func(4));
}    

fn main(){


    let add_five = |x| x + 5;

    let obj = MyStruct { callback: add_five };

    let result = obj.call(10); // 10 + 5 = 15
    println!("Result: {}", result);

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


    let x = String::from("hello");

    let closure = move || println!("{}", x); // ??
    funct(closure);   

    let fn_plain0= create_fn0();
    let result0 =fn_plain0(1);
    println!("{}", result0);
    let fn_plain = create_fn();  // fn_plain = |x| x+ num;

    let result = fn_plain(1); //6
    println!("{}",result);

}

fn create_fn0()-> Box< dyn Fn(i32)->i32>{
    let num: i32 =4;
   Box::new( move |x| x+ num)
}
fn create_fn()-> impl Fn(i32) -> i32 {
    let num: i32 = 5;

    move |x| x+num
}
fn funct<F: Fn()>(f: F) {
    f();
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

