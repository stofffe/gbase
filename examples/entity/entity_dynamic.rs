use std::any::Any;

// use std::any::Any;
//
// trait Entity {}
//
// fn extract<T: Entity>(e: Box<dyn Entity>) -> T {
//     e.
//
// }
//
// struct Game {
//     entities: Vec<Box<dyn Entity>>,
// }
trait Entity: Any {}

struct A;
struct B;
struct C;

impl Entity for A {}
impl Entity for B {}
impl Entity for C {}

fn main() {
    let entites: Vec<Box<dyn Any>> = vec![Box::new(B), Box::new(C), Box::new(B)];

    let player = Box::new(A);

    for mut e in entites {
        if let Some(b) = e.downcast_mut::<B>() {
            println!("Found a B entity!");
        }
    }

    // let a = Box::new(B) as Box<dyn Any>;
    // let _: &C = match a.downcast_ref::<C>() {
    //     Some(b) => b,
    //     None => panic!("&a isn't a B!"),
    // };
}
