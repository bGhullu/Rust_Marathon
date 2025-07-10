
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


fn main (){

    let s: Student= Student {  };
    assert_eq!(s.say_hi(), "Hi!");
    assert_eq!(s.say_something(),"I'm a good student");
    let t: Teacher = Teacher{};
    assert_eq!(t.say_hi(), "Hi, I'm your new teacher");
    assert_eq!(t.say_something(),"I'm not a bad teacher");

    println!("Success!");
}


