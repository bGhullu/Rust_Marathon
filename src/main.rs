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