
// -------------------- Generics ------------------


fn Sum<T: std::ops::Add<Output =T>>(a:T,b:T) ->T{
    a + b
}
struct Point<T> {
    x:  T,
    y: T,
}

struct Dif<T,U>{
    x:T ,
    y: U,
}

struct Val<T>{
    val: T,
}

impl<T> Val<T>{
    fn value(&self)-> &T{
        &self.val
    }
}

struct Poin<T,U>{
    x: T,
    y: U,
}
impl<T,U> Poin<T,U>{
    fn mixup<V,W>(self, other: Poin<V,W>) -> Poin<T,W>{
        Poin {
             x: self.x,
             y: other.y,
        }
    }
}

struct Array<T, const N: usize>{
    data: [T;N]
}
fn main (){
    let integer:Point<i32>= Point{x:5, y:10};
    let float : Point<f64>= Point{x:2.0 , y:1.0};
    let diff: Dif<i32,String> = Dif{x:10, y: "hello".to_string()};

    let x =Val{val:3.0};
    let y=Val{val:"hello".to_string()};
    println!("{},{} ", x.value(), y.value());

    let p1: Poin<i32, i32> = Poin{x:5, y:10};
    let p2: Poin<&str, char> = Poin{x:"Hello", y: 'c'};
    let p3 = p1.mixup(p2);

    assert_eq!(p3.x, 5);
    assert_eq!(p3.y,'c');

    let arrays: [Array<i32,3>;3]=[
        Array{
            data: [1,2,3],
        },
        Array{
            data:[1,2,3],
        },
        Array{
            data:[4,5,6]
        },
    ];

    let floats: [Array<f64,3>;2]=[
        Array{
            data:[1.0,2.0,3.0]
        },
        Array{
            data: [4.0,5.0,6.0]
        },
    ];
}

