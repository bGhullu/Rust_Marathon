
// -------------------- Method & Associate Functions ------------------


struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle{
    fn area(self) -> u32 {
        self.width * self.height
    }
}

struct TrafficLight{
    color: String
}

enum TrafficLightColor{
    Red,
    Yellow,
    Green,
}
impl TrafficLightColor{
    fn color(&self) -> &str {
        match self {
            Self::Yellow => "Yellow",
            Self::Red => "Red",
            Self::Green => "Green",
        }
    }
}



impl TrafficLight{

    pub fn new() -> Self {
        Self { 
            color: String::from("Red")
        }
    }

    pub fn get_state(&self) -> &str{
        &self.color
    }
    
    pub fn show_state(self: &Self) { // &self is similar. to self: &self
        println!("The current state is {}", self.color)
    } 

    pub fn change_state(&mut self){
        self.color = "green".to_string()
    }
}
fn main (){
    let rect1 = Rectangle{width: 10, height: 10};
    assert_eq!(rect1.area(),100);


    let light: TrafficLight = TrafficLight::new();
    assert_eq!(light.get_state(), "Red");

    let c  : TrafficLightColor = TrafficLightColor::Yellow;
    assert_eq!(c.color(), "Yellow");

    println!("Success!")

}