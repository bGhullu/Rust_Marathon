
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

use std::{default, hash::RandomState, ops};

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

fn random_animal(random_number: f64) -> Box<dyn Animal>{
    if random_number < 0.5{
        Box::new(Sheep{})
    }else{
        Box::new(Cow{})
    }
}

fn sum<T: std::ops::Add<Output = T>>(x:T, y:T) -> T {
    x+y
}

struct Pair<T>{
    x: T,
    y: T,
}

impl <T> Pair<T>{
    fn new(x:T, y:T) -> Self{
        Self { 
           x,
           y,
         }
    }
}

impl<T: std::fmt::Debug + PartialOrd + PartialEq > Pair<T>{
    fn cm_display(&self){
        if self.x >= self.y {
            println!("The largest member is x = {:?}", self.x);
        }else {
            println!("The largest member is y = {:?}", self.y);
        }
    }
}
#[derive(Debug, PartialEq, PartialOrd)]
struct Unit(i32);

// ................... Trait Object ...................................

trait Bird {
    fn quack(&self) -> String;
    fn sound(&self);
}

struct Duck;
impl Duck {
    fn swim(&self) {
        println!("Look, the duck is swimming!");
    }
}

struct Swan;
impl Swan{
    fn fly(&self){
        println!("Look, The duck .. oh sorry, the swam is flying!");
    }
}

impl Bird for Duck{
    fn quack(&self)-> String{
        "duck duck".to_string()
    }
    fn sound(&self){
        println!("{}","duck duck")
    }
}

impl Bird for Swan{
    fn quack(&self)-> String{
        "swan swan".to_string()
    }

    fn sound(&self) {
        println!("{}", "swan swan")
    }
}

// ......................  &dyn and Box<dyn> .........................

trait Draw{
    fn draw (&self) -> String;
}

impl Draw for u8{
    fn draw(&self) -> String{
        format!("u8:{}",self)
    }
}

impl Draw for f64{
    fn draw(&self)-> String{
        format!("f64:{}",self)
    }
}

trait Fooo{
    fn method(&self) -> String;
}

impl Fooo for u8{
    fn method(&self) -> String {format!("u8: {}", self)}
}

impl Fooo for String{
    fn method(&self) -> String {
        format!("string: {}", self)
    }
}

fn static_dispatc < T: Fooo> (a: T) -> String{
    a.method()
    
}

fn dynamic_dispatch(x: Box<dyn Fooo>)-> String {
    x.method()
    
}


// ................. Object Safe .............................

trait MyTrait{
    fn f(&self)-> Box <dyn MyTrait>;
}

impl MyTrait for u32{
    fn f(&self) -> Box<dyn MyTrait> {Box::new(42)}
}

impl MyTrait for String{
    fn f(&self) -> Box<dyn MyTrait>{Box::new(self.clone())}
}

fn my_function(x: Box<dyn MyTrait>) ->Box<dyn MyTrait> {
    x.f()
}

fn main (){

    let x1: u8 = 5u8;
    let y: String = "Hello".to_string();
    static_dispatc(x1);
    dynamic_dispatch(Box::new(y));
    
    my_function(Box::new(12_u32));
    my_function(Box::new(String::from("abe")));

    let x = 1.1f64;
    let y =8u8;

    // Draw x
    draw_with_box(Box::new(x));

    // Draw y
    draw_with_ref(&y);

    
    let birds:[&dyn Bird;2] = [&Duck,&Swan];
    for bird in birds {
        bird.sound();
    }
    let duck = Duck;
    duck.swim();

    let swan = Swan;
    swan.fly();

    let bird = hatch_a_bird(2);
    assert_eq!(bird.quack(), "duck duck");

    let bird = hatch_a_bird(1);
    assert_eq!(bird.quack(),"swan swan");


 
    let pair = Pair::new(Unit(1), Unit(2));

    pair.cm_display();


    assert_eq!(sum(1,2), 3);
    println!("{}", sum(5.0,1.0));

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

fn draw_with_box(x: Box<dyn Draw>) {
    x.draw();
}

fn draw_with_ref(x: &dyn Draw){
    x.draw();
}

fn hatch_a_bird(species: u8) -> Box<dyn Bird> {
    match species{
        1=> Box::new(Swan),
        2=> Box::new(Duck),
        _=> panic! (),
    }
}

fn summary<T: Summary>(a: &T){
   let output: String =  a.summarize();
   println!("{}", output);
}


