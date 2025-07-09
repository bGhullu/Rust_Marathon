use core::panic::PanicMessage;


#[allow(unused_variables)]

struct Person{
    name: String,
    age: u8,
}

enum Message {
        Quit,
        Move {x:i32, y:i32},
        Write(String),
        ChangeColor(i32,i32,i32),
}

enum MyEnum{
        Foo,
        Bar
}

enum Foo{
    Bar(u8)
}

enum Foo1{
    Bar, 
    Baz,
    Qux(u32)
}
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

    assert!(1u32 + 2u32 ==3);

    let t: (String, String)=(String::from("hello"), String::from("world"));

    let (ref s1, ref s2) = t;
    println!("{:?}, {:?},{:?}",s1,s2,t);

    let (s3,s4) = t.clone();
    println!("{:?},{:?},{:?}",s3,s4,t);

    let byte_escape = "I'm writing Ru\x73\x74!";
    println!("What are you doing \x3F ( \\x3F means ?) {}", byte_escape);

    let unicode_codepoint = "\u{211D}";
    let character_name = "\"DOUBLE-STRUCK CAPITAL R\"";

    // ------------- Arrays ---------------------------

    let names: [String; 2] = [String::from("hello"), String::from("world!")];
    let numbers:[i32;3]= [1,2,3];




    // ------------- Flow Control ---------------------

    let   (mut n, mut y): (i32, i32) = (0,0);
    for i in 0..=100{
        if n ==66 {
            break;
        }
        n +=1;
    }

    for j in 0..100{
        if y!= 22{
            y+=1;
            continue;
        }
        break;
    }
    println!("The value of y is: {}", y);
    

    assert_eq!(n, 66);
    println!("The value of n is: {}",n);

    for name in &names {
        println!("{}", name)
    }

    println!("{:?}", names);

     for n in numbers{
        println!("{}", n);
    }

    let ae:[i32;4]= [1,2,3,4,];
    for (i,v) in ae.iter().enumerate() {
        println!("The {}th element is {}", i+1, v);
    }

    println!("{:?}", numbers);

    for n in 1..100{
        if n == 100{
            panic!("Never let this Run!")
        }
    }

    println!("Success!");

    let mut count: u32 = 0u32;
    println!("Let's count until infinity!");
    loop{
        count +=1;
        if count ==3 {
            println!("Three");
            continue;
        }
        println!("The count is:{}", count);

        if count == 5 {
            println!("Ok! that's enough.");
            break;
        }
    }
    
    let mut counter: i32 = 0;

    let result:i32 = loop{
        counter+=1;
        if counter == 10{
            break counter *2;
        }  
    };

    println!("The value of result is: {}", result);
    
   let mut inner_outer: i32 = 0;

   'outer: loop{
        'inner1: loop{
            if inner_outer>= 20{
                // This would break only the inner1 loop
                break 'inner1; // `break` is also works.
            }
            inner_outer+=2;
        }
        inner_outer+=5;
        'inner2: loop{
            if inner_outer >=30 {
                // This breaks the outer loop
                break 'outer;
            }
            // This will continue the outer loop
            continue 'outer;
        }
   }

   println!("The value of inner_outer loop variable is {}:", inner_outer);

//------------------ Pattern Match ---------------------------

    enum Coin {
        Penny,
        Quid,
        Fiver,
        Tenner,
    }

    fn value_in_pound(coin: Coin) -> f64{
        match coin{
            Coin::Penny => 0.01,
            Coin::Quid => 1.0,
            Coin::Fiver => 5.0,
            Coin::Tenner =>10.0,
        }
    }

    // -------------------- if let ---------------------------------

    let config_max = Some(3u8);
    match config_max{
        Some(max)=> println!("The max is configured to be {}:",max),
        _ => (),
    }

    if let Some(max) = config_max{
        println!("The maximum is configured to be {}:", max);
    }

    enum Direction {
        East,
        West,
        North,
        South,
    }

    let dire: Direction = Direction::South;
    match dire{
        Direction::East => println!("East"),
        Direction::South | Direction::North => {
            println!("SOuth or North");
        },
        _ => println!("West"),
    }

    let boolean = true;

    let binary = match boolean {
        true => 1,
        false => 0,
    };

    println!("The binary value is {}:", binary);

 

    let msgs:[Message;3]= [
        Message::Quit,
        Message::Move{x:1, y:3},
        Message::ChangeColor(255,255, 0),
    ];

    for msg in msgs{
        show_message(msg)
    }

    let alphabets = ['a','E','Z','0','x','9','Y'];
    for ab in alphabets{
        assert!(matches!(ab,'A'..='Z' | 'a'..='z' | '0'..='9'));
    }

    let mut count1 =0;
    let v = vec![MyEnum::Foo, MyEnum::Bar, MyEnum::Foo];
    for e in v {
        if matches!(e,MyEnum::Foo){
            count1 +=1;
        }
    }

    println!("The count for MyEnum::Foo is: {}", count1);

    // ------- For some cases, when matching enums, match is too heavy.
    // ------- We can use if let instead
    let o: Option<i32> = Some(7);

    match o{
        Some(i) =>{
            println!("This is a really long string and {:?}", i);
        
        }
        _ => {}
    };

    if let Some(i) = o{
        println!("This is a really long string and `{:?}`", i);
    }

    let a = Foo::Bar(1);
    if let Foo::Bar(i) = a {
        println!("Foobar hold the value: {}", i );
    }
  
    let a = Foo1::Qux(10);

    if let Foo1::Bar = a {
        println!("match foo1::bar")
    }else if let Foo1::Baz = a {
        println!("match foo1::baz")
    }else{
        println!("match others")
    }
    
    match a {
        Foo1::Bar => println!("match foo::bar"),
        Foo1::Baz => println!("match foo::baz"),
        _ => println!("match others")
    }



}

  

fn show_message(msg: Message){
    match msg{
        Message::Move{x:a,y:b} => {
            assert_eq!(a, 1);
            assert_eq!(b, 3);
            println!("a: {} and b: {}", a, b);
        },
        Message::ChangeColor(r,g ,b )=>{
            assert_eq!(g,255);
            assert_eq!(b,0);
            println!("R,G,B:{},{},{}", r,g,b);
        },
        _ => println!("no data in these variants")
    }
    

 
}

fn build_person(name: String, age: u8) -> Person {
    Person { age, name, }
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