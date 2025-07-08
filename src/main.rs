
#[allow(unused_variables)]
fn main (){
    let x: i32 = 5;
    print!("x:{}",x);
    println!("\n{value} shifted is {value:#x}", value = 42);
    println!("x:{:?}",x);
    println!("x:{:#?}",x);
    println!("x:{:.3}",x);

    let greeting = String::from("Hello world!");
    println!("{}", greeting);

    let x = 5;

    assert_eq!(x, 5);

    let y: i32;

    let (x, y);

    (x,..) = (3,4);
    [..,y] = [1,2];

    assert_eq!([x,y], [3,2]);
    println!("Success!");

    let v: u16 = 38_u8 as u16;
    println!("Success!");

    for a in 'a' .. 'd'{
        println!("{}", a as u8);
    }

    assert!(1u32 + 2 ==3);
}



// use std::fs::read_to_string;

// fn main(){

//     let result= read_to_string( "a.txt");
//     match result{
//         Ok(data)=> println!("{}",data),
//         Err(err) => println!("{}",err),
//     }

//     let read_data = read_from_file (String::from("a.txt"));
//     println!("{}",read_data);

// }

// fn read_from_file(file_path: String) -> String {
//     let result = read_to_string(file_path);
//     match result {
//         Ok(data) => data,
//         Err(err)=> String::from("-1"),
//     }
// }




// fn main (){
//     let index = find_first_a(String::from("RUST"));


//     match index {
//         Some(value) => println!("index is {}", value),
//         None => println!("a not found!"),
//     }

// }

// fn find_first_a(s: String)  -> Option<i32> {
//     for (index,char) in s.chars().enumerate(){
//         if char == 'a'{
//             return Some(index as i32);
//     }
// }

//     return None;
// }




// struct Rect {
//     width: u32,
//     height: u32,
// }
// enum Shape{
//     Rectangle(f64, f64),
//     Circle(f64),
// }

// impl Rect {
//     fn area(&self) -> u32{
//       self.width * self.height
//     }
// }

//fn main(){
//     // let rect1 = Rect{
//     //     width: 10,
//     //     height: 10,
//     // };

//     // println!("The area of rect1 is {}", rect1.area());

//     let rectangle = Shape::Rectangle(2.0,2.0);
//     let Shape::Rectangle(a, b) = rectangle else {
//         panic!("expected a rectangle");
//     };
//     println!("The are of rectangle {} and {} :{}",a,b,calculate_area(rectangle));
//     let circle = Shape::Circle(2.0);
//     println!("The area of circle: {}",calculate_area(circle));
    

// }

// fn calculate_area(shape: Shape) -> f64  {
//    match shape {
//     Shape::Rectangle(a,b) => a*b,
//     Shape::Circle(a) => 3.14 *a *a,
//    }
 
//}