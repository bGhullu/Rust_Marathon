
// -------------------- Trait ------------------

/* 
trait Animal {
    fn sound(&self) -> String;
}

struct Sheep;
struct Cow;

impl Animal for Sheep {
    fn sound (&self) -> String {
        String::from("Maah")
    }
}

impl Animal for Cow{
    fn sound(&self) -> String{
        String::from("Mooh")
    }
}


trait Summary{}


pub fn notify (item: &impl Summary){
    println!("BreakingNews!{}", item.summarize());
}

use this to avoid pub fn notify1(item:  &impl Summary,  items2: &impl Summary)

pub fn notify1<T: Summary> (item: &T) {
    println!("Breaking News! {}", item.summarize());
}

fn some_function<T: Display + Clone, U: Clone + Debug>(t: &T, u: &U) ->i32{} can be written as 
fn some_function<T,U>(t: &T, u: &U)->i32 
where
    T: Display + Clone,
    U: Clone + Debug,
{}


*/

trait Hello{
    fn say_hi(&self) -> String{
        String::from("Hi!")
    }
    fn say_something(&self) -> String;
}

struct Student{}
impl Hello for Student{
    fn say_something(&self) -> String {
        String::from("I'm a good student")
    }
}

struct Teacher{}
impl Hello for Teacher{

    fn say_hi(&self) -> String {
        String::from("Hi, I'm your new teacher")
    }
    fn say_something(&self) -> String {
        String::from("I'm not a bad teacher")
    }
}

// ...... Derive .......

#[derive(PartialEq, PartialOrd)]
struct Centimeters(f64);

#[derive(Debug)]
struct Inches(i32);

impl Inches {
    fn to_centimeters(&self) -> Centimeters{
        let &Inches(inches)= self;
        Centimeters(inches as f64 * 2.54)
    }
}
#[derive(Debug, PartialEq, PartialOrd )]
struct Seconds(i32);

//......... Operator .......

use std::{hash::RandomState, ops};

fn multiply<T: std::ops::Mul <Output = T>>(x: T, y: T) -> T{
    x * y
}

struct Foo;
struct Bar;
#[derive(Debug,PartialEq)]
struct FooBar;
#[derive(Debug,PartialEq)]
struct BarFoo;

impl ops::Add<Bar> for Foo{
    type Output = FooBar;
    fn add(self, _rhs:Bar) -> FooBar{
        FooBar
    }
}

impl ops::Sub<Bar> for Foo{
    type Output = BarFoo;
    fn sub(self, _rhs:Bar) -> BarFoo{
        BarFoo
    }
}

//....... as Function Parameters ..................

trait Summary {
    fn summarize(&self) -> String;
}

#[derive(Debug)]
struct Post{
    title: String,
    author: String,
    content: String,
}
impl Summary for Post{
    fn summarize(&self) -> String {
        format!("The author of post {} is {}", self.title, self.author)
    }
}
#[derive(Debug)]
struct Weibo {
    username: String,
    content: String,
}

impl Summary for Weibo{
    fn summarize(&self) -> String {
        format!("{} published a weibo {}", self.username,self.content)
    }
}

//................. Returning Types that Immplements Traits ..........................

struct Sheep{}
struct Cow{}

trait Animal {
    fn noise(&self) -> String;
}

impl Animal for Sheep{
    fn noise(&self) -> String{
        String::from("baahhh!")
    }
}

impl Animal for Cow {
    fn noise(&self) -> String{
        String::from("Mooooooo!")
    }
}

fn random_animal(random_number: f64) -> &dyn Animal{
    if random_number < 0.5{
        Sheep{}
    }else{
        Cow{}
    }
}
fn main (){

    let s: Student= Student {  };
    assert_eq!(s.say_hi(), "Hi!");
    assert_eq!(s.say_something(),"I'm a good student");
    let t: Teacher = Teacher{};
    assert_eq!(t.say_hi(), "Hi, I'm your new teacher");
    assert_eq!(t.say_something(),"I'm not a bad teacher");

    println!("Success!");

    let _one_second = Seconds(1);
    println!("One second lookks like: {:?}", _one_second);
    let _this_is_true = _one_second ==_one_second;
    let _this_is_false = _one_second > _one_second;
    println!("{}",_this_is_true);
    println!("{}",_this_is_false);

    let foot = Inches(12);
    println!("One foot equal {:?}", foot);

    let meter = Centimeters(100.0);
    let cmp =
        if foot.to_centimeters() < meter{
            "smaller"
        }else{
            "bigger"
        };
    println!("One foot is {} than one meter.", cmp);
    
    assert_eq!(6, multiply(2u8, 3u8));
    assert_eq!(5.0, multiply(1.0, 5.0));

    assert_eq!(Foo + Bar, FooBar);
    assert_eq!(Foo - Bar, BarFoo);

    let post: Post = Post { 
        title: "Popular Rust".to_string(), 
        author: "Sunface".to_string(),
        content: "Rust is awesoem!".to_string()
    };
    let weibo: Weibo = Weibo { 
        username: "surface".to_string() , 
        content:  "Weibo seems to be worse than X".to_string(),
    };
    
    summary(&post);
    summary(&weibo);

    println!("{:?}", post);
    println!("{:?}",weibo);

    let random_number = 0.234;
    let animal= random_animal(random_number);
    println!("You've randomly chosen an animal, and it says {}",animal.noise());

   
}

fn summary<T: Summary>(a: &T){
   let output: String =  a.summarize();
   println!("{}", output);
}


