
// -------------------- Trait ------------------

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

fn main (){}



