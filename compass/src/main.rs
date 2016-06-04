mod compass;

fn main() {
    {
        use compass::{Circle, Line, P2, V2, Intersect};

        let circle = Circle{center: P2::new(0.0, 0.0), radius: 1.0};
        let line = Line{start: P2::new(0.0, 0.5), direction: V2::new(1.0, 0.0)};

        println!("{:?}", (&line, &circle).intersect());
        println!("{:?} {:?}", circle, line);
    }

}
