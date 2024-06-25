fn main() {
    let mut res = Builder::new().a(12).deref();
    eprintln!("res {:?}", res.build());
}

#[derive(Debug, Clone)]
struct Builder<'a> {
    a: i32,
    b: &'a str,
}

impl<'a> Builder<'a> {
    fn new() -> Self {
        Self { a: 0, b: "" }
    }

    fn a(&mut self, value: i32) -> &mut Self {
        self.a = value;
        self
    }

    fn b(&mut self, value: &'a str) -> &mut Self {
        self.b = value;
        self
    }

    fn build(&mut self) -> Res {
        Res {
            a: self.a,
            b: self.b.to_string(),
        }
    }
    fn deref(&mut self) -> Self {
        self.clone()
    }
}

#[derive(Debug)]
struct Res {
    a: i32,
    b: String,
}
