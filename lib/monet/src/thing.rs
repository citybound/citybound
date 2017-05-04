
use compact::CVec;

use Vertex;

#[derive(Compact, Debug)]
pub struct Thing {
    pub vertices: CVec<Vertex>,
    pub indices: CVec<u16>,
}

impl Thing {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Thing {
        Thing {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }
}

impl Clone for Thing {
    fn clone(&self) -> Thing {
        Thing {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into(),
        }
    }
}

impl ::std::ops::Add for Thing {
    type Output = Thing;

    fn add(self, rhs: Thing) -> Thing {
        let self_n_vertices = self.vertices.len();
        Thing::new(self.vertices
                       .iter()
                       .chain(rhs.vertices.iter())
                       .cloned()
                       .collect(),
                   self.indices
                       .iter()
                       .cloned()
                       .chain(rhs.indices.iter().map(|i| *i + self_n_vertices as u16))
                       .collect())
    }
}

impl ::std::ops::AddAssign for Thing {
    fn add_assign(&mut self, rhs: Thing) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl ::std::iter::Sum for Thing {
    fn sum<I: Iterator<Item = Thing>>(iter: I) -> Thing {
        let mut summed_thing = Thing {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for thing in iter {
            summed_thing += thing;
        }
        summed_thing
    }
}

impl<'a> ::std::ops::AddAssign<&'a Thing> for Thing {
    fn add_assign(&mut self, rhs: &'a Thing) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl<'a> ::std::iter::Sum<&'a Thing> for Thing {
    fn sum<I: Iterator<Item = &'a Thing>>(iter: I) -> Thing {
        let mut summed_thing = Thing {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for thing in iter {
            summed_thing += thing;
        }
        summed_thing
    }
}
